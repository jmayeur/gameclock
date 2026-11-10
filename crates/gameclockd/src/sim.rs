//! A [`Panel`] that draws frames in a terminal on a dev machine
//! (`openspec/specs/terminal-simulator`).
//!
//! Each character cell is the upper half block `▀`. Its foreground color is
//! the upper pixel, and its background color is the lower pixel. So a 64×32
//! frame uses 64 columns and 16 lines.

use std::io::{self, Write};

use gameclock_core::embedded_graphics::pixelcolor::{Rgb888, RgbColor};
use gameclock_core::embedded_graphics::prelude::Point;
use gameclock_core::{Brightness, Frame, Panel};

/// Clear the screen and hide the cursor. Sent once, before the first frame.
const SETUP: &[u8] = b"\x1b[2J\x1b[?25l";
/// Move the cursor to the top-left. Sent at the start of each frame.
const HOME: &[u8] = b"\x1b[H";
/// Reset the colors. Sent at the end of each row and on drop.
const RESET: &[u8] = b"\x1b[0m";
/// Show the cursor. Sent on drop.
const SHOW: &[u8] = b"\x1b[?25h";
/// The upper half block. Foreground is the upper pixel, background the lower.
const CELL: &str = "▀";

/// Draws frames to `out` with 24-bit color escape codes.
#[derive(Debug)]
pub struct TerminalPanel<W: Write> {
    out: W,
    size: (usize, usize),
    /// True after the setup codes are written.
    started: bool,
    /// One buffer for all frames. Each frame goes out in one write.
    buf: Vec<u8>,
}

impl<W: Write> TerminalPanel<W> {
    /// A terminal panel of `width` × `height` pixels that writes to `out`.
    pub fn new(out: W, width: usize, height: usize) -> Self {
        Self {
            out,
            size: (width, height),
            started: false,
            buf: Vec::new(),
        }
    }

    /// The number of character rows for the frame height.
    fn rows(&self) -> usize {
        self.size.1.div_ceil(2)
    }
}

/// Scale `color` by `percent`, rounded down.
fn dim(color: Rgb888, percent: u8) -> (u8, u8, u8) {
    let scale = |c: u8| {
        let v = u16::from(c) * u16::from(percent) / 100;
        u8::try_from(v).unwrap_or(u8::MAX)
    };
    (scale(color.r()), scale(color.g()), scale(color.b()))
}

/// The pixel at (x, y), or black outside the frame.
fn pixel(frame: &Frame, x: usize, y: usize) -> Rgb888 {
    let (Ok(x), Ok(y)) = (i32::try_from(x), i32::try_from(y)) else {
        return Rgb888::BLACK;
    };
    frame.pixel(Point::new(x, y)).unwrap_or(Rgb888::BLACK)
}

impl<W: Write> Panel for TerminalPanel<W> {
    type Error = io::Error;

    fn dimensions(&self) -> (usize, usize) {
        self.size
    }

    fn present(&mut self, frame: &Frame, brightness: Brightness) -> Result<(), io::Error> {
        let percent = brightness.percent();
        self.buf.clear();
        if !self.started {
            self.buf.extend_from_slice(SETUP);
        }
        self.buf.extend_from_slice(HOME);
        for row in 0..frame.height().div_ceil(2) {
            for x in 0..frame.width() {
                let (fr, fg, fb) = dim(pixel(frame, x, 2 * row), percent);
                let (br, bg, bb) = dim(pixel(frame, x, 2 * row + 1), percent);
                write!(
                    self.buf,
                    "\x1b[38;2;{fr};{fg};{fb}m\x1b[48;2;{br};{bg};{bb}m{CELL}"
                )?;
            }
            self.buf.extend_from_slice(RESET);
            self.buf.push(b'\n');
        }
        self.out.write_all(&self.buf)?;
        self.out.flush()?;
        self.started = true;
        Ok(())
    }
}

