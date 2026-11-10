//! The time-of-day schedule: the color and brightness for each local time
//! (`openspec/specs/clock-scene`, docs/adr/0014).
//!
//! The schedule is data. A later change can read the same types from a file.

use core::fmt;

use embedded_graphics::pixelcolor::Rgb888;
use jiff::Zoned;
use jiff::civil::Weekday;

use crate::Brightness;

/// Minutes in one day.
const DAY: u16 = 24 * 60;

/// A time of day in whole minutes after midnight, from 00:00 to 24:00.
/// 24:00 is the end of the day.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TimeOfDay(u16);

impl TimeOfDay {
    /// The end of the day, 24:00.
    pub const END_OF_DAY: Self = Self(DAY);

    /// The time `hour`:`minute`.
    ///
    /// # Errors
    /// Gives [`ScheduleError::InvalidTime`] if the hour is more than 24, the
    /// minute is more than 59, or the time is after 24:00.
    pub const fn hm(hour: u8, minute: u8) -> Result<Self, ScheduleError> {
        let minutes = hour as u16 * 60 + minute as u16;
        if hour > 24 || minute > 59 || minutes > DAY {
            return Err(ScheduleError::InvalidTime { hour, minute });
        }
        Ok(Self(minutes))
    }

    /// The whole minute of the day that contains `now`, in its time zone.
    #[must_use]
    pub fn of(now: &Zoned) -> Self {
        // jiff keeps the hour in 0..=23 and the minute in 0..=59.
        let hour = u16::try_from(now.hour()).unwrap_or(0);
        let minute = u16::try_from(now.minute()).unwrap_or(0);
        Self(hour * 60 + minute)
    }
}

impl fmt::Display for TimeOfDay {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:02}:{:02}", self.0 / 60, self.0 % 60)
    }
}

/// A set of weekdays. The empty set means each day.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Weekdays(u8);

impl Weekdays {
    /// The empty set: the rule applies on each day.
    pub const EACH_DAY: Self = Self(0);

    /// The set of `days`.
    #[must_use]
    pub fn of(days: &[Weekday]) -> Self {
        Self(days.iter().fold(0, |bits, &d| bits | bit(d)))
    }

    /// True if the set is empty or has `day`.
    #[must_use]
    pub fn contains(self, day: Weekday) -> bool {
        self.0 == 0 || self.0 & bit(day) != 0
    }
}

/// The bit of `day` in a [`Weekdays`] set.
fn bit(day: Weekday) -> u8 {
    // The offset is in 0..=6.
    1 << day.to_monday_zero_offset().unsigned_abs()
}

/// A text color and a panel brightness.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Style {
    /// The text color.
    pub color: Rgb888,
    /// The panel brightness.
    pub brightness: Brightness,
}

impl Style {
    /// A style with `color` and `brightness`.
    #[must_use]
    pub const fn new(color: Rgb888, brightness: Brightness) -> Self {
        Self { color, brightness }
    }
}

/// One period of the schedule: from `start` to before `end`, on `days`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Rule {
    days: Weekdays,
    start: TimeOfDay,
    end: TimeOfDay,
    style: Style,
}

impl Rule {
    /// A rule for the times from `start` to before `end`, on `days`.
    ///
    /// # Errors
    /// Gives [`ScheduleError::EmptyRule`] if `start` is not before `end`. A
    /// period that crosses midnight is two rules.
    pub const fn new(
        days: Weekdays,
        start: TimeOfDay,
        end: TimeOfDay,
        style: Style,
    ) -> Result<Self, ScheduleError> {
        if start.0 >= end.0 {
            return Err(ScheduleError::EmptyRule { start, end });
        }
        Ok(Self {
            days,
            start,
            end,
            style,
        })
    }

    fn matches(&self, day: Weekday, time: TimeOfDay) -> bool {
        self.days.contains(day) && self.start <= time && time < self.end
    }
}

/// A list of rules and a default style. The first rule that matches gives
/// the style.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Schedule {
    rules: Vec<Rule>,
    default: Style,
}

impl Schedule {
    /// A schedule with `rules` in order, and `default` for the times that no
    /// rule matches.
    #[must_use]
    pub fn new(rules: Vec<Rule>, default: Style) -> Self {
        Self { rules, default }
    }

    /// The style for the local time and weekday of `now`.
    #[must_use]
    pub fn style_at(&self, now: &Zoned) -> Style {
        let day = now.weekday();
        let time = TimeOfDay::of(now);
        self.rules
            .iter()
            .find(|r| r.matches(day, time))
            .map_or(self.default, |r| r.style)
    }
}

/// A schedule value that is not valid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScheduleError {
    /// The time is not a time of day from 00:00 to 24:00.
    InvalidTime {
        /// The hour that was given.
        hour: u8,
        /// The minute that was given.
        minute: u8,
    },
    /// The start of the rule is not before its end.
    EmptyRule {
        /// The start of the rule.
        start: TimeOfDay,
        /// The end of the rule.
        end: TimeOfDay,
    },
}

