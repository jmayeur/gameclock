//! Built-in driver defaults for this hardware (docs/hardware/pi-zero-w.md).
//!
//! Command-line `--led-*` flags are parsed by hzeller over these values, so a
//! flag always wins over a default (docs/adr/0009).

use core::ffi::{CStr, c_int};

/// The driver options gameclock sets before hzeller parses flags.
///
/// Anything not listed keeps hzeller's own default.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Defaults {
    /// Columns per panel (`--led-cols`).
    pub cols: c_int,
    /// Rows per panel (`--led-rows`).
    pub rows: c_int,
    /// Panels daisy-chained (`--led-chain`).
    pub chain_length: c_int,
    /// Parallel chains (`--led-parallel`).
    pub parallel: c_int,
    /// GPIO mapping name (`--led-gpio-mapping`).
    pub hardware_mapping: &'static CStr,
}

/// A single 64×32 panel on the Adafruit RGB Matrix HAT.
pub const DEFAULTS: Defaults = Defaults {
    cols: 64,
    rows: 32,
    chain_length: 1,
    parallel: 1,
    hardware_mapping: c"adafruit-hat",
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn built_in_defaults_describe_this_pi() {
        assert_eq!(DEFAULTS.cols, 64);
        assert_eq!(DEFAULTS.rows, 32);
        assert_eq!(DEFAULTS.chain_length, 1);
        assert_eq!(DEFAULTS.parallel, 1);
        assert_eq!(DEFAULTS.hardware_mapping, c"adafruit-hat");
    }
}
