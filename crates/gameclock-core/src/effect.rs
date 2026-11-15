//! Background effects that alerts share (`openspec/specs/alerts`,
//! docs/adr/0018).

use embedded_graphics::pixelcolor::{Rgb888, RgbColor};
use jiff::SignedDuration;

/// The time for each step of the color wheel.
pub const WHEEL_STEP: SignedDuration = SignedDuration::from_millis(50);

/// The number of wheel positions.
const WHEEL_SIZE: u16 = 765;

/// The wheel position after `steps` steps. It moves 3 positions in each step,
/// 60 positions each second as in the legacy clock.
#[must_use]
pub fn wheel_position(steps: u64) -> u16 {
    let position = steps.wrapping_mul(3) % u64::from(WHEEL_SIZE);
    // The value is less than 765.
    u16::try_from(position).unwrap_or(0)
}

/// The color at a wheel position from 0 to 764.
#[must_use]
pub fn wheel(position: u16) -> Rgb888 {
    // The three segments of the legacy wheel. Each value is 255 or less.
    let c = |v: u16| u8::try_from(v.min(255)).unwrap_or(u8::MAX);
    let position = position % WHEEL_SIZE;
    if position <= 255 {
        Rgb888::new(c(position), 0, c(255 - position))
    } else if position <= 511 {
        let v = position - 256;
        Rgb888::new(c(255 - v), c(v), 0)
    } else {
        let v = position - 512;
        Rgb888::new(0, c(255 - v), c(v))
    }
}

/// Black or white, whichever is easier to read on `background`.
#[must_use]
pub fn contrast(background: Rgb888) -> Rgb888 {
    let luminance = 299 * u32::from(background.r())
        + 587 * u32::from(background.g())
        + 114 * u32::from(background.b());
    if luminance > 145_000 {
        Rgb888::BLACK
    } else {
        Rgb888::WHITE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wheel_segments() {
        assert_eq!(wheel(0), Rgb888::new(0, 0, 255));
        assert_eq!(wheel(255), Rgb888::new(255, 0, 0));
        assert_eq!(wheel(256), Rgb888::new(255, 0, 0));
        assert_eq!(wheel(300), Rgb888::new(211, 44, 0));
        assert_eq!(wheel(501), Rgb888::new(10, 245, 0));
        assert_eq!(wheel(600), Rgb888::new(0, 167, 88));
        assert_eq!(wheel(764), Rgb888::new(0, 3, 252));
    }

    #[test]
    fn wheel_moves_3_positions_each_step() {
        assert_eq!(wheel_position(0), 0);
        assert_eq!(wheel_position(100), 300);
        assert_eq!(wheel_position(167), 501);
        assert_eq!(wheel_position(200), 600);
        assert_eq!(wheel_position(255), 0);
        assert_eq!(wheel_position(256), 3);
    }

    #[test]
    fn contrast_is_black_or_white() {
        let white = Rgb888::new(255, 255, 255);
        let black = Rgb888::new(0, 0, 0);
        assert_eq!(contrast(Rgb888::new(0, 0, 255)), white);
        assert_eq!(contrast(Rgb888::new(211, 44, 0)), white);
        assert_eq!(contrast(Rgb888::new(10, 245, 0)), black);
    }

    #[test]
    fn contrast_at_the_limit_is_white() {
        // 299 × 2 + 587 × 246 + 114 × 0 = 145,000. Black starts above it.
        assert_eq!(contrast(Rgb888::new(2, 246, 0)), Rgb888::new(255, 255, 255));
        assert_eq!(contrast(Rgb888::new(3, 246, 0)), Rgb888::new(0, 0, 0));
    }
}
