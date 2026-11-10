//! The in-memory frame buffer that everything renders into.

use core::convert::Infallible;

use embedded_graphics::pixelcolor::Rgb888;
use embedded_graphics::prelude::*;

/// A row-major RGB frame buffer.
///
/// Implements [`DrawTarget`], so any `embedded-graphics` primitive, font, or
/// image can be drawn onto it. Drawing outside the bounds is silently clipped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    width: usize,
    height: usize,
    pixels: Box<[Rgb888]>,
}

impl Frame {
    /// A new frame of `width` × `height` pixels, all black.
    #[must_use]
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            pixels: vec![Rgb888::BLACK; width * height].into_boxed_slice(),
        }
    }

    /// Width in pixels.
    #[must_use]
    pub fn width(&self) -> usize {
        self.width
    }

    /// Height in pixels.
    #[must_use]
    pub fn height(&self) -> usize {
        self.height
    }

    /// The color at `point`, or `None` if it is out of bounds.
    #[must_use]
    pub fn pixel(&self, point: Point) -> Option<Rgb888> {
        self.index(point).map(|i| self.pixels[i])
    }

    /// All pixels in row-major order; this is what a panel blits.
    #[must_use]
    pub fn pixels(&self) -> &[Rgb888] {
        &self.pixels
    }

    fn index(&self, point: Point) -> Option<usize> {
        let x = usize::try_from(point.x).ok()?;
        let y = usize::try_from(point.y).ok()?;
        (x < self.width && y < self.height).then(|| y * self.width + x)
    }
}

impl OriginDimensions for Frame {
    fn size(&self) -> Size {
        // Frames are panel-sized (tens of pixels); saturate rather than wrap.
        let w = u32::try_from(self.width).unwrap_or(u32::MAX);
        let h = u32::try_from(self.height).unwrap_or(u32::MAX);
        Size::new(w, h)
    }
}

impl DrawTarget for Frame {
    type Color = Rgb888;
    type Error = Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        for Pixel(point, color) in pixels {
            if let Some(i) = self.index(point) {
                self.pixels[i] = color;
            }
        }
        Ok(())
    }

    fn clear(&mut self, color: Self::Color) -> Result<(), Self::Error> {
        self.pixels.fill(color);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use embedded_graphics::mono_font::{MonoTextStyle, ascii::FONT_6X10};
    use embedded_graphics::primitives::{PrimitiveStyle, Rectangle};
    use embedded_graphics::text::Text;

    #[test]
    fn new_frame_is_black_and_sized() {
        let f = Frame::new(64, 32);
        assert_eq!(f.size(), Size::new(64, 32));
        assert_eq!(f.pixels().len(), 64 * 32);
        assert!(f.pixels().iter().all(|&p| p == Rgb888::BLACK));
    }

    #[test]
    fn draws_primitives_and_clips_out_of_bounds() {
        let mut f = Frame::new(4, 4);
        Rectangle::new(Point::new(2, 2), Size::new(5, 5))
            .into_styled(PrimitiveStyle::with_fill(Rgb888::RED))
            .draw(&mut f)
            .unwrap();
        assert_eq!(f.pixel(Point::new(3, 3)), Some(Rgb888::RED));
        assert_eq!(f.pixel(Point::new(1, 1)), Some(Rgb888::BLACK));
        assert_eq!(f.pixel(Point::new(4, 4)), None);
        assert_eq!(f.pixel(Point::new(-1, 0)), None);
    }

    #[test]
    fn renders_text_without_hardware() {
        let mut f = Frame::new(64, 32);
        Text::new(
            "12:00",
            Point::new(0, 10),
            MonoTextStyle::new(&FONT_6X10, Rgb888::BLUE),
        )
        .draw(&mut f)
        .unwrap();
        assert!(f.pixels().contains(&Rgb888::BLUE));
    }

    #[test]
    fn clear_fills_every_pixel() {
        let mut f = Frame::new(8, 8);
        f.clear(Rgb888::GREEN).unwrap();
        assert!(f.pixels().iter().all(|&p| p == Rgb888::GREEN));
    }
}
