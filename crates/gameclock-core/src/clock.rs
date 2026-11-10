//! The clock face: the local time in the legacy format, in the color and
//! brightness of the schedule (`openspec/specs/clock-scene`).

use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::mono_font::ascii::FONT_7X13_BOLD;
use embedded_graphics::prelude::*;
use embedded_graphics::text::{Baseline, Text};
use jiff::{Timestamp, Zoned};

use crate::schedule::Schedule;
use crate::{Brightness, Frame, Scene};

/// The left edge and the baseline of the text, as the legacy clock draws it.
const ORIGIN: Point = Point::new(8, 20);

/// The local time as `09:05PM`, in the style that the schedule gives.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Clock {
    schedule: Schedule,
}

impl Clock {
    /// A clock that gets its color and brightness from `schedule`.
    #[must_use]
    pub fn new(schedule: Schedule) -> Self {
        Self { schedule }
    }
}

impl Scene for Clock {
    fn render(&mut self, now: &Zoned, frame: &mut Frame) {
        let style = MonoTextStyle::new(&FONT_7X13_BOLD, self.schedule.style_at(now).color);
        let text = legacy_text(now);
        // The text is ASCII, so this conversion does not fail.
        let Ok(text) = core::str::from_utf8(&text) else {
            return;
        };
        match Text::with_baseline(text, ORIGIN, style, Baseline::Alphabetic).draw(frame) {
            Ok(_) => {}
            Err(never) => match never {},
        }
    }

    /// The start of the next whole minute. All current time zones have
    /// offsets in whole minutes, so this is also the next local minute.
    /// Each schedule rule starts at a whole minute too.
    fn next_change(&self, now: &Zoned) -> Option<Timestamp> {
        // `div_euclid` rounds down for times before 1970 too. A time before
        // 1970 has a negative second, or a negative nanosecond.
        let t = now.timestamp();
        let second = t.as_second() - i64::from(t.subsec_nanosecond() < 0);
        let next = second.div_euclid(60).checked_add(1)?.checked_mul(60)?;
        Timestamp::from_second(next).ok()
    }

    fn is_finished(&self, _now: &Zoned) -> bool {
        false
    }

    fn brightness(&self, now: &Zoned) -> Brightness {
        self.schedule.style_at(now).brightness
    }
}

