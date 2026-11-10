//! The boundary between rendered frames and physical (or simulated) output.

use crate::Frame;

/// Panel brightness as a percentage, clamped to `0..=100`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Brightness(u8);

impl Brightness {
    /// Full brightness.
    pub const MAX: Self = Self(100);

    /// A brightness of `percent`, clamped to 100.
    #[must_use]
    pub const fn new(percent: u8) -> Self {
        Self(if percent > 100 { 100 } else { percent })
    }

    /// The brightness as a percentage.
    #[must_use]
    pub const fn percent(self) -> u8 {
        self.0
    }
}

impl Default for Brightness {
    fn default() -> Self {
        Self::MAX
    }
}

/// A thing that can show a [`Frame`]: the LED matrix on the Pi (through
/// hzeller), a desktop simulator, or a test double.
///
/// Implementations only blit pixels. They have no rendering logic, so a new
/// driver never changes scenes (docs/adr/0005).
pub trait Panel {
    /// Driver-specific failure.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Panel dimensions as `(width, height)` in pixels.
    fn dimensions(&self) -> (usize, usize);

    /// Show `frame` at `brightness`. Called only when the frame changed.
    ///
    /// # Errors
    /// Returns the driver's error if the panel cannot show the frame.
    fn present(&mut self, frame: &Frame, brightness: Brightness) -> Result<(), Self::Error>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn brightness_clamps_to_100() {
        assert_eq!(Brightness::new(250).percent(), 100);
        assert_eq!(Brightness::new(40).percent(), 40);
        assert_eq!(Brightness::default(), Brightness::MAX);
    }
}
