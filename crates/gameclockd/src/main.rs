//! `gameclockd`: the game clock daemon.
//!
//! It runs the event loop with the clock, or with the heartbeat in the
//! test-pattern mode, until a stop signal (`openspec/specs/daemon-lifecycle`). The library has the behavior. This
//! file connects the arguments, the signals, the clock, and the panel that
//! the build selects.

#[cfg(all(feature = "hzeller", feature = "simulator"))]
compile_error!("the hzeller and simulator features cannot be used together");

use std::process::ExitCode;
use std::sync::mpsc;

use jiff::tz::TimeZone;
use signal_hook::consts::{SIGINT, SIGTERM};
use signal_hook::iterator::Signals;

use gameclockd::{ChannelWait, Event, SystemClock, exit_status, local_zone, parse_args, run};

/// Exit status for command-line usage errors.
const USAGE: u8 = 2;

fn main() -> ExitCode {
    let args = match parse_args(std::env::args().skip(1)) {
        Ok(args) => args,
        Err(e) => {
            eprintln!("gameclockd: {e}");
            return ExitCode::from(USAGE);
        }
    };

    // Register before the driver starts. Then a stop signal during
    // initialization is not lost (docs/adr/0010, docs/adr/0016).
    let (events, receiver) = mpsc::channel();
    if let Err(e) = spawn_signal_thread(events) {
        eprintln!("gameclockd: cannot register signal handlers: {e}");
        return ExitCode::FAILURE;
    }
    let mut wait = ChannelWait::new(receiver);
    // Read the time zone once. A change needs a restart (docs/adr/0013).
    let (zone, message) = local_zone(TimeZone::try_system());
    eprintln!("{message}");
    let clock = SystemClock::new(zone);
    eprintln!("{}", args.mode.announcement());

    let result = {
        let mut panel = match panel::create(&args.driver) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("gameclockd: {e}");
                return ExitCode::FAILURE;
            }
        };
        // Each wait blocks on the event channel, with a timeout until the
        // next scene change (docs/adr/0016).
        run(
            &mut panel,
            args.mode.base_scene(),
            args.mode,
            &clock,
            &mut wait,
        )
        // The panel drops here, before the report and the exit. The hzeller
        // driver stops its refresh thread and blanks the LEDs.
    };

    if let Err(e) = &result {
        eprintln!("gameclockd: {e}");
    }
    ExitCode::from(exit_status(&result))
}

/// The stack size of each helper thread. They only block and send events.
const THREAD_STACK: usize = 64 * 1024;

/// Register SIGTERM and SIGINT. Start a thread that sends one shutdown event
/// when the first of them arrives.
fn spawn_signal_thread(events: mpsc::Sender<Event>) -> std::io::Result<()> {
    let mut signals = Signals::new([SIGTERM, SIGINT])?;
    std::thread::Builder::new()
        .name("signals".into())
        .stack_size(THREAD_STACK)
        .spawn(move || {
            if signals.forever().next().is_some() {
                // The loop can be gone already. Then there is nothing to stop.
                let _ = events.send(Event::Shutdown);
            }
        })?;
    Ok(())
}

/// The real LED matrix (docs/adr/0009).
#[cfg(feature = "hzeller")]
mod panel {
    pub fn create(args: &[String]) -> Result<rgbmatrix_sys::HzellerPanel, rgbmatrix_sys::Error> {
        rgbmatrix_sys::HzellerPanel::new(args.iter().cloned())
    }
}

/// No LED driver is built in. Draw the frames in the terminal
/// (`openspec/specs/terminal-simulator`).
#[cfg(all(feature = "simulator", not(feature = "hzeller")))]
mod panel {
    use core::convert::Infallible;
    use std::io::StdoutLock;

    use gameclockd::sim::TerminalPanel;

    /// Panel size of the current hardware (docs/hardware/pi-zero-w.md).
    const SIZE: (usize, usize) = (64, 32);

    #[allow(clippy::unnecessary_wraps)] // Same signature as the hzeller build.
    pub fn create(args: &[String]) -> Result<TerminalPanel<StdoutLock<'static>>, Infallible> {
        eprintln!("gameclockd: no LED driver is built in. Frames are drawn in the terminal.");
        if !args.is_empty() {
            eprintln!("gameclockd: --led-* flags are ignored without the driver");
        }
        Ok(TerminalPanel::new(std::io::stdout().lock(), SIZE.0, SIZE.1))
    }
}

/// No LED driver is built in. Log the frames, so the daemon runs anywhere.
#[cfg(not(any(feature = "hzeller", feature = "simulator")))]
mod panel {
    use core::convert::Infallible;

    use gameclockd::LogPanel;

    /// Panel size of the current hardware (docs/hardware/pi-zero-w.md).
    const SIZE: (usize, usize) = (64, 32);

    #[allow(clippy::unnecessary_wraps)] // Same signature as the hzeller build.
    pub fn create(args: &[String]) -> Result<LogPanel, Infallible> {
        eprintln!("gameclockd: no LED driver is built in. Frames are only logged.");
        if !args.is_empty() {
            eprintln!("gameclockd: --led-* flags are ignored without the driver");
        }
        Ok(LogPanel::new(SIZE.0, SIZE.1))
    }
}
