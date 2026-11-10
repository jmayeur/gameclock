//! A diagnostic image that checks a panel's geometry and colors.
//!
//! - Corner markers show orientation and mirroring.
//! - The white border shows missing edge rows or columns.
//! - The color bars show channel order.
//! - The gray ramp shows color depth.
//!
//! The spec is `openspec/specs/test-pattern`.

use embedded_graphics::pixelcolor::Rgb888;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;

/// Size of the corner markers, and the inset of the content area from each
/// edge, in pixels.
const MARGIN: i32 = 3;

/// Bar colors, left to right.
const BARS: [Rgb888; 7] = [
    Rgb888::RED,
    Rgb888::GREEN,
    Rgb888::BLUE,
    Rgb888::CYAN,
    Rgb888::MAGENTA,
    Rgb888::new(255, 255, 0),
    Rgb888::WHITE,
];

/// Draw the test pattern over the whole of `target`.
///
/// The result depends only on the target's size. Any size works. Features
/// that do not fit are clipped.
///
/// # Errors
/// Returns the target's drawing error. Drawing into a [`crate::Frame`] never
/// fails.
pub fn render<D>(target: &mut D) -> Result<(), D::Error>
where
    D: DrawTarget<Color = Rgb888> + OriginDimensions,
{
    let size = target.size();
    let w = i32::try_from(size.width).unwrap_or(i32::MAX);
    let h = i32::try_from(size.height).unwrap_or(i32::MAX);

    target.clear(Rgb888::BLACK)?;
    content(target, w, h)?;

    // Border: top, bottom, left, right edges.
    fill(target, 0, 0, w, 1, Rgb888::WHITE)?;
    fill(target, 0, h - 1, w, 1, Rgb888::WHITE)?;
    fill(target, 0, 0, 1, h, Rgb888::WHITE)?;
    fill(target, w - 1, 0, 1, h, Rgb888::WHITE)?;

    // Corner markers last, so they sit on top of the border.
    fill(target, 0, 0, MARGIN, MARGIN, BARS[0])?;
    fill(target, w - MARGIN, 0, MARGIN, MARGIN, BARS[1])?;
    fill(target, 0, h - MARGIN, MARGIN, MARGIN, BARS[2])?;
    fill(target, w - MARGIN, h - MARGIN, MARGIN, MARGIN, BARS[5])
}

/// Color bars in the upper half of the content area, a gray ramp in the lower.
fn content<D>(target: &mut D, w: i32, h: i32) -> Result<(), D::Error>
where
    D: DrawTarget<Color = Rgb888>,
{
    let cw = w.saturating_sub(2 * MARGIN);
    let ch = h.saturating_sub(2 * MARGIN);
    if cw <= 0 || ch <= 0 {
        return Ok(());
    }
    let upper = ch / 2;
    let bars = i32::try_from(BARS.len()).unwrap_or(i32::MAX);

    for (i, &color) in (0..).zip(BARS.iter()) {
        let x0 = MARGIN + i * cw / bars;
        let x1 = MARGIN + (i + 1) * cw / bars;
        fill(target, x0, MARGIN, x1 - x0, upper, color)?;
    }

    for x in 0..cw {
        let level = if cw > 1 { 255 * x / (cw - 1) } else { 255 };
        let v = u8::try_from(level).unwrap_or(u8::MAX);
        fill(
            target,
            MARGIN + x,
            MARGIN + upper,
            1,
            ch - upper,
            Rgb888::new(v, v, v),
        )?;
    }
    Ok(())
}

