//! [`HzellerPanel`]: a [`Panel`] that shows frames on the LED matrix.
//!
//! The pure work (size check, pixel conversion, brightness) is in
//! [`crate::convert`] and tested on the host. This module only moves the
//! result across the FFI; it is verified on the device
//! (`specs/hzeller-panel`, on-device scenarios).

use core::ffi::{c_char, c_int};
use core::fmt;
use core::ptr::NonNull;
use std::ffi::CString;

use gameclock_core::{Brightness, Frame, Panel};

use crate::Error;
use crate::convert::{self, Color};
use crate::ffi;
use crate::options::DEFAULTS;

/// The LED matrix, driven by hzeller's library.
///
/// Creating it starts the driver's refresh thread; dropping it stops the
/// thread and blanks the panel.
pub struct HzellerPanel {
    matrix: NonNull<ffi::RGBLedMatrix>,
    /// The canvas we draw into; shown on the next swap.
    offscreen: NonNull<ffi::LedCanvas>,
    size: (usize, usize),
    /// Preallocated so `present` never allocates (docs/adr/0006).
    buf: Box<[Color]>,
    /// hzeller's flag parser stores pointers into these strings in the
    /// matrix's options, so they must live as long as the matrix.
    _args: Vec<CString>,
}

impl HzellerPanel {
    /// Start the driver with `--led-*` `args` applied over this Pi's defaults
    /// ([`DEFAULTS`]).
    ///
    /// # Errors
    /// [`Error::Init`] if hzeller rejects a flag or can't initialize the
    /// hardware (it prints the reason to stderr; GPIO needs root).
    /// [`Error::UnknownArgs`] for arguments hzeller didn't consume.
    pub fn new<I, S>(args: I) -> Result<Self, Error>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        ffi::retain_sections();
        let mut owned = vec![c"gameclockd".to_owned()];
        for arg in args {
            let arg = arg.into();
            let c = CString::new(arg.clone()).map_err(|_| Error::UnknownArgs(vec![arg]))?;
            owned.push(c);
        }
        let mut arg_ptrs: Vec<*mut c_char> = owned.iter().map(|s| s.as_ptr().cast_mut()).collect();
        arg_ptrs.push(core::ptr::null_mut());
        let mut arg_count = c_int::try_from(owned.len()).map_err(|_| Error::Init)?;
        let mut arg_array = arg_ptrs.as_mut_ptr();

        let mut options = ffi::RGBLedMatrixOptions::zeroed();
        options.cols = DEFAULTS.cols;
        options.rows = DEFAULTS.rows;
        options.chain_length = DEFAULTS.chain_length;
        options.parallel = DEFAULTS.parallel;
        options.hardware_mapping = DEFAULTS.hardware_mapping.as_ptr();

        // SAFETY: `options` is a valid, initialized struct whose layout is
        // checked against the header at build time (layout_check.c, ffi.rs).
        // `arg_count`/`arg_array` describe `arg_ptrs`: `arg_count` non-null C strings plus a
        // terminating null, all owned by `owned`, which outlives the matrix
        // (moved into `_args`). hzeller only reorders the pointer array and
        // keeps pointers into the strings, which never move (CString heap).
        let matrix = unsafe {
            ffi::led_matrix_create_from_options(
                &raw mut options,
                &raw mut arg_count,
                &raw mut arg_array,
            )
        };
        let matrix = NonNull::new(matrix).ok_or(Error::Init)?;

        // Wrap immediately so every early return below deletes the matrix.
        let mut panel = Self {
            matrix,
            offscreen: NonNull::dangling(),
            size: (0, 0),
            buf: Box::default(),
            _args: owned,
        };

        // hzeller moved the flags it consumed out of argv[1..arg_count]; anything
        // left is not a flag it knows.
        let remaining = usize::try_from(arg_count).unwrap_or(0);
        if remaining > 1 {
            // SAFETY: arg_array[1..arg_count] are pointers into `panel._args`,
            // which are alive and NUL-terminated.
            let leftovers = (1..remaining)
                .map(|i| unsafe { std::ffi::CStr::from_ptr(*arg_array.add(i)) })
                .map(|s| s.to_string_lossy().into_owned())
                .collect();
            return Err(Error::UnknownArgs(leftovers));
        }

        // SAFETY: `panel.matrix` is a live matrix from hzeller; the canvas it
        // returns is owned by the matrix and freed with it.
        let canvas = unsafe { ffi::led_matrix_create_offscreen_canvas(panel.matrix.as_ptr()) };
        panel.offscreen = NonNull::new(canvas).ok_or(Error::Init)?;

        let (mut w, mut h): (c_int, c_int) = (0, 0);
        // SAFETY: `offscreen` is a live canvas; `w` and `h` are valid out
        // pointers.
        unsafe { ffi::led_canvas_get_size(panel.offscreen.as_ptr(), &raw mut w, &raw mut h) };
        let w = usize::try_from(w).map_err(|_| Error::Init)?;
        let h = usize::try_from(h).map_err(|_| Error::Init)?;
        panel.size = (w, h);
        panel.buf = vec![Color::default(); w * h].into_boxed_slice();
        Ok(panel)
    }
}

impl Panel for HzellerPanel {
    type Error = Error;

    fn dimensions(&self) -> (usize, usize) {
        self.size
    }

    fn present(&mut self, frame: &Frame, brightness: Brightness) -> Result<(), Error> {
        let driver_brightness = convert::prepare(frame, self.size, brightness, &mut self.buf)?;
        // `prepare` checked the frame is exactly `size`, which came from
        // c_int values, so these conversions can't fail.
        let w = c_int::try_from(self.size.0).map_err(|_| Error::Init)?;
        let h = c_int::try_from(self.size.1).map_err(|_| Error::Init)?;

        if let Some(b) = driver_brightness {
            // SAFETY: `matrix` is live for as long as `self`. hzeller applies
            // brightness when pixels are set, so this precedes set_pixels.
            unsafe { ffi::led_matrix_set_brightness(self.matrix.as_ptr(), b) };
        }
        // SAFETY: `offscreen` is a live canvas of `w × h` pixels, and `buf`
        // holds exactly `w × h` `Color`s (allocated from the same size, and
        // `Color` is `repr(C)` with its size checked against the header).
        unsafe {
            ffi::led_canvas_set_pixels(self.offscreen.as_ptr(), 0, 0, w, h, self.buf.as_mut_ptr());
        }
        // SAFETY: `matrix` is live and `offscreen` is one of its canvases.
        // The returned canvas is the previously shown one, also owned by the
        // matrix, and becomes our next drawing target.
        let previous =
            unsafe { ffi::led_matrix_swap_on_vsync(self.matrix.as_ptr(), self.offscreen.as_ptr()) };
        self.offscreen = NonNull::new(previous).ok_or(Error::Init)?;
        Ok(())
    }
}

impl Drop for HzellerPanel {
    fn drop(&mut self) {
        // SAFETY: `matrix` came from `led_matrix_create_from_options` and is
        // deleted exactly once, here. It stops the refresh thread, blanks the
        // panel, and frees its canvases; nothing uses them afterwards.
        unsafe { ffi::led_matrix_delete(self.matrix.as_ptr()) };
    }
}

impl fmt::Debug for HzellerPanel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HzellerPanel")
            .field("size", &self.size)
            .finish_non_exhaustive()
    }
}
