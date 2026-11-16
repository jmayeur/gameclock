//! The behavior of `gameclockd`. It is not in `main`, so tests do not need
//! hardware or signals (`openspec/specs/daemon-lifecycle`).

use core::fmt;
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Duration;

#[cfg(feature = "simulator")]
pub mod sim;

pub mod command;
pub mod config;
pub mod control;
pub mod http;

use command::{Command, OK, no_alert_name_reply, status_reply, unknown_alert_reply};
use config::{ConfigError, Origin};

use gameclock_core::alert::Triggers;
use gameclock_core::board::Board;
use gameclock_core::clock::Clock as ClockScene;
use gameclock_core::heartbeat::Heartbeat;
use gameclock_core::schedule::Schedule;
use gameclock_core::tables::Tables;
use gameclock_core::{Brightness, Compositor, Frame, Panel, Scene};
use jiff::tz::TimeZone;
use jiff::{Timestamp, Zoned};

/// Why the daemon stopped with an error.
#[derive(Debug)]
pub enum RunError<E> {
    /// The panel failed to show a frame.
    Present(E),
}

impl<E: fmt::Display> fmt::Display for RunError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Present(e) => write!(f, "the panel did not show the frame: {e}"),
        }
    }
}

impl<E: std::error::Error + 'static> std::error::Error for RunError<E> {}

/// The source of the current time. The daemon injects it into the scenes
/// (docs/adr/0013).
pub trait Clock {
    /// The current time in the daemon's time zone.
    fn now(&self) -> Zoned;
}

/// The system clock in a fixed time zone.
#[derive(Debug, Clone)]
pub struct SystemClock {
    zone: TimeZone,
}

impl SystemClock {
    /// A clock that gives the system time in `zone`.
    #[must_use]
    pub fn new(zone: TimeZone) -> Self {
        Self { zone }
    }
}

impl Clock for SystemClock {
    fn now(&self) -> Zoned {
        // A time zone clone changes only a reference count.
        Timestamp::now().to_zoned(self.zone.clone())
    }
}

/// The stack size of each helper thread. They only block and send events
/// (docs/adr/0016).
pub const THREAD_STACK: usize = 64 * 1024;

/// A command and the sender for its reply line (docs/adr/0016).
#[derive(Debug)]
pub struct Request {
    /// The parsed command.
    pub command: Command,
    /// The loop sends the reply line here.
    pub reply: mpsc::Sender<String>,
}

/// An event for the loop: from the signal thread or the socket thread.
#[derive(Debug)]
pub enum Event {
    /// A stop signal came.
    Shutdown,
    /// A command came.
    Command(Request),
}

/// Why a wait ended.
#[derive(Debug)]
pub enum Wake {
    /// The deadline came, or the wait ended early. Render again.
    Deadline,
    /// A stop signal came. Stop the daemon.
    Shutdown,
    /// A command came. Do it and reply.
    Command(Request),
}

/// Blocks until a deadline, a command, or a stop signal.
pub trait Wait {
    /// Block until `deadline`, a command, or a stop signal. `now` is the time
    /// when the wait starts. With no deadline, block until a command or a
    /// stop signal.
    fn wait(&mut self, deadline: Option<Timestamp>, now: Timestamp) -> Wake;
}

/// Waits on the event channel, with a timeout until the deadline
/// (docs/adr/0016).
///
/// The signal thread and the socket thread send the events. A closed channel
/// means that no thread can send a stop signal. The daemon then stops.
#[derive(Debug)]
pub struct ChannelWait {
    events: mpsc::Receiver<Event>,
}

impl ChannelWait {
    /// A wait on `events`.
    #[must_use]
    pub fn new(events: mpsc::Receiver<Event>) -> Self {
        Self { events }
    }
}

impl Wait for ChannelWait {
    fn wait(&mut self, deadline: Option<Timestamp>, now: Timestamp) -> Wake {
        let event = match deadline {
            None => self
                .events
                .recv()
                .map_err(|_| mpsc::RecvTimeoutError::Disconnected),
            Some(deadline) => match Duration::try_from(deadline.duration_since(now)) {
                Ok(d) if !d.is_zero() => self.events.recv_timeout(d),
                // The deadline is now or in the past.
                _ => return Wake::Deadline,
            },
        };
        match event {
            Ok(Event::Shutdown) | Err(mpsc::RecvTimeoutError::Disconnected) => Wake::Shutdown,
            Ok(Event::Command(request)) => Wake::Command(request),
            Err(mpsc::RecvTimeoutError::Timeout) => Wake::Deadline,
        }
    }
}

/// Turn the result of the system time zone lookup into the zone that the
/// daemon uses, and the line to write to stderr. If the lookup failed, the
/// zone is UTC (docs/adr/0013).
#[must_use]
pub fn local_zone(lookup: Result<TimeZone, jiff::Error>) -> (TimeZone, String) {
    match lookup {
        Ok(zone) => {
            let message = match zone.iana_name() {
                Some(name) => format!("gameclockd: the time zone is {name}."),
                None => "gameclockd: the time zone has no name. The daemon uses it.".to_owned(),
            };
            (zone, message)
        }
        Err(_) => (
            TimeZone::UTC,
            "gameclockd: the system time zone is not available. The daemon uses UTC.".to_owned(),
        ),
    }
}

/// Run the event loop until a stop signal (docs/adr/0006).
///
/// The loop renders `base` for the current time and presents each changed
/// frame on `panel` at the brightness of the top scene. Then it waits until
/// the next scene change, the next trigger time, or an event. It does not
/// wake at other times. A command runs at once, and its reply goes back on
/// its sender. `mode` is the mode of `base`, for `status`. The board shows
/// messages and alerts on top of the base scene (docs/adr/0017,
/// docs/adr/0018).
///
/// # Errors
/// [`RunError::Present`] if the panel fails to show a frame.
pub fn run<P: Panel>(
    panel: &mut P,
    base: Box<dyn Scene>,
    mut mode: Mode,
    tables: Tables,
    mut reload: impl FnMut() -> Result<Tables, ConfigError>,
    clock: &impl Clock,
    wait: &mut impl Wait,
) -> Result<(), RunError<P::Error>> {
    let (width, height) = panel.dimensions();
    let mut compositor = Compositor::new(base, width, height);
    let Tables {
        mut schedule,
        mut alerts,
        triggers,
    } = tables;
    let mut board = Board::new(width, schedule.clone());
    let mut triggers = Triggers::new(triggers, &clock.now());
    loop {
        let now = clock.now();
        board.advance(&now, &mut compositor);
        for alert in triggers.due(&now) {
            // A refused trigger changes nothing (docs/adr/0018).
            let _ = board.alert(alert, &now, &mut compositor);
        }
        present(panel, &mut compositor, &now)?;
        let deadline = match (compositor.deadline(&now), triggers.next()) {
            (Some(scene), Some(trigger)) => Some(scene.min(trigger)),
            (scene, trigger) => scene.or(trigger),
        };
        match wait.wait(deadline, now.timestamp()) {
            Wake::Deadline => {}
            Wake::Shutdown => return Ok(()),
            Wake::Command(Request { command, reply }) => {
                // Each change shows on the panel before the reply.
                let now = clock.now();
                let text = match command {
                    Command::Mode(next) => {
                        if next != mode {
                            mode = next;
                            compositor.set_base(next.base_scene(&schedule));
                            board.advance(&now, &mut compositor);
                            present(panel, &mut compositor, &now)?;
                        }
                        OK.to_owned()
                    }
                    Command::Status => {
                        let zone = now.time_zone().clone();
                        status_reply(
                            mode,
                            &zone,
                            compositor.last_brightness(),
                            board.messages(),
                            board.alert_name(),
                        )
                    }
                    Command::Message(message) => {
                        match board.message(message, &now, &mut compositor) {
                            Ok(()) => {
                                present(panel, &mut compositor, &now)?;
                                OK.to_owned()
                            }
                            Err(full) => format!("error: {full}"),
                        }
                    }
                    Command::Clear => {
                        board.clear(&now, &mut compositor);
                        present(panel, &mut compositor, &now)?;
                        OK.to_owned()
                    }
                    Command::Alert(None) => no_alert_name_reply(&alerts),
                    Command::Alert(Some(name)) => match alerts.find(&name) {
                        Some(alert) => match board.alert(alert, &now, &mut compositor) {
                            Ok(()) => {
                                present(panel, &mut compositor, &now)?;
                                OK.to_owned()
                            }
                            Err(refused) => format!("error: {refused}"),
                        },
                        None => unknown_alert_reply(&name, &alerts),
                    },
                    Command::Reload => match reload() {
                        Ok(new) => {
                            // The scene on the panel keeps its alert and its
                            // schedule. New scenes use the new tables.
                            schedule = new.schedule;
                            alerts = new.alerts;
                            triggers = Triggers::new(new.triggers, &now);
                            board.set_schedule(schedule.clone());
                            if mode == Mode::Clock {
                                compositor.set_base(mode.base_scene(&schedule));
                                board.advance(&now, &mut compositor);
                                present(panel, &mut compositor, &now)?;
                            }
                            OK.to_owned()
                        }
                        Err(e) => format!("error: {e}"),
                    },
                    Command::StopAlert => {
                        board.stop_alert(&now, &mut compositor);
                        present(panel, &mut compositor, &now)?;
                        OK.to_owned()
                    }
                };
                // The client can be gone. That is not an error of the loop.
                let _ = reply.send(text);
            }
        }
    }
}

