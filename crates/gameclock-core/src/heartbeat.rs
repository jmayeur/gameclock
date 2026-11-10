//! A temporary base scene: the test pattern with one marker that moves each
//! second (`openspec/specs/heartbeat-scene`).
//!
//! It shows that the event loop works until the clock scene replaces it.

use embedded_graphics::pixelcolor::Rgb888;
use embedded_graphics::prelude::*;
use jiff::{Timestamp, Zoned};

use crate::{Frame, Scene, test_pattern};

/// The test pattern's corner size and inset (`openspec/specs/test-pattern`).
const INSET: usize = 3;

/// The row of the marker, in the black margin under the top border.
const ROW: i32 = 1;

/// The test pattern with one white marker on row 1. The marker moves one
/// column each second. The scene has no state.
#[derive(Debug, Clone, Copy, Default)]
pub struct Heartbeat;

impl Scene for Heartbeat {
    fn render(&mut self, now: &Zoned, frame: &mut Frame) {
        match test_pattern::render(frame) {
            Ok(()) => {}
            Err(never) => match never {},
        }
        let Some(span) = frame.width().checked_sub(2 * INSET).filter(|&s| s > 0) else {
            return;
        };
        if frame.height() <= 1 {
            return;
        }
        // The span is smaller than the frame width, so it fits in i64 and i32.
        let span = i64::try_from(span).unwrap_or(i64::MAX);
        let step = whole_second(now.timestamp()).rem_euclid(span);
        let column = i64::try_from(INSET).unwrap_or(0) + step;
        if let Ok(x) = i32::try_from(column) {
            match Pixel(Point::new(x, ROW), Rgb888::WHITE).draw(frame) {
                Ok(()) => {}
                Err(never) => match never {},
            }
        }
    }

    fn next_change(&self, now: &Zoned) -> Option<Timestamp> {
        let next = whole_second(now.timestamp()).checked_add(1)?;
        Timestamp::from_second(next).ok()
    }

    fn is_finished(&self, _now: &Zoned) -> bool {
        false
    }
}

/// The Unix second that contains `t`. Times before 1970 round down too.
fn whole_second(t: Timestamp) -> i64 {
    let second = t.as_second();
    if t.subsec_nanosecond() < 0 {
        second - 1
    } else {
        second
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Frame, Scene, test_pattern};
    use embedded_graphics::pixelcolor::Rgb888;
    use jiff::Timestamp;
    use jiff::tz::TimeZone;

    fn at(s: &str) -> jiff::Zoned {
        s.parse::<Timestamp>().unwrap().to_zoned(TimeZone::UTC)
    }

    fn heartbeat(width: usize, height: usize, time: &str) -> Frame {
        let mut f = Frame::new(width, height);
        Heartbeat.render(&at(time), &mut f);
        f
    }

    fn pattern(width: usize, height: usize) -> Frame {
        let mut f = Frame::new(width, height);
        test_pattern::render(&mut f).unwrap();
        f
    }

    fn differences(a: &Frame, b: &Frame) -> Vec<usize> {
        let pairs = a.pixels().iter().zip(b.pixels());
        (0..)
            .zip(pairs)
            .filter(|(_, (p, q))| p != q)
            .map(|(i, _)| i)
            .collect()
    }

    fn white_at(f: &Frame, x: i32) -> bool {
        f.pixel(Point::new(x, 1)) == Some(Rgb888::WHITE)
    }

    #[test]
    fn only_the_marker_differs_from_the_test_pattern() {
        for time in ["1970-01-01T00:00:00Z", "2026-11-10T08:15:42.5Z"] {
            let f = heartbeat(64, 32, time);
            let diff = differences(&f, &pattern(64, 32));
            assert_eq!(diff.len(), 1, "at {time}");
            assert_eq!(diff[0] / 64, 1, "the marker is on row 1 at {time}");
            assert_eq!(f.pixels()[diff[0]], Rgb888::WHITE);
        }
    }

    #[test]
    fn marker_at_the_start_of_a_cycle() {
        assert!(white_at(&heartbeat(64, 32, "1970-01-01T00:00:00Z"), 3));
    }

    #[test]
    fn marker_at_the_end_of_a_cycle() {
        assert!(white_at(&heartbeat(64, 32, "1970-01-01T00:00:57.999Z"), 60));
    }

    #[test]
    fn marker_starts_the_cycle_again() {
        let f = heartbeat(64, 32, "1970-01-01T00:00:58Z");
        assert!(white_at(&f, 3));
        assert!(!white_at(&f, 60));
    }

    #[test]
    fn marker_before_1970() {
        // -1 s is the last step of a cycle.
        assert!(white_at(&heartbeat(64, 32, "1969-12-31T23:59:59Z"), 60));
    }

    #[test]
    fn next_change_inside_a_second() {
        let next = Heartbeat.next_change(&at("2026-01-01T12:00:00.250Z"));
        assert_eq!(next, Some("2026-01-01T12:00:01Z".parse().unwrap()));
    }

    #[test]
    fn next_change_on_a_whole_second() {
        let next = Heartbeat.next_change(&at("2026-01-01T12:00:00Z"));
        assert_eq!(next, Some("2026-01-01T12:00:01Z".parse().unwrap()));
    }

    #[test]
    fn never_finished() {
        for time in ["1970-01-01T00:00:00Z", "2026-01-01T12:00:00Z"] {
            assert!(!Heartbeat.is_finished(&at(time)));
        }
    }

    #[test]
    fn tiny_frames() {
        for (w, h) in [(0, 0), (1, 1), (6, 32), (64, 1)] {
            let f = heartbeat(w, h, "1970-01-01T00:00:05Z");
            assert_eq!(f, pattern(w, h), "{w}×{h}");
        }
    }
}
