//! The behavior of `gameclockd`. It is not in `main`, so tests do not need
//! hardware or signals (`openspec/specs/daemon-lifecycle`).

use core::fmt;
use std::io::{self, Read};
use std::os::unix::net::UnixStream;
use std::time::Duration;

#[cfg(feature = "simulator")]
pub mod sim;

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

/// Why a wait ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Wake {
    /// The deadline came, or the wait ended early. Render again.
    Deadline,
    /// A stop signal came. Stop the daemon.
    Shutdown,
}

/// Blocks until a deadline or a stop signal.
pub trait Wait {
    /// Block until `deadline` or a stop signal. `now` is the time when the
    /// wait starts. With no deadline, block until a stop signal.
    fn wait(&mut self, deadline: Option<Timestamp>, now: Timestamp) -> Wake;
}

/// Waits on the read end of the signal pipe (docs/adr/0013).
///
/// `main` registers SIGTERM and SIGINT to write a byte to the other end. A
/// byte means a stop signal. The read timeout ends the wait at the deadline.
/// A signal that comes during a render stays in the pipe, so the next wait
/// ends at once.
#[derive(Debug)]
pub struct SignalWait {
    pipe: UnixStream,
}

impl SignalWait {
    /// A wait on `pipe`, the read end of the signal pipe.
    #[must_use]
    pub fn new(pipe: UnixStream) -> Self {
        Self { pipe }
    }
}

impl Wait for SignalWait {
    fn wait(&mut self, deadline: Option<Timestamp>, now: Timestamp) -> Wake {
        let timeout = match deadline {
            None => None,
            Some(deadline) => match Duration::try_from(deadline.duration_since(now)) {
                Ok(d) if !d.is_zero() => Some(d),
                // The deadline is now or in the past.
                _ => return Wake::Deadline,
            },
        };
        if let Err(e) = self.pipe.set_read_timeout(timeout) {
            eprintln!("gameclockd: the wait failed: {e}. The daemon stops.");
            return Wake::Shutdown;
        }
        let mut byte = [0_u8; 1];
        match self.pipe.read(&mut byte) {
            // A byte is a stop signal. End of file means that the signal
            // handlers are gone, so stop too. Do not loop on it.
            Ok(_) => Wake::Shutdown,
            Err(e)
                if matches!(
                    e.kind(),
                    io::ErrorKind::WouldBlock
                        | io::ErrorKind::TimedOut
                        | io::ErrorKind::Interrupted
                ) =>
            {
                Wake::Deadline
            }
            Err(e) => {
                eprintln!("gameclockd: the wait failed: {e}. The daemon stops.");
                Wake::Shutdown
            }
        }
    }
}

/// Run the event loop until a stop signal (docs/adr/0006).
///
/// The loop renders `base` for the current time and presents each changed
/// frame on `panel` at full brightness. Then it waits until the next scene
/// change. It does not wake at other times.
///
/// # Errors
/// [`RunError::Present`] if the panel fails to show a frame.
pub fn run<P: Panel>(
    panel: &mut P,
    base: Box<dyn Scene>,
    clock: &impl Clock,
    wait: &mut impl Wait,
) -> Result<(), RunError<P::Error>> {
    let (width, height) = panel.dimensions();
    let mut compositor = Compositor::new(base, width, height);
    loop {
        let now = clock.now();
        if let Some(frame) = compositor.update(&now) {
            panel
                .present(frame, Brightness::MAX)
                .map_err(RunError::Present)?;
        }
        match wait.wait(compositor.deadline(&now), now.timestamp()) {
            Wake::Deadline => {}
            Wake::Shutdown => return Ok(()),
        }
    }
}

/// An argument that isn't an hzeller `--led-*` driver flag.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UsageError(pub String);

impl fmt::Display for UsageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "unknown argument '{}'. gameclockd accepts only --led-* driver flags.",
            self.0
        )
    }
}

impl std::error::Error for UsageError {}

