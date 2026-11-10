//! Hand-written bindings to the parts of `include/led-matrix-c.h` that we
//! use, at the pinned commit (docs/adr/0009). Keep them in sync with the header
//! at each submodule update. The layout assertions below catch struct drift.

/// Expected layouts. `build.rs` generates them from the table that
/// `layout_check.c` also uses to check the C header.
mod layout {
    include!(concat!(env!("OUT_DIR"), "/layout.rs"));
}

use core::mem::{offset_of, size_of};

use crate::Color;

const _: () = assert!(size_of::<Color>() == layout::COLOR_SIZE);

const _: () = {
    type O = RGBLedMatrixOptions;
    assert!(size_of::<O>() == layout::OPTIONS_SIZE);
    assert!(offset_of!(O, hardware_mapping) == layout::OPTIONS_HARDWARE_MAPPING);
    assert!(offset_of!(O, rows) == layout::OPTIONS_ROWS);
    assert!(offset_of!(O, cols) == layout::OPTIONS_COLS);
    assert!(offset_of!(O, chain_length) == layout::OPTIONS_CHAIN_LENGTH);
    assert!(offset_of!(O, parallel) == layout::OPTIONS_PARALLEL);
    assert!(offset_of!(O, pwm_bits) == layout::OPTIONS_PWM_BITS);
    assert!(offset_of!(O, pwm_lsb_nanoseconds) == layout::OPTIONS_PWM_LSB_NANOSECONDS);
    assert!(offset_of!(O, pwm_dither_bits) == layout::OPTIONS_PWM_DITHER_BITS);
    assert!(offset_of!(O, brightness) == layout::OPTIONS_BRIGHTNESS);
    assert!(offset_of!(O, scan_mode) == layout::OPTIONS_SCAN_MODE);
    assert!(offset_of!(O, row_address_type) == layout::OPTIONS_ROW_ADDRESS_TYPE);
    assert!(offset_of!(O, multiplexing) == layout::OPTIONS_MULTIPLEXING);
    assert!(offset_of!(O, disable_hardware_pulsing) == layout::OPTIONS_DISABLE_HARDWARE_PULSING);
    assert!(offset_of!(O, show_refresh_rate) == layout::OPTIONS_SHOW_REFRESH_RATE);
    assert!(offset_of!(O, inverse_colors) == layout::OPTIONS_INVERSE_COLORS);
    assert!(offset_of!(O, led_rgb_sequence) == layout::OPTIONS_LED_RGB_SEQUENCE);
    assert!(offset_of!(O, pixel_mapper_config) == layout::OPTIONS_PIXEL_MAPPER_CONFIG);
    assert!(offset_of!(O, panel_type) == layout::OPTIONS_PANEL_TYPE);
    assert!(offset_of!(O, limit_refresh_rate_hz) == layout::OPTIONS_LIMIT_REFRESH_RATE_HZ);
    assert!(offset_of!(O, disable_busy_waiting) == layout::OPTIONS_DISABLE_BUSY_WAITING);
};

use core::ffi::{c_char, c_int};

/// `struct RGBLedMatrix`: opaque.
#[repr(C)]
pub struct RGBLedMatrix {
    _private: [u8; 0],
}

/// `struct LedCanvas`: opaque.
#[repr(C)]
pub struct LedCanvas {
    _private: [u8; 0],
}

/// `struct RGBLedMatrixOptions`. A zero field means "hzeller's default".
#[repr(C)]
#[derive(Debug)]
pub struct RGBLedMatrixOptions {
    pub hardware_mapping: *const c_char,
    pub rows: c_int,
    pub cols: c_int,
    pub chain_length: c_int,
    pub parallel: c_int,
    pub pwm_bits: c_int,
    pub pwm_lsb_nanoseconds: c_int,
    pub pwm_dither_bits: c_int,
    pub brightness: c_int,
    pub scan_mode: c_int,
    pub row_address_type: c_int,
    pub multiplexing: c_int,
    pub disable_hardware_pulsing: bool,
    pub show_refresh_rate: bool,
    pub inverse_colors: bool,
    pub led_rgb_sequence: *const c_char,
    pub pixel_mapper_config: *const c_char,
    pub panel_type: *const c_char,
    pub limit_refresh_rate_hz: c_int,
    pub disable_busy_waiting: bool,
}

impl RGBLedMatrixOptions {
    /// All fields zero or null: every option at hzeller's default.
    pub const fn zeroed() -> Self {
        Self {
            hardware_mapping: core::ptr::null(),
            rows: 0,
            cols: 0,
            chain_length: 0,
            parallel: 0,
            pwm_bits: 0,
            pwm_lsb_nanoseconds: 0,
            pwm_dither_bits: 0,
            brightness: 0,
            scan_mode: 0,
            row_address_type: 0,
            multiplexing: 0,
            disable_hardware_pulsing: false,
            show_refresh_rate: false,
            inverse_colors: false,
            led_rgb_sequence: core::ptr::null(),
            pixel_mapper_config: core::ptr::null(),
            panel_type: core::ptr::null(),
            limit_refresh_rate_hz: 0,
            disable_busy_waiting: false,
        }
    }
}

unsafe extern "C" {
    /// The RP1 (Pi 5) backend's entry in the `piochips` ELF section
    /// (`DECLARE_PIO_CHIP` in the vendored piolib). piolib reads that section
    /// through `__start_piochips` and `__stop_piochips`. LLD's `--gc-sections`
    /// does not count those as references. It removes the section, and the
    /// link fails. A reference to the entry keeps the section. See
    /// [`retain_sections`].
    static __ptr_rp1_pio_chip: *const core::ffi::c_void;

    /// Create a matrix from `options`. hzeller parses the `--led-*` flags in
    /// `argc`/`argv` over the options, and removes the flags that it used.
    /// Returns null on bad flags or failed hardware initialization. It prints
    /// the reason first.
    pub fn led_matrix_create_from_options(
        options: *mut RGBLedMatrixOptions,
        argc: *mut c_int,
        argv: *mut *mut *mut c_char,
    ) -> *mut RGBLedMatrix;

    /// Stop the refresh thread, blank the panel, and free the matrix.
    pub fn led_matrix_delete(matrix: *mut RGBLedMatrix);

    /// A new canvas for double-buffering, owned by the matrix.
    pub fn led_matrix_create_offscreen_canvas(matrix: *mut RGBLedMatrix) -> *mut LedCanvas;

    /// Show `canvas` at the next vsync. Returns the previous canvas for reuse.
    pub fn led_matrix_swap_on_vsync(
        matrix: *mut RGBLedMatrix,
        canvas: *mut LedCanvas,
    ) -> *mut LedCanvas;

    /// Brightness in percent (1–100) for the pixels set after this call.
    pub fn led_matrix_set_brightness(matrix: *mut RGBLedMatrix, brightness: u8);

    /// The canvas's size in pixels.
    pub fn led_canvas_get_size(canvas: *const LedCanvas, width: *mut c_int, height: *mut c_int);

    /// Copy `width × height` row-major `colors` to the canvas at `(x, y)`.
    pub fn led_canvas_set_pixels(
        canvas: *mut LedCanvas,
        x: c_int,
        y: c_int,
        width: c_int,
        height: c_int,
        colors: *mut Color,
    );
}

/// Keep the linker sections that the driver finds by name, not by symbol,
/// under `--gc-sections`. Call this function from code that is always linked.
#[inline(never)]
pub fn retain_sections() {
    // Taking the address does not read the static, so this is safe. The black
    // box stops the optimizer from removing the reference.
    core::hint::black_box(&raw const __ptr_rp1_pio_chip);
}