impl<W: Write> Drop for TerminalPanel<W> {
    fn drop(&mut self) {
        // Drop cannot return an error. A failed write here only leaves the
        // terminal colors or cursor wrong, and `reset` corrects that.
        let below = self.rows() + 1;
        let _ = self.out.write_all(RESET);
        let _ = self.out.write_all(SHOW);
        let _ = write!(self.out, "\x1b[{below};1H");
        let _ = self.out.flush();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gameclock_core::embedded_graphics::pixelcolor::Rgb888;
    use gameclock_core::embedded_graphics::prelude::*;
    use gameclock_core::test_pattern;

    type Rgb = (u8, u8, u8);

    const HOME: &str = "\x1b[H";
    const CLEAR: &str = "\x1b[2J";
    const HIDE: &str = "\x1b[?25l";
    const SHOW: &str = "\x1b[?25h";
    const RESET: &str = "\x1b[0m";

    fn frame(width: usize, height: usize, pixels: &[((i32, i32), Rgb)]) -> Frame {
        let mut f = Frame::new(width, height);
        for &((x, y), (r, g, b)) in pixels {
            Pixel(Point::new(x, y), Rgb888::new(r, g, b))
                .draw(&mut f)
                .unwrap();
        }
        f
    }

    /// Present `frames` in order, drop the panel, and return all output.
    fn output(frames: &[(&Frame, u8)]) -> String {
        let mut buf = Vec::new();
        {
            let (w, h) = (frames[0].0.width(), frames[0].0.height());
            let mut panel = TerminalPanel::new(&mut buf, w, h);
            for &(f, pct) in frames {
                panel.present(f, Brightness::new(pct)).unwrap();
            }
        }
        String::from_utf8(buf).unwrap()
    }

    /// Read `ESC[<code>;2;R;G;Bm` at the start of `s`.
    fn color(s: &str, code: &str) -> Option<(Rgb, usize)> {
        let prefix = format!("\x1b[{code};2;");
        let rest = s.strip_prefix(&prefix)?;
        let end = rest.find('m')?;
        let mut parts = rest[..end].split(';').map(|p| p.parse::<u8>().ok());
        let rgb = (parts.next()??, parts.next()??, parts.next()??);
        Some((rgb, prefix.len() + end + 1))
    }

    /// The cells of the last frame in `out`, as rows of (foreground, background).
    fn last_frame(out: &str) -> Vec<Vec<(Rgb, Rgb)>> {
        let body = &out[out.rfind(HOME).expect("a frame") + HOME.len()..];
        let mut rows = vec![Vec::new()];
        let mut i = 0;
        while i < body.len() {
            let rest = &body[i..];
            if let Some((fg, n)) = color(rest, "38") {
                let (bg, m) = color(&rest[n..], "48").expect("background after foreground");
                assert!(rest[n + m..].starts_with('▀'), "cell must be ▀");
                rows.last_mut().unwrap().push((fg, bg));
                i += n + m + '▀'.len_utf8();
            } else if rest.starts_with('\n') {
                rows.push(Vec::new());
                i += 1;
            } else {
                i += rest.chars().next().unwrap().len_utf8();
            }
        }
        rows.retain(|r| !r.is_empty());
        rows
    }

    const RED: Rgb = (255, 0, 0);
    const GREEN: Rgb = (0, 255, 0);
    const BLUE: Rgb = (0, 0, 255);
    const WHITE: Rgb = (255, 255, 255);
    const BLACK: Rgb = (0, 0, 0);

    #[test]
    fn cell_colors_come_from_a_pixel_pair() {
        let f = frame(
            2,
            2,
            &[
                ((0, 0), RED),
                ((0, 1), BLUE),
                ((1, 0), GREEN),
                ((1, 1), WHITE),
            ],
        );
        let rows = last_frame(&output(&[(&f, 100)]));
        assert_eq!(rows, vec![vec![(RED, BLUE), (GREEN, WHITE)]]);
    }

    #[test]
    fn a_64x32_frame_uses_16_rows_of_64_cells() {
        let mut f = Frame::new(64, 32);
        test_pattern::render(&mut f).unwrap();
        let rows = last_frame(&output(&[(&f, 100)]));
        assert_eq!(rows.len(), 16);
        assert!(rows.iter().all(|r| r.len() == 64), "each row has 64 cells");
    }

    #[test]
    fn an_odd_last_row_has_a_black_lower_half() {
        let f = frame(1, 3, &[((0, 0), WHITE), ((0, 1), WHITE), ((0, 2), WHITE)]);
        let rows = last_frame(&output(&[(&f, 100)]));
        assert_eq!(rows, vec![vec![(WHITE, WHITE)], vec![(WHITE, BLACK)]]);
    }

    #[test]
    fn half_brightness_halves_each_channel() {
        let f = frame(1, 2, &[((0, 0), (200, 100, 51))]);
        let rows = last_frame(&output(&[(&f, 50)]));
        assert_eq!(rows, vec![vec![((100, 50, 25), BLACK)]]);
    }

    #[test]
    fn zero_brightness_is_black() {
        let mut f = Frame::new(64, 32);
        test_pattern::render(&mut f).unwrap();
        let rows = last_frame(&output(&[(&f, 0)]));
        assert_eq!(rows.len(), 16);
        assert!(rows.iter().flatten().all(|&c| c == (BLACK, BLACK)));
    }

    #[test]
    fn setup_happens_once() {
        let f = frame(2, 2, &[((0, 0), RED)]);
        let out = output(&[(&f, 100), (&f, 100)]);
        assert_eq!(out.matches(CLEAR).count(), 1, "one clear");
        assert_eq!(out.matches(HIDE).count(), 1, "one hide");
        assert_eq!(out.matches(HOME).count(), 2, "one home for each frame");
        let first_home = out.find(HOME).unwrap();
        assert!(out.find(CLEAR).unwrap() < first_home);
        assert!(out.find(HIDE).unwrap() < first_home);
        let first_cell = out.find("\x1b[38;2;").unwrap();
        assert!(
            first_home < first_cell,
            "each frame starts with cursor home"
        );
    }

    #[test]
    fn drop_restores_the_terminal() {
        let f = Frame::new(64, 32);
        let out = output(&[(&f, 100)]);
        let tail = &out[out.rfind('▀').unwrap() + '▀'.len_utf8()..];
        let reset = tail.find(RESET).expect("reset after the frame");
        let show = tail.find(SHOW).expect("show cursor after the frame");
        let below = tail.find("\x1b[17;1H").expect("move to line 17");
        assert!(reset < show && show < below, "tail: {tail:?}");
    }

    #[test]
    fn a_closed_output_fails_the_present() {
        struct Closed;
        impl Write for Closed {
            fn write(&mut self, _: &[u8]) -> io::Result<usize> {
                Err(io::Error::other("the pipe is closed"))
            }
            fn flush(&mut self) -> io::Result<()> {
                Err(io::Error::other("the pipe is closed"))
            }
        }
        let mut panel = TerminalPanel::new(Closed, 2, 2);
        let err = panel
            .present(&Frame::new(2, 2), Brightness::MAX)
            .unwrap_err();
        assert!(err.to_string().contains("the pipe is closed"), "{err}");
    }
}
