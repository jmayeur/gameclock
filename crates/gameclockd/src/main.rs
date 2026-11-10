//! `gameclockd`: the game clock daemon.
//!
//! Placeholder. The render loop, time source, command input, and panel driver
//! arrive through `OpenSpec` changes (see `docs/design/roadmap.md`).

use gameclock_core::Frame;

/// Panel size of the current hardware (docs/hardware/pi-zero-w.md).
const PANEL: (usize, usize) = (64, 32);

fn main() {
    let frame = Frame::new(PANEL.0, PANEL.1);
    println!(
        "gameclockd {}: {}x{} frame ready; no panel driver yet",
        env!("CARGO_PKG_VERSION"),
        frame.width(),
        frame.height()
    );
}