/// Render, and present a changed frame.
fn present<P: Panel>(
    panel: &mut P,
    compositor: &mut Compositor,
    now: &Zoned,
) -> Result<(), RunError<P::Error>> {
    if let Some((frame, brightness)) = compositor.update(now) {
        panel
            .present(frame, brightness)
            .map_err(RunError::Present)?;
    }
    Ok(())
}

/// A command line that the binary does not accept.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UsageError {
    /// An argument that is not a `--led-*` driver flag, `--test-pattern`,
    /// `--control-socket=<path>`, or `--config=<path>`.
    Unknown(String),
    /// `--control-socket=` with no path.
    EmptySocketPath,
    /// `--config=` with no path.
    EmptyConfigPath,
    /// `ctl` with no command.
    Ctl,
}

impl fmt::Display for UsageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unknown(arg) => write!(
                f,
                "unknown argument '{arg}'. gameclockd accepts only --led-* driver flags, --test-pattern, --control-socket=<path>, and --config=<path>."
            ),
            Self::EmptySocketPath => write!(
                f,
                "--control-socket needs a path, for example --control-socket={DEFAULT_SOCKET}."
            ),
            Self::EmptyConfigPath => write!(
                f,
                "--config needs a path, for example --config={DEFAULT_CONFIG}."
            ),
            Self::Ctl => {
                f.write_str("usage: gameclockd ctl [--control-socket=<path>] <command> [<word>...]")
            }
        }
    }
}

impl std::error::Error for UsageError {}

/// The argument that selects the test-pattern mode.
pub const TEST_PATTERN: &str = "--test-pattern";

/// The base scene that the daemon shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// The clock with the legacy schedule. This is the default.
    Clock,
    /// The heartbeat scene, to check the panel (`--test-pattern`).
    TestPattern,
}

impl Mode {
    /// Make the base scene of this mode.
    #[must_use]
    pub fn base_scene(self, schedule: &Schedule) -> Box<dyn Scene> {
        match self {
            Self::Clock => Box::new(ClockScene::new(schedule.clone())),
            Self::TestPattern => Box::new(Heartbeat),
        }
    }

    /// The name of the mode in commands and replies.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Clock => "clock",
            Self::TestPattern => "test-pattern",
        }
    }

    /// The startup line that names the base scene.
    #[must_use]
    pub fn announcement(self) -> &'static str {
        match self {
            Self::Clock => "gameclockd: the base scene is the clock.",
            Self::TestPattern => "gameclockd: the base scene is the test pattern.",
        }
    }
}

/// The checked command line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Args {
    /// The base scene.
    pub mode: Mode,
    /// The `--led-*` flags in their order, for the driver.
    pub driver: Vec<String>,
    /// The path of the control socket.
    pub socket: PathBuf,
    /// The path of the config file (docs/adr/0019).
    pub config: PathBuf,
    /// Where the config path came from.
    pub config_origin: Origin,
}

/// The checked command line of `gameclockd ctl`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CtlArgs {
    /// The path of the control socket.
    pub socket: PathBuf,
    /// The command to send.
    pub command: String,
}

/// What the binary runs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Invocation {
    /// The daemon.
    Daemon(Args),
    /// The client: send one command (`gameclockd ctl`).
    Ctl(CtlArgs),
}

/// The default path of the control socket (docs/adr/0016).
pub const DEFAULT_SOCKET: &str = "/run/gameclockd/control.sock";

/// The argument that changes the path of the control socket.
const CONTROL_SOCKET: &str = "--control-socket=";

/// The default path of the config file (docs/adr/0019).
pub const DEFAULT_CONFIG: &str = "/etc/gameclockd/config.toml";

/// The argument that changes the path of the config file.
const CONFIG: &str = "--config=";

/// The path of a `--control-socket=<path>` argument, or `None` for another
/// argument.
fn socket_path(arg: &str) -> Option<Result<PathBuf, UsageError>> {
    let path = arg.strip_prefix(CONTROL_SOCKET)?;
    Some(if path.is_empty() {
        Err(UsageError::EmptySocketPath)
    } else {
        Ok(PathBuf::from(path))
    })
}

/// Check the command line. If the first argument is `ctl`, the rest is the
/// client's command line. Otherwise only `--led-*` driver flags,
/// `--test-pattern`, `--control-socket=<path>`, and `--config=<path>` are
/// accepted. The
/// function gives the driver flags unchanged and in their order, for the
/// driver to parse (`openspec/specs/daemon-lifecycle`, docs/adr/0009).
///
/// # Errors
/// [`UsageError`] for the first argument that is not accepted.
pub fn parse_args<I>(args: I) -> Result<Invocation, UsageError>
where
    I: IntoIterator<Item = String>,
{
    let mut args = args.into_iter().peekable();
    if args.peek().is_some_and(|a| a == "ctl") {
        args.next();
        return parse_ctl(args).map(Invocation::Ctl);
    }
    let mut parsed = Args {
        mode: Mode::Clock,
        driver: Vec::new(),
        socket: PathBuf::from(DEFAULT_SOCKET),
        config: PathBuf::from(DEFAULT_CONFIG),
        config_origin: Origin::Default,
    };
    for arg in args {
        if arg == TEST_PATTERN {
            parsed.mode = Mode::TestPattern;
        } else if let Some(path) = socket_path(&arg) {
            parsed.socket = path?;
        } else if let Some(path) = arg.strip_prefix(CONFIG) {
            if path.is_empty() {
                return Err(UsageError::EmptyConfigPath);
            }
            parsed.config = PathBuf::from(path);
            parsed.config_origin = Origin::Argument;
        } else if arg.starts_with("--led-") {
            parsed.driver.push(arg);
        } else {
            return Err(UsageError::Unknown(arg));
        }
    }
    Ok(Invocation::Daemon(parsed))
}

/// The client's arguments: optional socket paths, then the command words.
/// The words are joined with one space. After the first word, a
/// `--control-socket=` argument is a word too.
fn parse_ctl(args: impl Iterator<Item = String>) -> Result<CtlArgs, UsageError> {
    let mut socket = PathBuf::from(DEFAULT_SOCKET);
    let mut words: Vec<String> = Vec::new();
    for arg in args {
        if words.is_empty()
            && let Some(path) = socket_path(&arg)
        {
            socket = path?;
        } else {
            words.push(arg);
        }
    }
    if words.is_empty() {
        return Err(UsageError::Ctl);
    }
    Ok(CtlArgs {
        socket,
        command: words.join(" "),
    })
}

/// A panel for builds without the LED driver. It only logs its presents, so
/// the daemon runs on any machine.
#[derive(Debug)]
pub struct LogPanel {
    size: (usize, usize),
}

impl LogPanel {
    /// A logging panel of `width` × `height` pixels.
    #[must_use]
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            size: (width, height),
        }
    }
}

impl Panel for LogPanel {
    type Error = core::convert::Infallible;

    fn dimensions(&self) -> (usize, usize) {
        self.size
    }

    fn present(&mut self, frame: &Frame, brightness: Brightness) -> Result<(), Self::Error> {
        eprintln!(
            "gameclockd: presented a {}×{} frame at brightness {}%",
            frame.width(),
            frame.height(),
            brightness.percent()
        );
        Ok(())
    }
}