impl fmt::Display for ScheduleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidTime { hour, minute } => write!(
                f,
                "the time {hour}:{minute:02} is not valid. Use a time from 00:00 to 24:00."
            ),
            Self::EmptyRule { start, end } => write!(
                f,
                "the rule starts at {start} and ends at {end}. The start must be before the end."
            ),
        }
    }
}

impl std::error::Error for ScheduleError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Brightness;
    use embedded_graphics::pixelcolor::{Rgb888, RgbColor};
    use jiff::civil::Weekday;
    use jiff::tz::TimeZone;
    use jiff::{Timestamp, Zoned};

    fn at(s: &str) -> Zoned {
        s.parse::<Timestamp>().unwrap().to_zoned(TimeZone::UTC)
    }

    fn hm(hour: u8, minute: u8) -> TimeOfDay {
        TimeOfDay::hm(hour, minute).unwrap()
    }

    fn style(color: Rgb888, percent: u8) -> Style {
        Style::new(color, Brightness::new(percent))
    }

    fn rule(days: Weekdays, start: TimeOfDay, end: TimeOfDay, style: Style) -> Rule {
        Rule::new(days, start, end, style).unwrap()
    }

    const WHITE_80: (Rgb888, u8) = (Rgb888::WHITE, 80);

    // Rule times are whole minutes.

    #[test]
    fn valid_times() {
        for (h, m) in [(0, 0), (23, 59), (24, 0)] {
            assert!(TimeOfDay::hm(h, m).is_ok(), "{h}:{m}");
        }
    }

    #[test]
    fn times_that_are_not_valid() {
        for (h, m) in [(24, 1), (25, 0), (12, 60), (255, 255)] {
            assert_eq!(
                TimeOfDay::hm(h, m),
                Err(ScheduleError::InvalidTime { hour: h, minute: m }),
                "{h}:{m}"
            );
        }
    }

    // A rule must not be empty.

    #[test]
    fn rule_with_start_not_before_end() {
        let white = style(WHITE_80.0, WHITE_80.1);
        for (start, end) in [(hm(10, 0), hm(10, 0)), (hm(12, 0), hm(10, 0))] {
            assert_eq!(
                Rule::new(Weekdays::EACH_DAY, start, end, white),
                Err(ScheduleError::EmptyRule { start, end })
            );
        }
    }

    // First matching rule wins. End time is not part of the rule.

    fn green_then_white() -> Schedule {
        Schedule::new(
            vec![
                rule(
                    Weekdays::EACH_DAY,
                    hm(10, 0),
                    hm(12, 0),
                    style(Rgb888::GREEN, 40),
                ),
                rule(
                    Weekdays::EACH_DAY,
                    hm(0, 0),
                    hm(24, 0),
                    style(WHITE_80.0, WHITE_80.1),
                ),
            ],
            style(Rgb888::RED, 25),
        )
    }

    #[test]
    fn first_matching_rule_wins() {
        let s = green_then_white();
        assert_eq!(
            s.style_at(&at("2026-11-10T11:00:00Z")),
            style(Rgb888::GREEN, 40)
        );
    }

    #[test]
    fn end_time_is_not_part_of_the_rule() {
        let s = green_then_white();
        assert_eq!(
            s.style_at(&at("2026-11-10T12:00:00Z")),
            style(WHITE_80.0, WHITE_80.1)
        );
        assert_eq!(
            s.style_at(&at("2026-11-10T11:59:59.999Z")),
            style(Rgb888::GREEN, 40)
        );
    }

    #[test]
    fn start_time_is_part_of_the_rule() {
        let s = green_then_white();
        assert_eq!(
            s.style_at(&at("2026-11-10T10:00:00Z")),
            style(Rgb888::GREEN, 40)
        );
    }

    // Weekday set limits a rule.

    #[test]
    fn weekday_set_limits_a_rule() {
        let monday = Weekdays::of(&[Weekday::Monday]);
        let s = Schedule::new(
            vec![rule(monday, hm(0, 0), hm(24, 0), style(Rgb888::GREEN, 40))],
            style(WHITE_80.0, WHITE_80.1),
        );
        // 2026-11-10 is a Tuesday. 2026-11-09 is a Monday.
        assert_eq!(
            s.style_at(&at("2026-11-10T08:00:00Z")),
            style(WHITE_80.0, WHITE_80.1)
        );
        assert_eq!(
            s.style_at(&at("2026-11-09T08:00:00Z")),
            style(Rgb888::GREEN, 40)
        );
    }

    #[test]
    fn weekday_set_contains() {
        let days = Weekdays::of(&[Weekday::Sunday, Weekday::Thursday]);
        assert!(days.contains(Weekday::Sunday));
        assert!(days.contains(Weekday::Thursday));
        assert!(!days.contains(Weekday::Friday));
        assert!(Weekdays::EACH_DAY.contains(Weekday::Friday));
    }

    // No rule matches.

    #[test]
    fn no_rule_matches() {
        let s = Schedule::new(Vec::new(), style(Rgb888::RED, 25));
        for time in [
            "2026-11-10T00:00:00Z",
            "2026-11-10T13:37:00Z",
            "2026-11-10T23:59:59Z",
        ] {
            assert_eq!(s.style_at(&at(time)), style(Rgb888::RED, 25), "at {time}");
        }
    }
}
