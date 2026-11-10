//! Hardware-free core of the game clock.
//!
//! Everything here is pure: it renders into an in-memory [`Frame`] and never
//! touches hardware, the filesystem, the system clock, or threads. Inputs such
//! as the current time are injected by the daemon. That keeps all display logic
//! deterministic and unit-testable on any machine (docs/adr/0005).
//!
//! A [`Panel`] is the only boundary to real output: it takes a finished frame
//! and puts it on an LED matrix, a simulator, or a test double.

#![forbid(unsafe_code)]

pub mod frame;
pub mod panel;
pub mod test_pattern;

/// Re-exported so downstream crates draw with the exact same version.
pub use embedded_graphics;

pub use frame::Frame;
pub use panel::{Brightness, Panel};