/// The process exit status for a finished run: 0 on success, 1 on error.
#[must_use]
pub fn exit_status<E>(result: &Result<(), RunError<E>>) -> u8 {
    u8::from(result.is_err())
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::cell::Cell;
    use core::convert::Infallible;
    use std::collections::VecDeque;
    use std::path::PathBuf;
    use std::rc::Rc;
    use std::sync::mpsc;

    use crate::command::{Command, OK};
    use crate::config::ConfigError;
    use gameclock_core::alert::{Alert, AlertScene, Alerts, Trigger};
    use gameclock_core::message::{Message, Scroll};
    use gameclock_core::schedule::{Style, TimeOfDay, Weekdays};

    use gameclock_core::embedded_graphics::pixelcolor::Rgb888;
    use gameclock_core::embedded_graphics::prelude::*;
    use gameclock_core::{Brightness, Frame, Scene};
    use jiff::tz::TimeZone;
    use jiff::{Timestamp, Zoned};

    /// Records every present.
    #[derive(Debug)]
    struct Recorder {
        size: (usize, usize),
        presents: Vec<(Frame, Brightness)>,
    }

    impl Recorder {
        fn new(width: usize, height: usize) -> Self {
            Self {
                size: (width, height),
                presents: Vec::new(),
            }
        }
    }

    impl Panel for Recorder {
        type Error = Infallible;
        fn dimensions(&self) -> (usize, usize) {
            self.size
        }
        fn present(&mut self, frame: &Frame, brightness: Brightness) -> Result<(), Infallible> {
            self.presents.push((frame.clone(), brightness));
            Ok(())
        }
    }

    #[derive(Debug)]
    struct Broken;

    #[derive(Debug)]
    struct BrokenError;

    impl fmt::Display for BrokenError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("the LEDs are on fire")
        }
    }

    impl std::error::Error for BrokenError {}

    impl Panel for Broken {
        type Error = BrokenError;
        fn dimensions(&self) -> (usize, usize) {
            (64, 32)
        }
        fn present(&mut self, _: &Frame, _: Brightness) -> Result<(), BrokenError> {
            Err(BrokenError)
        }
    }

    /// A clock that shows the time that the scripted wait sets.
    #[derive(Debug)]
    struct FakeClock(Rc<Cell<Timestamp>>);

    impl Clock for FakeClock {
        fn now(&self) -> Zoned {
            self.0.get().to_zoned(TimeZone::UTC)
        }
    }

    /// One scripted end of a wait.
    #[derive(Debug)]
    enum Step {
        /// The time moves to the deadline, and the wait ends.
        Deadline,
        /// The time moves to this time, and the wait ends before the deadline.
        EarlyAt(&'static str),
        /// The time moves to this time, and this command arrives. The reply
        /// goes to the sender.
        CommandAt(&'static str, Command, mpsc::Sender<String>),
    }

    /// A wait that follows a script and records each deadline. After the last
    /// step, it gives `Wake::Shutdown`.
    #[derive(Debug)]
    struct ScriptedWait {
        now: Rc<Cell<Timestamp>>,
        steps: VecDeque<Step>,
        deadlines: Vec<Option<Timestamp>>,
    }

    impl Wait for ScriptedWait {
        fn wait(&mut self, deadline: Option<Timestamp>, _now: Timestamp) -> Wake {
            self.deadlines.push(deadline);
            match self.steps.pop_front() {
                None => Wake::Shutdown,
                Some(Step::Deadline) => {
                    self.now.set(deadline.expect("a deadline to reach"));
                    Wake::Deadline
                }
                Some(Step::EarlyAt(t)) => {
                    self.now.set(ts(t));
                    Wake::Deadline
                }
                Some(Step::CommandAt(t, command, reply)) => {
                    self.now.set(ts(t));
                    Wake::Command(Request { command, reply })
                }
            }
        }
    }

    fn ts(s: &str) -> Timestamp {
        s.parse().unwrap()
    }

    const START: &str = "1970-01-01T00:00:00.250Z";

    fn script(start: &str, steps: impl IntoIterator<Item = Step>) -> (FakeClock, ScriptedWait) {
        let now = Rc::new(Cell::new(ts(start)));
        let wait = ScriptedWait {
            now: Rc::clone(&now),
            steps: steps.into_iter().collect(),
            deadlines: Vec::new(),
        };
        (FakeClock(now), wait)
    }

    fn run_heartbeat<P: Panel>(
        panel: &mut P,
        steps: impl IntoIterator<Item = Step>,
    ) -> (Result<(), RunError<P::Error>>, ScriptedWait) {
        let (clock, mut wait) = script(START, steps);
        let result = run(
            panel,
            Box::new(Heartbeat),
            Mode::TestPattern,
            Tables::legacy(),
            no_reload,
            &clock,
            &mut wait,
        );
        (result, wait)
    }

    fn legacy_clock() -> Box<dyn Scene> {
        Box::new(ClockScene::new(Schedule::legacy()))
    }

    fn clock_frame(width: usize, height: usize, time: &str) -> Frame {
        let mut f = Frame::new(width, height);
        legacy_clock().render(&ts(time).to_zoned(TimeZone::UTC), &mut f);
        f
    }

    /// A red image that does not change, at brightness 25 before
    /// 1970-01-01T00:00:01Z and `after` from that time.
    struct Dim {
        after: u8,
    }

    impl Scene for Dim {
        fn render(&mut self, _now: &Zoned, frame: &mut Frame) {
            frame.clear(Rgb888::RED).unwrap();
        }
        fn next_change(&self, now: &Zoned) -> Option<Timestamp> {
            let at = ts("1970-01-01T00:00:01Z");
            (now.timestamp() < at).then_some(at)
        }
        fn is_finished(&self, _now: &Zoned) -> bool {
            false
        }
        fn brightness(&self, now: &Zoned) -> Brightness {
            if now.timestamp() < ts("1970-01-01T00:00:01Z") {
                Brightness::new(25)
            } else {
                Brightness::new(self.after)
            }
        }
    }

    fn heartbeat_frame(width: usize, height: usize, time: &str) -> Frame {
        let mut f = Frame::new(width, height);
        Heartbeat.render(&ts(time).to_zoned(TimeZone::UTC), &mut f);
        f
    }

    fn marker_column(frame: &Frame) -> Option<i32> {
        (3..61).find(|&x| frame.pixel(Point::new(x, 1)) == Some(Rgb888::WHITE))
    }

    /// A static red scene.
    struct Red;

    impl Scene for Red {
        fn render(&mut self, _now: &Zoned, frame: &mut Frame) {
            frame.clear(Rgb888::RED).unwrap();
        }
        fn next_change(&self, _now: &Zoned) -> Option<Timestamp> {
            None
        }
        fn is_finished(&self, _now: &Zoned) -> bool {
            false
        }
    }

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn driver_flags_are_accepted_unchanged() {
        let given = args(&["--led-pwm-bits=7", "--led-show-refresh"]);
        let expected = Args {
            mode: Mode::Clock,
            driver: given.clone(),
            socket: PathBuf::from(DEFAULT_SOCKET),
            config: PathBuf::from(DEFAULT_CONFIG),
            config_origin: Origin::Default,
        };
        assert_eq!(parse_args(given), Ok(Invocation::Daemon(expected)));
    }

    #[test]
    fn no_arguments_is_the_clock() {
        let expected = Args {
            mode: Mode::Clock,
            driver: Vec::new(),
            socket: PathBuf::from(DEFAULT_SOCKET),
            config: PathBuf::from(DEFAULT_CONFIG),
            config_origin: Origin::Default,
        };
        assert_eq!(parse_args(Vec::new()), Ok(Invocation::Daemon(expected)));
    }

    #[test]
    fn test_pattern_mode_is_not_a_driver_flag() {
        let given = args(&[
            "--led-pwm-bits=8",
            "--test-pattern",
            "--led-slowdown-gpio=1",
        ]);
        let expected = Args {
            mode: Mode::TestPattern,
            driver: args(&["--led-pwm-bits=8", "--led-slowdown-gpio=1"]),
            socket: PathBuf::from(DEFAULT_SOCKET),
            config: PathBuf::from(DEFAULT_CONFIG),
            config_origin: Origin::Default,
        };
        assert_eq!(parse_args(given), Ok(Invocation::Daemon(expected)));
    }

    #[test]
    fn socket_path_is_not_a_driver_flag() {
        let given = args(&["--control-socket=/tmp/a.sock", "--led-pwm-bits=8"]);
        let expected = Args {
            mode: Mode::Clock,
            driver: args(&["--led-pwm-bits=8"]),
            socket: PathBuf::from("/tmp/a.sock"),
            config: PathBuf::from(DEFAULT_CONFIG),
            config_origin: Origin::Default,
        };
        assert_eq!(parse_args(given), Ok(Invocation::Daemon(expected)));
    }

    #[test]
    fn default_config_path() {
        assert_eq!(DEFAULT_CONFIG, "/etc/gameclockd/config.toml");
        let Ok(Invocation::Daemon(parsed)) = parse_args(args(&[])) else {
            panic!("a daemon invocation");
        };
        assert_eq!(parsed.config, PathBuf::from(DEFAULT_CONFIG));
        assert_eq!(parsed.config_origin, Origin::Default);
    }

    #[test]
    fn config_path_from_the_argument() {
        let given = args(&["--config=/tmp/gc.toml", "--led-pwm-bits=8"]);
        let expected = Args {
            mode: Mode::Clock,
            driver: args(&["--led-pwm-bits=8"]),
            socket: PathBuf::from(DEFAULT_SOCKET),
            config: PathBuf::from("/tmp/gc.toml"),
            config_origin: Origin::Argument,
        };
        assert_eq!(parse_args(given), Ok(Invocation::Daemon(expected)));
    }

    #[test]
    fn empty_config_path_is_a_usage_error() {
        let err = parse_args(args(&["--config="])).unwrap_err();
        assert_eq!(err, UsageError::EmptyConfigPath);
        assert!(err.to_string().contains("--config"), "{err}");
    }

    #[test]
    fn default_socket_path() {
        assert_eq!(DEFAULT_SOCKET, "/run/gameclockd/control.sock");
    }

    #[test]
    fn empty_socket_path_is_a_usage_error() {
        let err = parse_args(args(&["--control-socket="])).unwrap_err();
        assert_eq!(err, UsageError::EmptySocketPath);
        assert!(err.to_string().contains("--control-socket"), "{err}");
    }

    #[test]
    fn ctl_with_a_command() {
        let expected = CtlArgs {
            socket: PathBuf::from(DEFAULT_SOCKET),
            command: "status".into(),
        };
        assert_eq!(
            parse_args(args(&["ctl", "status"])),
            Ok(Invocation::Ctl(expected))
        );
    }

    #[test]
    fn ctl_with_a_socket_path() {
        let expected = CtlArgs {
            socket: PathBuf::from("/tmp/b.sock"),
            command: "clock".into(),
        };
        let given = args(&["ctl", "--control-socket=/tmp/b.sock", "clock"]);
        assert_eq!(parse_args(given), Ok(Invocation::Ctl(expected)));
    }

    #[test]
    fn ctl_needs_a_command() {
        for given in [&["ctl"][..], &["ctl", "--control-socket=/tmp/c.sock"]] {
            assert_eq!(parse_args(args(given)), Err(UsageError::Ctl), "{given:?}");
        }
        assert_eq!(
            UsageError::Ctl.to_string(),
            "usage: gameclockd ctl [--control-socket=<path>] <command> [<word>...]"
        );
    }

    #[test]
    fn message_with_no_quotes() {
        let expected = CtlArgs {
            socket: PathBuf::from(DEFAULT_SOCKET),
            command: "message hello world".into(),
        };
        let given = args(&["ctl", "message", "hello", "world"]);
        assert_eq!(parse_args(given), Ok(Invocation::Ctl(expected)));
    }

    #[test]
    fn socket_option_after_the_command_word() {
        let expected = CtlArgs {
            socket: PathBuf::from(DEFAULT_SOCKET),
            command: "message --control-socket=/tmp/a.sock".into(),
        };
        let given = args(&["ctl", "message", "--control-socket=/tmp/a.sock"]);
        assert_eq!(parse_args(given), Ok(Invocation::Ctl(expected)));
    }

    #[test]
    fn ctl_only_as_the_first_argument() {
        let err = parse_args(args(&["--test-pattern", "ctl"])).unwrap_err();
        assert_eq!(err, UsageError::Unknown("ctl".into()));
    }

    #[test]
    fn modes_name_their_base_scene() {
        assert_eq!(
            Mode::Clock.announcement(),
            "gameclockd: the base scene is the clock."
        );
        assert_eq!(
            Mode::TestPattern.announcement(),
            "gameclockd: the base scene is the test pattern."
        );
    }

    #[test]
    fn modes_make_their_base_scene() {
        let now = ts("2026-11-10T09:05:00Z").to_zoned(TimeZone::UTC);
        let mut clock = Frame::new(64, 32);
        Mode::Clock
            .base_scene(&Schedule::legacy())
            .render(&now, &mut clock);
        let mut expected = Frame::new(64, 32);
        legacy_clock().render(&now, &mut expected);
        assert_eq!(clock, expected);

        let mut pattern = Frame::new(64, 32);
        Mode::TestPattern
            .base_scene(&Schedule::legacy())
            .render(&now, &mut pattern);
        let mut expected = Frame::new(64, 32);
        Heartbeat.render(&now, &mut expected);
        assert_eq!(pattern, expected);
    }

    #[test]
    fn unknown_argument_is_a_usage_error_naming_it() {
        let err = parse_args(args(&["--led-rows=32", "--frobnicate"])).unwrap_err();
        assert_eq!(err, UsageError::Unknown("--frobnicate".into()));
        assert!(err.to_string().contains("--frobnicate"), "{err}");
    }

    #[test]
    fn bare_words_are_usage_errors() {
        assert!(parse_args(args(&["led-rows=32"])).is_err());
    }

    #[test]
    fn one_present_at_startup() {
        let mut panel = Recorder::new(64, 32);
        let (clock, mut wait) = script(START, []);
        run(
            &mut panel,
            legacy_clock(),
            Mode::Clock,
            Tables::legacy(),
            no_reload,
            &clock,
            &mut wait,
        )
        .unwrap();
        assert_eq!(panel.presents.len(), 1);
        let (frame, brightness) = &panel.presents[0];
        assert_eq!(*frame, clock_frame(64, 32, START));
        assert_eq!(brightness.percent(), 25);
    }

    #[test]
    fn heartbeat_presents_at_full_brightness() {
        let mut panel = Recorder::new(64, 32);
        run_heartbeat(&mut panel, []).0.unwrap();
        assert_eq!(panel.presents[0].0, heartbeat_frame(64, 32, START));
        assert_eq!(panel.presents[0].1, Brightness::MAX);
    }

    #[test]
    fn brightness_of_the_base_scene() {
        let mut panel = Recorder::new(64, 32);
        let (clock, mut wait) = script(START, []);
        run(
            &mut panel,
            Box::new(Dim { after: 25 }),
            Mode::Clock,
            Tables::legacy(),
            no_reload,
            &clock,
            &mut wait,
        )
        .unwrap();
        assert_eq!(panel.presents.len(), 1);
        assert_eq!(panel.presents[0].1, Brightness::new(25));
    }

    #[test]
    fn brightness_change_presents_the_same_image() {
        let mut panel = Recorder::new(64, 32);
        let (clock, mut wait) = script(START, [Step::Deadline]);
        run(
            &mut panel,
            Box::new(Dim { after: 60 }),
            Mode::Clock,
            Tables::legacy(),
            no_reload,
            &clock,
            &mut wait,
        )
        .unwrap();
        assert_eq!(panel.presents.len(), 2);
        assert_eq!(panel.presents[0].0, panel.presents[1].0);
        let levels: Vec<_> = panel.presents.iter().map(|(_, b)| b.percent()).collect();
        assert_eq!(levels, vec![25, 60]);
    }

    #[test]
    fn frame_follows_the_panels_size() {
        let mut panel = Recorder::new(32, 16);
        run_heartbeat(&mut panel, []).0.unwrap();
        assert_eq!(panel.presents.len(), 1);
        assert_eq!(panel.presents[0].0, heartbeat_frame(32, 16, START));
    }

    #[test]
    fn wait_ends_at_the_scene_deadline() {
        let (_, wait) = run_heartbeat(&mut Recorder::new(64, 32), []);
        assert_eq!(wait.deadlines[0], Some(ts("1970-01-01T00:00:01Z")));
    }

    #[test]
    fn static_scene_waits_until_the_next_trigger() {
        let mut panel = Recorder::new(64, 32);
        let (clock, mut wait) = script(START, []);
        run(
            &mut panel,
            Box::new(Red),
            Mode::Clock,
            Tables::legacy(),
            no_reload,
            &clock,
            &mut wait,
        )
        .unwrap();
        // START is a Thursday, a bedtime night (`openspec/specs/alerts`).
        assert_eq!(wait.deadlines, vec![Some(ts("1970-01-01T23:00:00Z"))]);
        assert_eq!(panel.presents.len(), 1);
    }

    // Commands (openspec/specs/command-input).

    /// Run `base` in `mode` with one command at `at`. Give the presents and
    /// the reply.
    fn run_command(
        base: Box<dyn Scene>,
        mode: Mode,
        at: &'static str,
        command: Command,
    ) -> (Recorder, ScriptedWait, String) {
        let mut panel = Recorder::new(64, 32);
        let (tx, rx) = mpsc::channel();
        let (clock, mut wait) = script(START, [Step::CommandAt(at, command, tx)]);
        run(
            &mut panel,
            base,
            mode,
            Tables::legacy(),
            no_reload,
            &clock,
            &mut wait,
        )
        .unwrap();
        let reply = rx.try_recv().expect("a reply");
        (panel, wait, reply)
    }

    #[test]
    fn a_command_ends_the_wait() {
        let (panel, wait, reply) = run_command(
            Box::new(Red),
            Mode::Clock,
            "1970-01-01T00:00:00.500Z",
            Command::Status,
        );
        assert!(reply.starts_with("ok"), "{reply}");
        let trigger = Some(ts("1970-01-01T23:00:00Z"));
        assert_eq!(wait.deadlines, vec![trigger, trigger]);
        assert_eq!(panel.presents.len(), 1);
    }

    #[test]
    fn switch_to_the_test_pattern() {
        let at = "1970-01-01T00:00:00.500Z";
        let (panel, _, reply) = run_command(
            legacy_clock(),
            Mode::Clock,
            at,
            Command::Mode(Mode::TestPattern),
        );
        assert_eq!(reply, OK);
        assert_eq!(panel.presents.len(), 2);
        assert_eq!(panel.presents[1].0, heartbeat_frame(64, 32, at));
        assert_eq!(panel.presents[1].1, Brightness::MAX);
    }

    #[test]
    fn switch_back_to_the_clock() {
        let at = "1970-01-01T00:00:00.500Z";
        let (panel, _, reply) = run_command(
            Box::new(Heartbeat),
            Mode::TestPattern,
            at,
            Command::Mode(Mode::Clock),
        );
        assert_eq!(reply, OK);
        assert_eq!(panel.presents.len(), 2);
        assert_eq!(panel.presents[1].0, clock_frame(64, 32, at));
        assert_eq!(panel.presents[1].1, Brightness::new(25));
    }

    #[test]
    fn same_mode_does_not_present() {
        let (panel, _, reply) = run_command(
            legacy_clock(),
            Mode::Clock,
            "1970-01-01T00:00:00.500Z",
            Command::Mode(Mode::Clock),
        );
        assert_eq!(reply, OK);
        assert_eq!(panel.presents.len(), 1);
    }

    #[test]
    fn status_of_the_clock() {
        let (_, _, reply) = run_command(legacy_clock(), Mode::Clock, START, Command::Status);
        assert_eq!(
            reply,
            "ok mode=clock zone=UTC brightness=25 messages=0 alert=none"
        );
    }

    #[test]
    fn status_of_the_test_pattern() {
        let (_, _, reply) = run_command(
            Box::new(Heartbeat),
            Mode::TestPattern,
            START,
            Command::Status,
        );
        assert_eq!(
            reply,
            "ok mode=test-pattern zone=UTC brightness=100 messages=0 alert=none"
        );
    }

    // Messages (openspec/specs/message-board).

    const T1: &str = "1970-01-01T00:00:00.500Z";

    /// `ms` milliseconds after T1.
    fn after_t1(ms: i64) -> &'static str {
        let t = ts(T1) + jiff::SignedDuration::from_millis(ms);
        Box::leak(t.to_string().into_boxed_str())
    }

    fn message(text: &str, passes: u8) -> Command {
        Command::Message(Message::new(text, passes, Message::DEFAULT_COLOR).unwrap())
    }

    /// The frame of `text` that starts at `start`, at the time `at`.
    fn scroll_frame(text: &str, start: &str, at: &str) -> Frame {
        let m = Message::new(text, 10, Message::DEFAULT_COLOR).unwrap();
        let mut scroll = Scroll::new(m, ts(start), 64, Schedule::legacy());
        let mut f = Frame::new(64, 32);
        scroll.render(&ts(at).to_zoned(TimeZone::UTC), &mut f);
        f
    }

    /// The finish time of the 1-pass message `one`, which starts at T1:
    /// (64 + 3 × 7) steps of 40 ms.
    const ONE_PASS_MS: i64 = 85 * 40;

    /// A command at `at`, and the receiver for its reply.
    fn command_at(at: &'static str, command: Command) -> (Step, mpsc::Receiver<String>) {
        let (tx, rx) = mpsc::channel();
        (Step::CommandAt(at, command, tx), rx)
    }

    fn run_clock(steps: Vec<Step>) -> (Recorder, ScriptedWait) {
        let mut panel = Recorder::new(64, 32);
        let (clock, mut wait) = script(START, steps);
        run(
            &mut panel,
            legacy_clock(),
            Mode::Clock,
            Tables::legacy(),
            no_reload,
            &clock,
            &mut wait,
        )
        .unwrap();
        (panel, wait)
    }

    #[test]
    fn first_message_starts_at_once() {
        let (tx, rx) = mpsc::channel();
        let replies = Rc::new(rx);
        let mut panel = ReplyWatch {
            inner: Recorder::new(64, 32),
            replies: Rc::clone(&replies),
            reply_before_present: Vec::new(),
        };
        let (clock, mut wait) = script(START, [Step::CommandAt(T1, message("hello", 4), tx)]);
        run(
            &mut panel,
            legacy_clock(),
            Mode::Clock,
            Tables::legacy(),
            no_reload,
            &clock,
            &mut wait,
        )
        .unwrap();
        assert_eq!(panel.reply_before_present, vec![false, false]);
        assert_eq!(replies.try_recv().as_deref(), Ok(OK));
        assert_eq!(panel.inner.presents[1].0, scroll_frame("hello", T1, T1));
        assert_eq!(panel.inner.presents[1].1, Brightness::new(25));
    }

    #[test]
    fn second_message_waits() {
        let (one, _r1) = command_at(T1, message("one", 1));
        let (two, r2) = command_at(after_t1(1000), message("two", 1));
        let (panel, _) = run_clock(vec![one, two, Step::EarlyAt(after_t1(2000))]);
        assert_eq!(r2.try_recv().as_deref(), Ok(OK));
        let last = &panel.presents.last().unwrap().0;
        assert_eq!(*last, scroll_frame("one", T1, after_t1(2000)));
    }

    #[test]
    fn next_message_starts_with_no_clock_between() {
        let finish = after_t1(ONE_PASS_MS);
        let (one, _r1) = command_at(T1, message("one", 1));
        let (two, _r2) = command_at(T1, message("two", 1));
        let steps = vec![
            one,
            two,
            Step::EarlyAt(after_t1(2000)),
            Step::EarlyAt(finish),
            Step::EarlyAt(after_t1(ONE_PASS_MS + 1000)),
        ];
        let (panel, _) = run_clock(steps);
        let clock = clock_frame(64, 32, START);
        assert!(panel.presents[1..].iter().all(|(f, _)| *f != clock));
        let last = &panel.presents.last().unwrap().0;
        let expected = scroll_frame("two", finish, after_t1(ONE_PASS_MS + 1000));
        assert_eq!(*last, expected, "two starts at the finish of one");
    }

    #[test]
    fn clock_returns_after_the_last_message() {
        let (one, _r1) = command_at(T1, message("one", 1));
        let steps = vec![
            one,
            Step::EarlyAt(after_t1(2000)),
            Step::EarlyAt(after_t1(ONE_PASS_MS)),
        ];
        let (panel, wait) = run_clock(steps);
        let last = &panel.presents.last().unwrap().0;
        assert_eq!(*last, clock_frame(64, 32, START));
        assert_eq!(
            wait.deadlines.last(),
            Some(&Some(ts("1970-01-01T00:01:00Z")))
        );
    }

    #[test]
    fn queue_is_full() {
        let mut steps = Vec::new();
        let mut replies = Vec::new();
        for n in 0..10 {
            let (step, rx) = command_at(T1, message(&format!("m{n}"), 1));
            steps.push(step);
            replies.push(rx);
        }
        run_clock(steps);
        let replies: Vec<_> = replies.iter().map(|r| r.try_recv().unwrap()).collect();
        assert!(replies[..9].iter().all(|r| r == OK), "{replies:?}");
        assert_eq!(
            replies[9],
            "error: the message queue is full. 8 messages wait."
        );
    }

    #[test]
    fn clear_during_a_message() {
        let (tx, rx) = mpsc::channel();
        let replies = Rc::new(rx);
        let mut panel = ReplyWatch {
            inner: Recorder::new(64, 32),
            replies: Rc::clone(&replies),
            reply_before_present: Vec::new(),
        };
        let (one, _r1) = command_at(T1, message("one", 4));
        let (two, _r2) = command_at(T1, message("two", 4));
        let (three, _r3) = command_at(T1, message("three", 4));
        let steps = vec![
            one,
            two,
            three,
            Step::EarlyAt(after_t1(1000)),
            Step::CommandAt(after_t1(1500), Command::Clear, tx),
            Step::EarlyAt(after_t1(20_000)),
        ];
        let (clock, mut wait) = script(START, steps);
        run(
            &mut panel,
            legacy_clock(),
            Mode::Clock,
            Tables::legacy(),
            no_reload,
            &clock,
            &mut wait,
        )
        .unwrap();
        assert_eq!(replies.try_recv().as_deref(), Ok(OK));
        let presents = &panel.inner.presents;
        assert_eq!(presents.len(), 4, "the clock, one, one, the clock");
        assert_eq!(presents[3].0, clock_frame(64, 32, START));
        assert!(panel.reply_before_present.iter().all(|&r| !r));
    }

    #[test]
    fn clear_with_no_message() {
        let (panel, _, reply) = run_command(legacy_clock(), Mode::Clock, T1, Command::Clear);
        assert_eq!(reply, OK);
        assert_eq!(panel.presents.len(), 1);
    }

    #[test]
    fn status_during_a_message() {
        let (one, _r1) = command_at(T1, message("one", 4));
        let (two, _r2) = command_at(T1, message("two", 4));
        let (three, _r3) = command_at(T1, message("three", 4));
        let (status, reply) = command_at(after_t1(100), Command::Status);
        run_clock(vec![one, two, three, status]);
        let reply = reply.try_recv().unwrap();
        assert!(reply.ends_with(" messages=3 alert=none"), "{reply}");
    }

    #[test]
    fn message_over_the_test_pattern() {
        let (panel, _, reply) =
            run_command(Box::new(Heartbeat), Mode::TestPattern, T1, message("hi", 4));
        assert_eq!(reply, OK);
        assert_eq!(panel.presents[0].1, Brightness::MAX);
        assert_eq!(panel.presents[1].1, Brightness::new(25));
    }

    /// The reload function of a test with no `reload` command.
    fn no_reload() -> Result<Tables, ConfigError> {
        panic!("this test sends no reload command");
    }

    // The reload command reads the file again (`openspec/specs/daemon-config`).

    fn green() -> Tables {
        let green = Style::new(Rgb888::new(0, 255, 0), Brightness::new(60));
        Tables {
            schedule: Schedule::new(Vec::new(), green),
            ..Tables::legacy()
        }
    }

    fn bad_file() -> ConfigError {
        ConfigError::Parse {
            path: "/tmp/gc.toml".into(),
            line: 3,
            column: 14,
            message: "the brightness must be from 0 to 100.".into(),
        }
    }

    /// Run the clock with `steps`, from `start`. Each reload gives the next
    /// result of `reloads`.
    fn run_reloads(
        start: &str,
        base: Box<dyn Scene>,
        mode: Mode,
        reloads: Vec<Result<Tables, ConfigError>>,
        steps: Vec<Step>,
    ) -> ReplyWatch {
        let (_, rx) = mpsc::channel();
        let mut panel = ReplyWatch {
            inner: Recorder::new(64, 32),
            replies: Rc::new(rx),
            reply_before_present: Vec::new(),
        };
        let mut reloads = reloads.into_iter();
        let reload = move || reloads.next().expect("a scripted reload");
        let (clock, mut wait) = script(start, steps);
        run(
            &mut panel,
            base,
            mode,
            Tables::legacy(),
            reload,
            &clock,
            &mut wait,
        )
        .unwrap();
        panel
    }

    fn green_clock_frame(time: &str) -> Frame {
        let mut f = Frame::new(64, 32);
        ClockScene::new(green().schedule).render(&ts(time).to_zoned(TimeZone::UTC), &mut f);
        f
    }

    #[test]
    fn reload_with_a_new_schedule() {
        let (tx, rx) = mpsc::channel();
        let replies = Rc::new(rx);
        let mut panel = ReplyWatch {
            inner: Recorder::new(64, 32),
            replies: Rc::clone(&replies),
            reply_before_present: Vec::new(),
        };
        let mut reloads = vec![Ok(green())].into_iter();
        let reload = move || reloads.next().expect("one reload");
        let (clock, mut wait) = script(START, [Step::CommandAt(T1, Command::Reload, tx)]);
        run(
            &mut panel,
            legacy_clock(),
            Mode::Clock,
            Tables::legacy(),
            reload,
            &clock,
            &mut wait,
        )
        .unwrap();
        assert_eq!(replies.try_recv().as_deref(), Ok(OK));
        assert_eq!(panel.reply_before_present, vec![false, false]);
        assert_eq!(panel.inner.presents[1].0, green_clock_frame(T1));
        assert_eq!(panel.inner.presents[1].1, Brightness::new(60));
    }

    #[test]
    fn reload_with_an_error() {
        let (step, reply) = command_at(T1, Command::Reload);
        let panel = run_reloads(
            START,
            legacy_clock(),
            Mode::Clock,
            vec![Err(bad_file())],
            vec![step, Step::EarlyAt(after_t1(60_000))],
        );
        assert_eq!(
            reply.try_recv().unwrap(),
            "error: /tmp/gc.toml:3:14: the brightness must be from 0 to 100."
        );
        let clock = clock_frame(64, 32, START);
        assert!(
            panel
                .inner
                .presents
                .iter()
                .all(|(f, _)| *f == clock || *f == clock_frame(64, 32, after_t1(60_000)))
        );
    }

    #[test]
    fn reload_during_an_alert() {
        let mut changed = Tables::legacy();
        let mut panic = (*changed.alerts.find("panic").unwrap()).clone();
        panic.text = "HELP".to_owned();
        let mut list: Vec<Alert> = changed.alerts.iter().map(|a| (**a).clone()).collect();
        list[0] = panic;
        changed.alerts = Alerts::new(list).unwrap();
        let (start_panic, _r1) = command_at(T1, alert("panic"));
        let (reload, r2) = command_at(after_t1(1000), Command::Reload);
        let (again, _r3) = command_at(after_t1(21_000), alert("panic"));
        let steps = vec![start_panic, reload, Step::EarlyAt(after_t1(2000)), again];
        let panel = run_reloads(
            START,
            legacy_clock(),
            Mode::Clock,
            vec![Ok(changed.clone())],
            steps,
        );
        assert_eq!(r2.try_recv().as_deref(), Ok(OK));
        let frames: Vec<_> = panel
            .inner
            .presents
            .iter()
            .map(|(f, _)| f.clone())
            .collect();
        assert!(
            frames.contains(&alert_frame("panic", T1, after_t1(2000))),
            "the old panic continues"
        );
        let new_panic = changed.alerts.find("panic").unwrap();
        let mut f = Frame::new(64, 32);
        AlertScene::new(new_panic, ts(after_t1(21_000)), 64, Schedule::legacy())
            .render(&ts(after_t1(21_000)).to_zoned(TimeZone::UTC), &mut f);
        assert_eq!(
            *frames.last().unwrap(),
            f,
            "the next panic uses the new definition"
        );
    }

    #[test]
    fn reload_restarts_the_triggers() {
        // 2026-11-16 is a Monday. The built-in triggers start at 23:00.
        let mut every_minute = Tables::legacy();
        let panic = every_minute.alerts.find("panic").unwrap();
        let rule = Trigger::new(
            Weekdays::EACH_DAY,
            TimeOfDay::hm(22, 0).unwrap(),
            TimeOfDay::hm(23, 0).unwrap(),
            jiff::SignedDuration::from_secs(60),
            panic,
        )
        .unwrap();
        every_minute.triggers = vec![rule];
        let (reload, r) = command_at("2026-11-16T22:00:00.500Z", Command::Reload);
        let steps = vec![reload, Step::Deadline];
        let panel = run_reloads(
            "2026-11-16T22:00:00.250Z",
            legacy_clock(),
            Mode::Clock,
            vec![Ok(every_minute)],
            steps,
        );
        assert_eq!(r.try_recv().as_deref(), Ok(OK));
        let at = "2026-11-16T22:01:00Z";
        assert_eq!(
            panel.inner.presents.last().unwrap().0,
            alert_frame("panic", at, at)
        );
    }

    #[test]
    fn reload_in_the_test_pattern_mode() {
        let (reload, r1) = command_at(T1, Command::Reload);
        let (clock, r2) = command_at(after_t1(100), Command::Mode(Mode::Clock));
        let panel = run_reloads(
            START,
            Box::new(Heartbeat),
            Mode::TestPattern,
            vec![Ok(green())],
            vec![reload, clock],
        );
        assert_eq!(r1.try_recv().as_deref(), Ok(OK));
        assert_eq!(r2.try_recv().as_deref(), Ok(OK));
        let presents = &panel.inner.presents;
        assert_eq!(presents.len(), 2, "the heartbeat, then the green clock");
        assert_eq!(presents[0].0, heartbeat_frame(64, 32, START));
        assert_eq!(presents[1].0, green_clock_frame(after_t1(100)));
    }

    #[test]
    fn alert_with_no_name_in_the_loop() {
        let (step, reply) = command_at(T1, Command::Alert(None));
        run_clock(vec![step]);
        assert_eq!(
            reply.try_recv().unwrap(),
            "error: the alert command has no name. The alerts are panic, bedtime, and bed-flash."
        );
    }

    #[test]
    fn unknown_alert_in_the_loop() {
        let (step, reply) = command_at(T1, alert("fire"));
        run_clock(vec![step]);
        assert_eq!(
            reply.try_recv().unwrap(),
            "error: unknown alert 'fire'. The alerts are panic, bedtime, and bed-flash."
        );
    }

    #[test]
    fn unknown_alert_names_the_current_table() {
        let dinner = Alert {
            name: "dinner".to_owned(),
            ..(*Alerts::legacy().find("bed-flash").unwrap()).clone()
        };
        let only_dinner = Tables {
            alerts: Alerts::new(vec![dinner]).unwrap(),
            triggers: Vec::new(),
            ..Tables::legacy()
        };
        let (reload, _r1) = command_at(T1, Command::Reload);
        let (panic, r2) = command_at(after_t1(100), alert("panic"));
        run_reloads(
            START,
            legacy_clock(),
            Mode::Clock,
            vec![Ok(only_dinner)],
            vec![reload, panic],
        );
        assert_eq!(
            r2.try_recv().unwrap(),
            "error: unknown alert 'panic'. The alerts are dinner."
        );
    }

    // Alerts and timed triggers (`openspec/specs/alerts`).

    fn alert(name: &str) -> Command {
        Command::Alert(Some(name.to_owned()))
    }

    /// The frame of the alert `name` that starts at `start`, at the time `at`.
    fn alert_frame(name: &str, start: &str, at: &str) -> Frame {
        let a = gameclock_core::alert::Alerts::legacy().find(name).unwrap();
        let mut scene = AlertScene::new(a, ts(start), 64, Schedule::legacy());
        let mut f = Frame::new(64, 32);
        scene.render(&ts(at).to_zoned(TimeZone::UTC), &mut f);
        f
    }

    fn run_clock_from(start: &str, steps: Vec<Step>) -> (Recorder, ScriptedWait) {
        let mut panel = Recorder::new(64, 32);
        let (clock, mut wait) = script(start, steps);
        run(
            &mut panel,
            legacy_clock(),
            Mode::Clock,
            Tables::legacy(),
            no_reload,
            &clock,
            &mut wait,
        )
        .unwrap();
        (panel, wait)
    }

    #[test]
    fn alert_over_the_clock() {
        let (tx, rx) = mpsc::channel();
        let replies = Rc::new(rx);
        let mut panel = ReplyWatch {
            inner: Recorder::new(64, 32),
            replies: Rc::clone(&replies),
            reply_before_present: Vec::new(),
        };
        let (clock, mut wait) = script(START, [Step::CommandAt(T1, alert("panic"), tx)]);
        run(
            &mut panel,
            legacy_clock(),
            Mode::Clock,
            Tables::legacy(),
            no_reload,
            &clock,
            &mut wait,
        )
        .unwrap();
        assert_eq!(panel.reply_before_present, vec![false, false]);
        assert_eq!(replies.try_recv().as_deref(), Ok(OK));
        assert_eq!(panel.inner.presents[1].0, alert_frame("panic", T1, T1));
    }

    #[test]
    fn alert_command_errors() {
        let (one, _r1) = command_at(T1, message("one", 4));
        let (flash, r2) = command_at(after_t1(100), alert("bed-flash"));
        let (stop, r3) = command_at(after_t1(200), Command::StopAlert);
        run_clock(vec![one, flash, stop]);
        assert_eq!(
            r2.try_recv().unwrap(),
            "error: the alert 'bed-flash' did not start. A message on the panel has the same or a higher priority."
        );
        assert_eq!(r3.try_recv().as_deref(), Ok(OK));
    }

    #[test]
    fn status_during_an_alert() {
        let (one, _r1) = command_at(T1, message("one", 4));
        let (two, _r2) = command_at(T1, message("two", 4));
        let (panic, _r3) = command_at(after_t1(100), alert("panic"));
        let (status, reply) = command_at(after_t1(200), Command::Status);
        run_clock(vec![one, two, panic, status]);
        let reply = reply.try_recv().unwrap();
        assert!(reply.ends_with(" messages=2 alert=panic"), "{reply}");
    }

    // 2026-11-16 is a Monday.

    #[test]
    fn wake_before_the_next_minute() {
        let (_, wait) = run_clock_from("2026-11-16T23:00:05Z", vec![]);
        assert_eq!(wait.deadlines[0], Some(ts("2026-11-16T23:00:10Z")));
    }

    #[test]
    fn no_early_wake_outside_a_window() {
        let (_, wait) = run_clock_from("2026-11-16T12:00:05Z", vec![]);
        assert_eq!(wait.deadlines[0], Some(ts("2026-11-16T12:01:00Z")));
    }

    #[test]
    fn bedtime_alert_repeats_with_no_gap() {
        let steps = vec![
            Step::Deadline,
            Step::EarlyAt("2026-11-16T23:05:10Z"),
            Step::EarlyAt("2026-11-16T23:05:20Z"),
            Step::EarlyAt("2026-11-16T23:05:21Z"),
        ];
        let (panel, wait) = run_clock_from("2026-11-16T23:04:59Z", steps);
        assert_eq!(wait.deadlines[0], Some(ts("2026-11-16T23:05:00Z")));
        let frames: Vec<_> = panel.presents.iter().map(|(f, _)| f).collect();
        assert_eq!(
            *frames[2],
            alert_frame("bedtime", "2026-11-16T23:05:00Z", "2026-11-16T23:05:10Z"),
            "the trigger at 23:05:10 changes nothing"
        );
        assert_eq!(
            **frames.last().unwrap(),
            alert_frame("bedtime", "2026-11-16T23:05:20Z", "2026-11-16T23:05:21Z"),
            "a new bedtime starts at 23:05:20"
        );
        let clock = clock_frame(64, 32, "2026-11-16T23:05:20Z");
        assert!(frames[1..].iter().all(|f| **f != clock));
    }

    #[test]
    fn refused_trigger_during_a_message() {
        let start = "2026-11-16T22:59:59.500Z";
        let (hi, _r) = command_at(start, message("HI", 10));
        let steps = vec![hi, Step::EarlyAt("2026-11-16T23:00:00Z")];
        let (panel, _) = run_clock_from(start, steps);
        let last = &panel.presents.last().unwrap().0;
        assert_eq!(*last, scroll_frame("HI", start, "2026-11-16T23:00:00Z"));
    }

    #[test]
    fn trigger_in_test_pattern_mode() {
        let mut panel = Recorder::new(64, 32);
        let (clock, mut wait) = script(
            "2026-11-16T22:59:59.900Z",
            [Step::EarlyAt("2026-11-16T23:00:00Z")],
        );
        run(
            &mut panel,
            Box::new(Heartbeat),
            Mode::TestPattern,
            Tables::legacy(),
            no_reload,
            &clock,
            &mut wait,
        )
        .unwrap();
        let last = &panel.presents.last().unwrap().0;
        let at = "2026-11-16T23:00:00Z";
        assert_eq!(*last, alert_frame("bed-flash", at, at));
    }

    /// Records, at each present, whether a reply was already sent.
    struct ReplyWatch {
        inner: Recorder,
        replies: Rc<mpsc::Receiver<String>>,
        reply_before_present: Vec<bool>,
    }

    impl Panel for ReplyWatch {
        type Error = Infallible;
        fn dimensions(&self) -> (usize, usize) {
            self.inner.dimensions()
        }
        fn present(&mut self, frame: &Frame, brightness: Brightness) -> Result<(), Infallible> {
            self.reply_before_present
                .push(self.replies.try_recv().is_ok());
            self.inner.present(frame, brightness)
        }
    }

    #[test]
    fn the_reply_comes_after_the_present() {
        let (tx, rx) = mpsc::channel();
        let replies = Rc::new(rx);
        let mut panel = ReplyWatch {
            inner: Recorder::new(64, 32),
            replies: Rc::clone(&replies),
            reply_before_present: Vec::new(),
        };
        let step = Step::CommandAt(
            "1970-01-01T00:00:00.500Z",
            Command::Mode(Mode::TestPattern),
            tx,
        );
        let (clock, mut wait) = script(START, [step]);
        run(
            &mut panel,
            legacy_clock(),
            Mode::Clock,
            Tables::legacy(),
            no_reload,
            &clock,
            &mut wait,
        )
        .unwrap();
        assert_eq!(panel.reply_before_present, vec![false, false]);
        assert_eq!(replies.try_recv().as_deref(), Ok(OK));
    }

    #[test]
    fn heartbeat_presents_each_second() {
        let mut panel = Recorder::new(64, 32);
        let steps = [Step::Deadline, Step::Deadline, Step::Deadline];
        run_heartbeat(&mut panel, steps).0.unwrap();
        let columns: Vec<_> = panel
            .presents
            .iter()
            .map(|(f, _)| marker_column(f))
            .collect();
        assert_eq!(columns, vec![Some(3), Some(4), Some(5), Some(6)]);
    }

    #[test]
    fn early_wake_does_not_present() {
        let mut panel = Recorder::new(64, 32);
        let steps = [Step::EarlyAt("1970-01-01T00:00:00.900Z")];
        let (result, wait) = run_heartbeat(&mut panel, steps);
        result.unwrap();
        assert_eq!(panel.presents.len(), 1);
        let second = ts("1970-01-01T00:00:01Z");
        assert_eq!(wait.deadlines, vec![Some(second), Some(second)]);
    }

    #[test]
    fn messages_are_exact() {
        let err = run_heartbeat(&mut Broken, []).0.unwrap_err();
        assert_eq!(
            err.to_string(),
            "the panel did not show the frame: the LEDs are on fire"
        );
        assert_eq!(
            UsageError::Unknown("--frobnicate".into()).to_string(),
            "unknown argument '--frobnicate'. gameclockd accepts only --led-* driver flags, --test-pattern, --control-socket=<path>, and --config=<path>."
        );
    }

    #[test]
    fn present_failure_is_reported_and_exits_non_zero() {
        let (result, _) = run_heartbeat(&mut Broken, []);
        let err = result.as_ref().unwrap_err();
        assert!(
            err.to_string().contains("the LEDs are on fire"),
            "error was: {err}"
        );
        assert_ne!(exit_status(&result), 0);
    }

    #[test]
    fn success_exits_zero() {
        let (result, _) = run_heartbeat(&mut Recorder::new(64, 32), []);
        assert_eq!(exit_status(&result), 0);
    }

    mod channel_wait {
        use super::*;
        use std::time::{Duration, Instant};

        fn channel() -> (ChannelWait, mpsc::Sender<Event>) {
            let (tx, rx) = mpsc::channel();
            (ChannelWait::new(rx), tx)
        }

        fn now_plus(ms: i64) -> (Option<Timestamp>, Timestamp) {
            let now = Timestamp::now();
            let deadline = now + jiff::SignedDuration::from_millis(ms);
            (Some(deadline), now)
        }

        #[test]
        fn past_deadline_ends_at_once() {
            let (mut wait, _tx) = channel();
            let (deadline, now) = now_plus(-10);
            let start = Instant::now();
            assert!(matches!(wait.wait(deadline, now), Wake::Deadline));
            assert!(start.elapsed() < Duration::from_millis(20));
        }

        #[test]
        fn deadline_ends_the_wait_after_its_time() {
            let (mut wait, _tx) = channel();
            let (deadline, now) = now_plus(50);
            let start = Instant::now();
            assert!(matches!(wait.wait(deadline, now), Wake::Deadline));
            assert!(
                start.elapsed() >= Duration::from_millis(50),
                "{:?}",
                start.elapsed()
            );
        }

        #[test]
        fn a_shutdown_event_is_a_shutdown() {
            let (mut wait, tx) = channel();
            tx.send(Event::Shutdown).unwrap();
            let (deadline, now) = now_plus(5_000);
            assert!(matches!(wait.wait(deadline, now), Wake::Shutdown));
        }

        #[test]
        fn a_request_is_a_command() {
            let (mut wait, tx) = channel();
            let (reply, _rx) = mpsc::channel();
            let request = Request {
                command: Command::Status,
                reply,
            };
            tx.send(Event::Command(request)).unwrap();
            match wait.wait(None, Timestamp::now()) {
                Wake::Command(r) => assert_eq!(r.command, Command::Status),
                other => panic!("expected a command, got {other:?}"),
            }
        }

        #[test]
        fn closed_channel_is_a_shutdown() {
            let (mut wait, tx) = channel();
            drop(tx);
            assert!(matches!(wait.wait(None, Timestamp::now()), Wake::Shutdown));
        }

        #[test]
        fn no_presents_while_waiting() {
            let (mut wait, tx) = channel();
            let mut panel = Recorder::new(64, 32);
            let clock = SystemClock::new(TimeZone::UTC);
            let sender = std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(500));
                tx.send(Event::Shutdown).unwrap();
            });
            let start = Instant::now();
            run(
                &mut panel,
                Box::new(Red),
                Mode::Clock,
                Tables::legacy(),
                no_reload,
                &clock,
                &mut wait,
            )
            .unwrap();
            assert!(
                start.elapsed() >= Duration::from_millis(500),
                "run must wait"
            );
            assert_eq!(panel.presents.len(), 1);
            sender.join().unwrap();
        }
    }

    mod time_zone {
        use super::*;

        #[test]
        fn named_zone_is_used_and_named() {
            let zone = TimeZone::get("America/Los_Angeles").unwrap();
            let (got, message) = local_zone(Ok(zone.clone()));
            assert_eq!(got, zone);
            assert_eq!(message, "gameclockd: the time zone is America/Los_Angeles.");
        }

        #[test]
        fn zone_with_no_name_is_used() {
            let zone = TimeZone::fixed(jiff::tz::offset(-8));
            let (got, message) = local_zone(Ok(zone.clone()));
            assert_eq!(got, zone);
            assert_eq!(
                message,
                "gameclockd: the time zone has no name. The daemon uses it."
            );
        }

        #[test]
        fn missing_time_zone_falls_back_to_utc() {
            let lookup = TimeZone::get("Not/A_Zone");
            assert!(lookup.is_err());
            let (got, message) = local_zone(lookup);
            assert_eq!(got, TimeZone::UTC);
            assert_eq!(
                message,
                "gameclockd: the system time zone is not available. The daemon uses UTC."
            );
        }
    }
}