/// The local time of `now` as `hh:mmAM` or `hh:mmPM`, with no heap memory.
/// The hour is 12 for the first hour after midnight and after noon.
fn legacy_text(now: &Zoned) -> [u8; 7] {
    // jiff keeps the hour in 0..=23 and the minute in 0..=59.
    let hour = now.hour().unsigned_abs();
    let minute = now.minute().unsigned_abs();
    let hour12 = (hour + 11) % 12 + 1;
    let half = if hour < 12 { b'A' } else { b'P' };
    [
        b'0' + hour12 / 10,
        b'0' + hour12 % 10,
        b':',
        b'0' + minute / 10,
        b'0' + minute % 10,
        half,
        b'M',
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schedule::{Rule, Schedule, Style, TimeOfDay, Weekdays};
    use crate::{Brightness, Frame, Scene};
    use embedded_graphics::mono_font::MonoTextStyle;
    use embedded_graphics::mono_font::ascii::FONT_7X13_BOLD;
    use embedded_graphics::pixelcolor::{Rgb888, RgbColor};
    use embedded_graphics::text::{Baseline, Text};
    use jiff::tz::TimeZone;
    use jiff::{Timestamp, Zoned};

    fn utc(s: &str) -> Zoned {
        s.parse::<Timestamp>().unwrap().to_zoned(TimeZone::UTC)
    }

    /// The rules of `America/Los_Angeles`. The core has no zone database.
    fn los_angeles(s: &str) -> Zoned {
        let zone = TimeZone::posix("PST8PDT,M3.2.0,M11.1.0").unwrap();
        s.parse::<Timestamp>().unwrap().to_zoned(zone)
    }

    /// The reference image: `text` in the 7×13 bold font at x=8, baseline
    /// y=20, in `color`, on black.
    fn expected(text: &str, color: Rgb888) -> Frame {
        let mut f = Frame::new(64, 32);
        let style = MonoTextStyle::new(&FONT_7X13_BOLD, color);
        Text::with_baseline(text, Point::new(8, 20), style, Baseline::Alphabetic)
            .draw(&mut f)
            .unwrap();
        f
    }

    fn render(clock: &mut Clock, now: &Zoned) -> Frame {
        let mut f = Frame::new(64, 32);
        clock.render(now, &mut f);
        f
    }

    fn shows(now: &Zoned, text: &str) {
        let mut clock = Clock::new(Schedule::legacy());
        let color = Schedule::legacy().style_at(now).color;
        assert_eq!(render(&mut clock, now), expected(text, color), "at {now}");
    }

    // The clock shows the local time in the legacy format.

    #[test]
    fn morning_time() {
        shows(&utc("2026-11-10T09:05:30Z"), "09:05AM");
    }

    #[test]
    fn evening_time() {
        shows(&utc("2026-11-10T21:05:00Z"), "09:05PM");
    }

    #[test]
    fn midnight_is_12_am() {
        shows(&utc("2026-11-10T00:00:00Z"), "12:00AM");
    }

    #[test]
    fn noon_is_12_pm() {
        shows(&utc("2026-11-10T12:30:00Z"), "12:30PM");
    }

    #[test]
    fn last_minute_of_the_day() {
        shows(&utc("2026-11-10T23:59:59.999Z"), "11:59PM");
    }

    #[test]
    fn time_zone_of_the_injected_time() {
        shows(&los_angeles("2026-11-10T17:05:00Z"), "09:05AM");
    }

    #[test]
    fn daylight_saving_time_starts() {
        shows(&los_angeles("2026-03-08T10:00:00Z"), "03:00AM");
        shows(&los_angeles("2026-03-08T09:59:00Z"), "01:59AM");
    }

    #[test]
    fn the_text_is_lit() {
        let f = render(
            &mut Clock::new(Schedule::legacy()),
            &utc("2026-11-10T09:05:00Z"),
        );
        assert!(f.pixels().iter().any(|&p| p != Rgb888::BLACK));
    }

    #[test]
    fn tiny_frames() {
        let mut clock = Clock::new(Schedule::legacy());
        for (w, h) in [(0, 0), (1, 1), (8, 20)] {
            let mut f = Frame::new(w, h);
            clock.render(&utc("2026-11-10T09:05:00Z"), &mut f);
        }
    }

    // Text color and brightness from the style.

    #[test]
    fn text_color_and_brightness_from_the_style() {
        let green_40 = Style::new(Rgb888::GREEN, Brightness::new(40));
        let all_day = Rule::new(
            Weekdays::EACH_DAY,
            TimeOfDay::hm(0, 0).unwrap(),
            TimeOfDay::END_OF_DAY,
            green_40,
        )
        .unwrap();
        let schedule = Schedule::new(vec![all_day], Style::new(Rgb888::RED, Brightness::new(25)));
        let mut clock = Clock::new(schedule);
        for time in ["2026-11-10T03:00:00Z", "2026-11-10T21:30:00Z"] {
            let now = utc(time);
            let f = render(&mut clock, &now);
            let lit: Vec<_> = f.pixels().iter().filter(|&&p| p != Rgb888::BLACK).collect();
            assert!(!lit.is_empty(), "at {time}");
            assert!(lit.iter().all(|&&p| p == Rgb888::GREEN), "at {time}");
            assert_eq!(clock.brightness(&now), Brightness::new(40), "at {time}");
        }
    }

    // The clock changes only at each new minute.

    fn next(now: &Zoned) -> Option<Timestamp> {
        Clock::new(Schedule::legacy()).next_change(now)
    }

    #[test]
    fn next_change_inside_a_minute() {
        let expected = "2026-11-10T09:06:00Z".parse().ok();
        assert_eq!(next(&utc("2026-11-10T09:05:30.250Z")), expected);
    }

    #[test]
    fn next_change_on_a_whole_minute() {
        let expected = "2026-11-10T09:06:00Z".parse().ok();
        assert_eq!(next(&utc("2026-11-10T09:05:00Z")), expected);
    }

    #[test]
    fn next_change_across_the_daylight_saving_time_gap() {
        let expected = "2026-03-08T10:00:00Z".parse().ok();
        assert_eq!(next(&los_angeles("2026-03-08T09:59:30Z")), expected);
    }

    #[test]
    fn next_change_before_1970() {
        let expected = "1969-12-31T23:59:00Z".parse().ok();
        assert_eq!(next(&utc("1969-12-31T23:58:59.5Z")), expected);
    }

    #[test]
    fn never_finished() {
        let clock = Clock::new(Schedule::legacy());
        for time in ["1970-01-01T00:00:00Z", "2026-11-10T23:59:59Z"] {
            assert!(!clock.is_finished(&utc(time)), "at {time}");
        }
    }
}