/// Fill a rectangle. Empty or negative sizes draw nothing.
fn fill<D>(target: &mut D, x: i32, y: i32, w: i32, h: i32, color: Rgb888) -> Result<(), D::Error>
where
    D: DrawTarget<Color = Rgb888>,
{
    let (Ok(w), Ok(h)) = (u32::try_from(w), u32::try_from(h)) else {
        return Ok(());
    };
    target.fill_solid(&Rectangle::new(Point::new(x, y), Size::new(w, h)), color)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Frame;

    const W: i32 = 64;
    const H: i32 = 32;
    /// Corner blocks and the content-area inset are both 3 pixels.
    const M: i32 = 3;

    const YELLOW: Rgb888 = Rgb888::new(255, 255, 0);

    fn pattern(width: usize, height: usize) -> Frame {
        let mut f = Frame::new(width, height);
        render(&mut f).unwrap();
        f
    }

    fn at(f: &Frame, x: i32, y: i32) -> Rgb888 {
        f.pixel(Point::new(x, y)).unwrap()
    }

    fn in_corner(x: i32, y: i32) -> bool {
        !(M..W - M).contains(&x) && !(M..H - M).contains(&y)
    }

    fn on_border(x: i32, y: i32) -> bool {
        x == 0 || y == 0 || x == W - 1 || y == H - 1
    }

    fn assert_block(f: &Frame, x0: i32, y0: i32, color: Rgb888) {
        for y in y0..y0 + M {
            for x in x0..x0 + M {
                assert_eq!(at(f, x, y), color, "corner pixel ({x}, {y})");
            }
        }
    }

    #[test]
    fn corners_identify_orientation() {
        let f = pattern(64, 32);
        assert_block(&f, 0, 0, Rgb888::RED);
        assert_block(&f, W - M, 0, Rgb888::GREEN);
        assert_block(&f, 0, H - M, Rgb888::BLUE);
        assert_block(&f, W - M, H - M, YELLOW);
    }

    #[test]
    fn border_is_white_outside_corners() {
        let f = pattern(64, 32);
        for y in 0..H {
            for x in 0..W {
                if on_border(x, y) && !in_corner(x, y) {
                    assert_eq!(at(&f, x, y), Rgb888::WHITE, "border pixel ({x}, {y})");
                }
            }
        }
    }

    #[test]
    fn margin_between_border_and_content_is_black() {
        let f = pattern(64, 32);
        for y in 0..H {
            for x in 0..W {
                let in_margin = (1..M).contains(&x)
                    || (W - M..W - 1).contains(&x)
                    || (1..M).contains(&y)
                    || (H - M..H - 1).contains(&y);
                if in_margin && !on_border(x, y) && !in_corner(x, y) {
                    assert_eq!(at(&f, x, y), Rgb888::BLACK, "margin pixel ({x}, {y})");
                }
            }
        }
    }

    #[test]
    fn color_bars_appear_in_order() {
        let f = pattern(64, 32);
        let upper_rows = (H - 2 * M) / 2;
        let y = M + upper_rows / 2;
        let mut seen: Vec<Rgb888> = Vec::new();
        for x in M..W - M {
            let c = at(&f, x, y);
            if seen.last() != Some(&c) {
                seen.push(c);
            }
        }
        assert_eq!(
            seen,
            [
                Rgb888::RED,
                Rgb888::GREEN,
                Rgb888::BLUE,
                Rgb888::CYAN,
                Rgb888::MAGENTA,
                YELLOW,
                Rgb888::WHITE,
            ]
        );
    }

    #[test]
    fn gray_ramp_is_monotonic_from_black_to_white() {
        let f = pattern(64, 32);
        let lower_start = M + (H - 2 * M) / 2;
        for y in lower_start..H - M {
            let row: Vec<Rgb888> = (M..W - M).map(|x| at(&f, x, y)).collect();
            for (i, c) in row.iter().enumerate() {
                assert!(
                    c.r() == c.g() && c.g() == c.b(),
                    "not gray at ({i}, {y}): {c:?}"
                );
            }
            for pair in row.windows(2) {
                assert!(pair[1].r() >= pair[0].r(), "ramp decreases on row {y}");
            }
            assert!(row[0].r() <= 16, "ramp starts at {} on row {y}", row[0].r());
            let last = row[row.len() - 1].r();
            assert!(last >= 240, "ramp ends at {last} on row {y}");
        }
    }

    #[test]
    fn same_size_gives_same_image() {
        assert_eq!(pattern(64, 32), pattern(64, 32));
    }

    #[test]
    fn tiny_frames_do_not_panic() {
        for (w, h) in [(0, 0), (1, 1), (2, 5), (5, 2)] {
            let _ = pattern(w, h);
        }
    }
}
