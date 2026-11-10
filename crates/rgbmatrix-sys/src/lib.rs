//! Drive hzeller's `rpi-rgb-led-matrix` from gameclock.
//!
//! This is the only crate in the workspace that permits `unsafe`
//! (docs/adr/0009). Each `unsafe` block has a `// SAFETY:` comment.
//!
//! The pure parts (driver defaults and frame conversion) always compile. Their
//! tests run on any host. The FFI and the hardware panel exist only with the
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
