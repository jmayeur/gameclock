//! `gameclockd`: the game clock daemon.
//!
//! Now it shows the diagnostic test pattern and waits for a stop signal
//! (`openspec/specs/daemon-lifecycle`). The library has the behavior. This
//! file connects the arguments, the signals, and the panel that the build
//! selects.

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

    // Register before the driver starts. Then a stop signal during
    // initialization is not lost (docs/adr/0010).
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
        // Blocks in a read on the signal-hook self-pipe. No timer, no extra
        // thread.
        run(&mut panel, || {
            let _ = signals.forever().next();
        })
        // The panel drops here, before the report and the exit. The hzeller
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

/// No LED driver is built in. Log the frames, so the daemon runs anywhere.
#[cfg(not(feature = "hzeller"))]
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
