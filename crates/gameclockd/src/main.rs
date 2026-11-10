//! `gameclockd`: the game clock daemon.
//!
//! It runs the event loop with the heartbeat scene until a stop signal
//! (`openspec/specs/daemon-lifecycle`). The library has the behavior. This
//! file connects the arguments, the signals, the clock, and the panel that
//! the build selects.

#[cfg(all(feature = "hzeller", feature = "simulator"))]
compile_error!("the hzeller and simulator features cannot be used together");

use std::os::unix::net::UnixStream;
use std::process::ExitCode;

use gameclock_core::heartbeat::Heartbeat;
use jiff::tz::TimeZone;
use signal_hook::consts::{SIGINT, SIGTERM};

use gameclockd::{SignalWait, SystemClock, exit_status, parse_args, run};

/// Exit status for command-line usage errors.
const USAGE: u8 = 2;

fn main() -> ExitCode {
    let driver_args = match parse_args(std::env::args().skip(1)) {
        Ok(args) => args,
        Err(e) => {
            eprintln!("gameclockd: {e}");
            return ExitCode::from(USAGE);
        }
    };

    // Register before the driver starts. Then a stop signal during
    // initialization is not lost (docs/adr/0010).
    let mut wait = match signal_pipe() {
        Ok(w) => w,
        Err(e) => {
            eprintln!("gameclockd: cannot register signal handlers: {e}");
            return ExitCode::FAILURE;
        }
    };
    let clock = SystemClock::new(TimeZone::UTC);

    let result = {
        let mut panel = match panel::create(&driver_args) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("gameclockd: {e}");
                return ExitCode::FAILURE;
            }
        };
        // Each wait blocks in a read on the signal pipe, with a timeout until
        // the next scene change. No extra thread (docs/adr/0013).
        run(&mut panel, Box::new(Heartbeat), &clock, &mut wait)
        // The panel drops here, before the report and the exit. The hzeller
        // driver stops its refresh thread and blanks the LEDs.
    };

    if let Err(e) = &result {
        eprintln!("gameclockd: {e}");
    }
    ExitCode::from(exit_status(&result))
}

/// Register SIGTERM and SIGINT to write a byte to a socket pair. Give the
/// wait on the read end.
fn signal_pipe() -> std::io::Result<SignalWait> {
    let (read, write) = UnixStream::pair()?;
    signal_hook::low_level::pipe::register(SIGTERM, write.try_clone()?)?;
    signal_hook::low_level::pipe::register(SIGINT, write)?;
    Ok(SignalWait::new(read))
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
