//! Converts a [`Frame`] into the pixel buffer that hzeller copies to its
//! canvas.
//!
//! This code is pure, so its tests run on any host. The panel calls it at
//! each present with a preallocated buffer. There is no allocation for each
//! frame (docs/adr/0006).

use gameclock_core::embedded_graphics::pixelcolor::RgbColor;
use gameclock_core::{Brightness, Frame};

use crate::Error;

/// One pixel as hzeller's C API expects it (`struct Color` in
/// `led-matrix-c.h`).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Color {
    /// Red.
    pub r: u8,
    /// Green.
    pub g: u8,
    /// Blue.
    pub b: u8,
}

/// Fill `buf` with `frame`'s pixels for a panel of `panel` `(width, height)`.
///
/// Returns the driver brightness to apply (1–100). For brightness 0, it returns
/// `None` and `buf` is all black. Do not change the driver brightness in that
/// case, because hzeller's range starts at 1.
///
/// # Errors
/// [`Error::DimensionMismatch`] if the frame is not panel-sized. `buf` stays
/// unchanged in that case.
///
/// # Panics
/// If `buf.len()` is not the panel's `width × height`. The panel allocates
/// the buffer once at that size, so this is a programming error.
pub fn prepare(
    frame: &Frame,
    panel: (usize, usize),
    brightness: Brightness,
    buf: &mut [Color],
) -> Result<Option<u8>, Error> {
    let size = (frame.width(), frame.height());
    if size != panel {
        return Err(Error::DimensionMismatch { frame: size, panel });
    }
    assert_eq!(buf.len(), panel.0 * panel.1, "buffer sized for the panel");

    let percent = brightness.percent();
    if percent == 0 {
        buf.fill(Color::default());
        return Ok(None);
    }
    for (dst, src) in buf.iter_mut().zip(frame.pixels()) {
        *dst = Color {
            r: src.r(),
            g: src.g(),
            b: src.b(),
        };
    }
    Ok(Some(percent))
}

#[cfg(test)]
mod tests {
    use super::*;
    use gameclock_core::embedded_graphics::pixelcolor::Rgb888;
    use gameclock_core::embedded_graphics::prelude::*;

    const PANEL: (usize, usize) = (64, 32);

    fn buffer() -> Vec<Color> {
        vec![Color::default(); PANEL.0 * PANEL.1]
    }

    /// Pixel (x, y) is (x·4, y·8, 7), so each position has a different color.
    fn gradient() -> Frame {
        let mut f = Frame::new(PANEL.0, PANEL.1);
        for y in 0..32u8 {
            for x in 0..64u8 {
                Pixel(
                    Point::new(i32::from(x), i32::from(y)),
                    Rgb888::new(x * 4, y * 8, 7),
                )
                .draw(&mut f)
                .unwrap();
            }
        }
        f
    }

    #[test]
    fn pixels_are_converted_in_row_major_order() {
        let mut buf = buffer();
        prepare(&gradient(), PANEL, Brightness::MAX, &mut buf).unwrap();
        assert_eq!(buf.len(), 2048);
        for y in 0..32u8 {
            for x in 0..64u8 {
                let i = usize::from(y) * 64 + usize::from(x);
                assert_eq!(
                    buf[i],
                    Color {
                        r: x * 4,
                        g: y * 8,
                        b: 7
                    },
                    "pixel ({x}, {y})"
                );
            }
        }
    }

    #[test]
    fn brightness_maps_to_the_driver_range() {
        let mut buf = buffer();
        for pct in [1, 40, 100] {
            let driver = prepare(&gradient(), PANEL, Brightness::new(pct), &mut buf).unwrap();
            assert_eq!(driver, Some(pct));
        }
    }

    #[test]
    fn zero_brightness_blanks_the_buffer() {
        let mut buf = vec![Color { r: 1, g: 2, b: 3 }; PANEL.0 * PANEL.1];
        let driver = prepare(&gradient(), PANEL, Brightness::new(0), &mut buf).unwrap();
        assert_eq!(driver, None);
        assert!(buf.iter().all(|&c| c == Color::default()));
    }

    #[test]
    fn wrong_sized_frame_is_rejected_without_writing() {
        let sentinel = Color { r: 9, g: 9, b: 9 };
        let mut buf = vec![sentinel; PANEL.0 * PANEL.1];
        let err = prepare(&Frame::new(32, 16), PANEL, Brightness::MAX, &mut buf).unwrap_err();
        assert_eq!(
            err,
            Error::DimensionMismatch {
                frame: (32, 16),
                panel: (64, 32)
            }
        );
        assert!(buf.iter().all(|&c| c == sentinel));
    }
}
