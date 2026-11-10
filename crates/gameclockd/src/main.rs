//! `gameclockd`: the game clock daemon.
//!
//! For now it shows the diagnostic test pattern and waits to be stopped
//! (`specs/daemon-lifecycle`). Behavior lives in the library; this file wires
//! up arguments, signals, and the panel chosen at build time.

use std::process::ExitCode;

use signal_hook::consts::{SIGINT, SIGTERM};
use signal_hook::iterator::Signals;

use gameclockd::{exit_status, parse_args, run};

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

    // Register before the driver starts, so a stop during init isn't lost
    // (docs/adr/0010).
    let mut signals = match Signals::new([SIGTERM, SIGINT]) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("gameclockd: cannot register signal handlers: {e}");
            return ExitCode::FAILURE;
        }
    };

    let result = {
        let mut panel = match panel::create(&driver_args) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("gameclockd: {e}");
                return ExitCode::FAILURE;
            }
        };
        // Blocks in a read on signal-hook's self-pipe: no timer, no extra
        // thread.
        run(&mut panel, || {
            let _ = signals.forever().next();
        })
        // The panel drops here, before we report and exit; the hzeller
        // driver stops its refresh thread and blanks the LEDs.
    };

    if let Err(e) = &result {
        eprintln!("gameclockd: {e}");
    }
    ExitCode::from(exit_status(&result))
}

/// The real LED matrix (docs/adr/0009).
#[cfg(feature = "hzeller")]
mod panel {
    pub fn create(args: &[String]) -> Result<rgbmatrix_sys::HzellerPanel, rgbmatrix_sys::Error> {
        rgbmatrix_sys::HzellerPanel::new(args.iter().cloned())
    }
}

/// No LED driver built in: log frames instead, so the daemon runs anywhere.
#[cfg(not(feature = "hzeller"))]
mod panel {
    use core::convert::Infallible;

    use gameclockd::LogPanel;

    /// Panel size of the current hardware (docs/hardware/pi-zero-w.md).
    const SIZE: (usize, usize) = (64, 32);

    #[allow(clippy::unnecessary_wraps)] // Same signature as the hzeller build.
    pub fn create(args: &[String]) -> Result<LogPanel, Infallible> {
        eprintln!("gameclockd: no LED driver is built in; frames are only logged");
        if !args.is_empty() {
            eprintln!("gameclockd: --led-* flags are ignored without the driver");
        }
        Ok(LogPanel::new(SIZE.0, SIZE.1))
    }
}
