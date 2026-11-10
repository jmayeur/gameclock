//! Drive hzeller's `rpi-rgb-led-matrix` from gameclock.
//!
//! This is the only crate in the workspace that allows `unsafe`
//! (docs/adr/0009). Every `unsafe` block carries a `// SAFETY:` comment.
//!
//! The pure parts (driver defaults and frame conversion) always compile and
//! are tested on any host. The FFI and the hardware panel exist only with the
//! `link` feature, which builds and links the vendored C++ library.

#![allow(unsafe_code)] // docs/adr/0009: the single crate permitted unsafe.
#![deny(clippy::undocumented_unsafe_blocks)]

pub mod convert;
mod error;
#[cfg(feature = "link")]
mod ffi;
pub mod options;
#[cfg(feature = "link")]
mod panel;

pub use convert::Color;
pub use error::Error;
#[cfg(feature = "link")]
pub use panel::HzellerPanel;