/// Check the command line. Only `--led-*` driver flags are accepted. The
/// function returns them unchanged for the driver to parse
/// (`openspec/specs/daemon-lifecycle`, docs/adr/0009).
///
/// # Errors
/// [`UsageError`] naming the first argument that isn't a `--led-*` flag.
pub fn parse_args<I>(args: I) -> Result<Vec<String>, UsageError>
where
    I: IntoIterator<Item = String>,
{
    args.into_iter()
        .map(|arg| {
            if arg.starts_with("--led-") {
                Ok(arg)
            } else {
                Err(UsageError(arg))
            }
        })
        .collect()
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
    use std::rc::Rc;

    use gameclock_core::embedded_graphics::pixelcolor::Rgb888;
    use gameclock_core::embedded_graphics::prelude::*;
    use gameclock_core::heartbeat::Heartbeat;
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
    #[derive(Debug, Clone, Copy)]
    enum Step {
        /// The time moves to the deadline, and the wait ends.
        Deadline,
        /// The time moves to this time, and the wait ends before the deadline.
        EarlyAt(&'static str),
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
            }
        }
    }

    fn ts(s: &str) -> Timestamp {
        s.parse().unwrap()
    }

    const START: &str = "1970-01-01T00:00:00.250Z";

    fn script(start: &str, steps: &[Step]) -> (FakeClock, ScriptedWait) {
        let now = Rc::new(Cell::new(ts(start)));
        let wait = ScriptedWait {
            now: Rc::clone(&now),
            steps: steps.iter().copied().collect(),
            deadlines: Vec::new(),
        };
        (FakeClock(now), wait)
    }

    fn run_heartbeat<P: Panel>(
        panel: &mut P,
        steps: &[Step],
    ) -> (Result<(), RunError<P::Error>>, ScriptedWait) {
        let (clock, mut wait) = script(START, steps);
        let result = run(panel, Box::new(Heartbeat), &clock, &mut wait);
        (result, wait)
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
        assert_eq!(parse_args(given.clone()), Ok(given));
    }

    #[test]
    fn no_arguments_is_fine() {
        assert_eq!(parse_args(Vec::new()), Ok(Vec::new()));
    }

    #[test]
    fn unknown_argument_is_a_usage_error_naming_it() {
        let err = parse_args(args(&["--led-rows=32", "--frobnicate"])).unwrap_err();
        assert_eq!(err, UsageError("--frobnicate".into()));
        assert!(err.to_string().contains("--frobnicate"), "{err}");
    }

    #[test]
    fn bare_words_are_usage_errors() {
        assert!(parse_args(args(&["led-rows=32"])).is_err());
    }

    #[test]
    fn one_present_at_startup() {
        let mut panel = Recorder::new(64, 32);
        let (result, _) = run_heartbeat(&mut panel, &[]);
        result.unwrap();
        assert_eq!(panel.presents.len(), 1);
        let (frame, brightness) = &panel.presents[0];
        assert_eq!(*frame, heartbeat_frame(64, 32, START));
        assert_eq!(brightness.percent(), 100);
    }

    #[test]
    fn frame_follows_the_panels_size() {
        let mut panel = Recorder::new(32, 16);
        run_heartbeat(&mut panel, &[]).0.unwrap();
        assert_eq!(panel.presents.len(), 1);
        assert_eq!(panel.presents[0].0, heartbeat_frame(32, 16, START));
    }

    #[test]
    fn wait_ends_at_the_scene_deadline() {
        let (_, wait) = run_heartbeat(&mut Recorder::new(64, 32), &[]);
        assert_eq!(wait.deadlines[0], Some(ts("1970-01-01T00:00:01Z")));
    }

    #[test]
    fn static_scene_waits_with_no_timeout() {
        let mut panel = Recorder::new(64, 32);
        let (clock, mut wait) = script(START, &[]);
        run(&mut panel, Box::new(Red), &clock, &mut wait).unwrap();
        assert_eq!(wait.deadlines, vec![None]);
        assert_eq!(panel.presents.len(), 1);
    }

    #[test]
    fn heartbeat_presents_each_second() {
        let mut panel = Recorder::new(64, 32);
        let steps = [Step::Deadline, Step::Deadline, Step::Deadline];
        run_heartbeat(&mut panel, &steps).0.unwrap();
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
        let (result, wait) = run_heartbeat(&mut panel, &steps);
        result.unwrap();
        assert_eq!(panel.presents.len(), 1);
        let second = ts("1970-01-01T00:00:01Z");
        assert_eq!(wait.deadlines, vec![Some(second), Some(second)]);
    }

    #[test]
    fn messages_are_exact() {
        let err = run_heartbeat(&mut Broken, &[]).0.unwrap_err();
        assert_eq!(
            err.to_string(),
            "the panel did not show the frame: the LEDs are on fire"
        );
        assert_eq!(
            UsageError("--frobnicate".into()).to_string(),
            "unknown argument '--frobnicate'. gameclockd accepts only --led-* driver flags."
        );
    }

    #[test]
    fn present_failure_is_reported_and_exits_non_zero() {
        let (result, _) = run_heartbeat(&mut Broken, &[]);
        let err = result.as_ref().unwrap_err();
        assert!(
            err.to_string().contains("the LEDs are on fire"),
            "error was: {err}"
        );
        assert_ne!(exit_status(&result), 0);
    }

    #[test]
    fn success_exits_zero() {
        let (result, _) = run_heartbeat(&mut Recorder::new(64, 32), &[]);
        assert_eq!(exit_status(&result), 0);
    }

    mod signal_wait {
        use super::*;
        use std::io::Write;
        use std::os::unix::net::UnixStream;
        use std::time::{Duration, Instant};

        fn pair() -> (SignalWait, UnixStream) {
            let (read, write) = UnixStream::pair().unwrap();
            (SignalWait::new(read), write)
        }

        fn now_plus(ms: i64) -> (Option<Timestamp>, Timestamp) {
            let now = Timestamp::now();
            let deadline = now + jiff::SignedDuration::from_millis(ms);
            (Some(deadline), now)
        }

        #[test]
        fn past_deadline_ends_at_once() {
            let (mut wait, _write) = pair();
            let (deadline, now) = now_plus(-10);
            let start = Instant::now();
            assert_eq!(wait.wait(deadline, now), Wake::Deadline);
            assert!(start.elapsed() < Duration::from_millis(20));
        }

        #[test]
        fn deadline_ends_the_wait_after_its_time() {
            let (mut wait, _write) = pair();
            let (deadline, now) = now_plus(50);
            let start = Instant::now();
            assert_eq!(wait.wait(deadline, now), Wake::Deadline);
            assert!(
                start.elapsed() >= Duration::from_millis(50),
                "{:?}",
                start.elapsed()
            );
        }

        #[test]
        fn a_byte_in_the_pipe_is_a_shutdown() {
            let (mut wait, mut write) = pair();
            write.write_all(&[1]).unwrap();
            let (deadline, now) = now_plus(5_000);
            assert_eq!(wait.wait(deadline, now), Wake::Shutdown);
        }

        #[test]
        fn closed_pipe_is_a_shutdown() {
            let (mut wait, write) = pair();
            drop(write);
            assert_eq!(wait.wait(None, Timestamp::now()), Wake::Shutdown);
        }

        #[test]
        fn no_presents_while_waiting() {
            let (mut wait, mut write) = pair();
            let mut panel = Recorder::new(64, 32);
            let clock = SystemClock::new(TimeZone::UTC);
            let writer = std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(500));
                write.write_all(&[1]).unwrap();
            });
            let start = Instant::now();
            run(&mut panel, Box::new(Red), &clock, &mut wait).unwrap();
            assert!(
                start.elapsed() >= Duration::from_millis(500),
                "run must wait"
            );
            assert_eq!(panel.presents.len(), 1);
            writer.join().unwrap();
        }
    }
}
