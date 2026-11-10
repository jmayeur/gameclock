//! Hardware-free core of the game clock.
//!
//! All code here is pure. It renders into an in-memory [`Frame`]. It never
//! touches hardware, the filesystem, the system clock, or threads. The daemon
//! injects inputs such as the current time. So all rendering is deterministic
//! and testable on any machine (docs/adr/0005).
//!
//! A [`Panel`] is the only boundary to real output. It puts a finished frame
//! on an LED matrix, a simulator, or a test double.

#![forbid(unsafe_code)]

pub mod frame;
pub mod heartbeat;
pub mod panel;
pub mod scene;
pub mod schedule;
pub mod test_pattern;

/// Re-exported, so that other crates draw with the same version.
pub use embedded_graphics;

pub use frame::Frame;
pub use panel::{Brightness, Panel};
pub use scene::{Compositor, Scene};
