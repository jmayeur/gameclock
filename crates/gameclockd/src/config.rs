//! The config file: its TOML format, the checks, and the load
//! (`openspec/specs/daemon-config`, docs/adr/0019).
//!
//! The parse is a pure function of the text. Only [`load`] reads a file.

use core::fmt;

use core::ops::Range;

use gameclock_core::Brightness;
use gameclock_core::alert::{
    Alert, AlertError, Alerts, Background, ClearRule, Font, Motion, Priority, TextColor, TextStyle,
    Trigger, TriggerError,
};
use gameclock_core::embedded_graphics::pixelcolor::Rgb888;
use gameclock_core::schedule::{Rule, Schedule, Style, TimeOfDay, Weekdays};
use gameclock_core::tables::Tables;
use jiff::SignedDuration;
use jiff::civil::Weekday;
use serde::Deserialize;
use toml::Spanned;

/// A config file that the daemon cannot use.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigError {
    /// The text has an error at a line and a column, both from 1.
    Parse {
        /// The path of the file.
        path: String,
        /// The line of the error.
        line: usize,
        /// The column of the error, in characters.
        column: usize,
        /// The problem.
        message: String,
    },
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse {
                path,
                line,
                column,
                message,
            } => write!(f, "{path}:{line}:{column}: {message}"),
        }
    }
}

impl std::error::Error for ConfigError {}

/// The tables of the config text `text`. `path` names the file in errors.
///
/// # Errors
/// [`ConfigError::Parse`] for a TOML error, an unknown key, a wrong type, or
/// a value out of its range.
pub fn parse(path: &str, text: &str) -> Result<Tables, ConfigError> {
    let at = |span: Range<usize>, message: String| {
        let (line, column) = position(text, span.start);
        ConfigError::Parse {
            path: path.to_owned(),
            line,
            column,
            message,
        }
    };
    let file: File = toml::from_str(text).map_err(|e| {
        let message = e.message().trim_end().to_owned();
        at(e.span().unwrap_or(0..0), message)
    })?;
    let legacy = Tables::legacy();
    let schedule = match file.schedule {
        Some(section) => section.schedule().map_err(|(span, m)| at(span, m))?,
        None => legacy.schedule,
    };
    let alerts = match file.alert {
        Some(entries) => alerts(entries).map_err(|(span, m)| at(span, m))?,
        None => legacy.alerts,
    };
    let triggers = match file.trigger {
        Some(entries) => entries
            .into_iter()
            .map(|e| e.trigger(&alerts))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|(span, m)| at(span, m))?,
        None => Trigger::legacy(&alerts),
    };
    Ok(Tables {
        schedule,
        alerts,
        triggers,
    })
}

/// The line and the column of the byte `offset` in `text`, both from 1. The
/// column counts characters.
fn position(text: &str, offset: usize) -> (usize, usize) {
    let before = text.get(..offset).unwrap_or(text);
    let line = before.matches('\n').count() + 1;
    let line_start = before.rfind('\n').map_or(0, |i| i + 1);
    let column = before.get(line_start..).map_or(0, |s| s.chars().count()) + 1;
    (line, column)
}

/// A problem at a byte range of the text.
type Problem = (Range<usize>, String);

/// A value with its byte range in the text.
type Value<T> = Spanned<T>;

fn problem<T>(value: &Value<T>, message: impl Into<String>) -> Problem {
    (value.span(), message.into())
}

/// The whole file. Each section is optional.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    schedule: Option<ScheduleSection>,
    alert: Option<Vec<AlertEntry>>,
    trigger: Option<Vec<TriggerEntry>>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ScheduleSection {
    default: StyleEntry,
    #[serde(default)]
    rule: Vec<RuleEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct StyleEntry {
    color: Value<String>,
    brightness: Value<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RuleEntry {
    days: Option<Value<Vec<Value<String>>>>,
    from: Value<String>,
    to: Value<String>,
    color: Value<String>,
    brightness: Value<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct AlertEntry {
    name: Value<String>,
    text: Value<String>,
    background: Value<String>,
    font: Value<String>,
    baseline: Value<i64>,
    color: Value<String>,
    x: Option<Value<i64>>,
    passes: Option<Value<i64>>,
    seconds: Value<i64>,
    priority: Value<i64>,
    clear: Value<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct TriggerEntry {
    alert: Value<String>,
    days: Option<Value<Vec<Value<String>>>>,
    from: Value<String>,
    to: Value<String>,
    every: Value<i64>,
}

impl ScheduleSection {
    fn schedule(self) -> Result<Schedule, Problem> {
        let default = self.default.style()?;
        let rules = self
            .rule
            .into_iter()
            .map(RuleEntry::rule)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Schedule::new(rules, default))
    }
}

impl StyleEntry {
    fn style(&self) -> Result<Style, Problem> {
        Ok(Style::new(
            color(&self.color)?,
            brightness(&self.brightness)?,
        ))
    }
}

impl RuleEntry {
    fn rule(self) -> Result<Rule, Problem> {
        let days = days(self.days.as_ref())?;
        let from = time(&self.from)?;
        let to = time(&self.to)?;
        let style = Style::new(color(&self.color)?, brightness(&self.brightness)?);
        Rule::new(days, from, to, style).map_err(|_| problem(&self.to, WINDOW))
    }
}

/// The message for a time window whose start is not before its end.
const WINDOW: &str = "the start time must be before the end time.";

fn alerts(entries: Vec<AlertEntry>) -> Result<Alerts, Problem> {
    let mut spans = Vec::with_capacity(entries.len());
    let mut list = Vec::with_capacity(entries.len());
    for entry in entries {
        spans.push((
            entry.name.span(),
            entry.text.span(),
            entry.passes.as_ref().map(Spanned::span),
            entry.seconds.span(),
        ));
        list.push(entry.alert()?);
    }
    Alerts::new(list).map_err(|e| {
        let (name, text, passes, seconds) = spans.get(e.index).cloned().unwrap_or_default();
        let span = match e.error {
            AlertError::Name | AlertError::Stop | AlertError::Duplicate(_) => name,
            AlertError::Text => text,
            AlertError::Passes => passes.unwrap_or(name),
            AlertError::Duration => seconds,
        };
        (span, e.error.to_string())
    })
}

impl AlertEntry {
    fn alert(self) -> Result<Alert, Problem> {
        let background = match self.background.get_ref().as_str() {
            "black" => Background::Black,
            "color-wheel" => Background::ColorWheel,
            _ => {
                return Err(problem(
                    &self.background,
                    "the background must be black or color-wheel.",
                ));
            }
        };
        let font = match self.font.get_ref().as_str() {
            "small" => Font::Small,
            "large" => Font::Large,
            _ => return Err(problem(&self.font, "the font must be small or large.")),
        };
        let baseline = in_range(&self.baseline, 0..=63, "the baseline must be from 0 to 63.")?;
        let text_color = if self.color.get_ref() == "contrast" {
            TextColor::Contrast
        } else {
            TextColor::Fixed(hex(self.color.get_ref()).ok_or_else(|| {
                problem(
                    &self.color,
                    "the color must be contrast or six hexadecimal digits.",
                )
            })?)
        };
        let motion = match (&self.x, &self.passes) {
            (Some(x), None) => Motion::Still {
                x: in_range(x, -64..=64, "x must be from -64 to 64.")?,
            },
            (None, Some(passes)) => Motion::Scroll {
                passes: in_range(passes, 0..=255, &AlertError::Passes.to_string())?,
            },
            _ => {
                return Err(problem(
                    &self.name,
                    "an alert must have x or passes, but not both.",
                ));
            }
        };
        let seconds = in_range(
            &self.seconds,
            0..=i64::from(u16::MAX),
            &AlertError::Duration.to_string(),
        )?;
        let priority = in_range(
            &self.priority,
            0..=255,
            "the priority must be from 0 to 255.",
        )?;
        let clear = match self.clear.get_ref().as_str() {
            "removable" => ClearRule::Removable,
            "kept" => ClearRule::Kept,
            _ => {
                return Err(problem(
                    &self.clear,
                    "the clear rule must be removable or kept.",
                ));
            }
        };
        Ok(Alert {
            name: self.name.into_inner(),
            background,
            text: self.text.into_inner(),
            style: TextStyle {
                font,
                baseline,
                color: text_color,
                motion,
            },
            duration: SignedDuration::from_secs(seconds),
            priority: Priority(priority),
            clear,
        })
    }
}

impl TriggerEntry {
    fn trigger(self, alerts: &Alerts) -> Result<Trigger, Problem> {
        let alert = alerts.find(self.alert.get_ref()).ok_or_else(|| {
            problem(
                &self.alert,
                format!(
                    "the trigger names the alert '{}', which is not in the alert table.",
                    self.alert.get_ref()
                ),
            )
        })?;
        let days = days(self.days.as_ref())?;
        let from = time(&self.from)?;
        let to = time(&self.to)?;
        let every = SignedDuration::from_secs(*self.every.get_ref());
        Trigger::new(days, from, to, every, alert).map_err(|e| match e {
            TriggerError::Window => problem(&self.to, WINDOW),
            TriggerError::Every => problem(&self.every, e.to_string()),
        })
    }
}

/// A value of `value` in `range`, as the type `T`.
fn in_range<T: TryFrom<i64>>(
    value: &Value<i64>,
    range: core::ops::RangeInclusive<i64>,
    message: &str,
) -> Result<T, Problem> {
    let v = *value.get_ref();
    if !range.contains(&v) {
        return Err(problem(value, message));
    }
    T::try_from(v).map_err(|_| problem(value, message))
}

fn brightness(value: &Value<i64>) -> Result<Brightness, Problem> {
    let percent: u8 = in_range(value, 0..=100, "the brightness must be from 0 to 100.")?;
    Ok(Brightness::new(percent))
}

fn color(value: &Value<String>) -> Result<Rgb888, Problem> {
    hex(value.get_ref()).ok_or_else(|| {
        problem(
            value,
            "the color must be six hexadecimal digits, for example ff8c00.",
        )
    })
}

/// The color of six hexadecimal digits `RRGGBB`, or `None`.
fn hex(text: &str) -> Option<Rgb888> {
    if text.len() != 6 || !text.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let rgb = u32::from_str_radix(text, 16).ok()?;
    let [_, r, g, b] = rgb.to_be_bytes();
    Some(Rgb888::new(r, g, b))
}

/// A time `HH:MM` from `00:00` to `24:00`.
fn time(value: &Value<String>) -> Result<TimeOfDay, Problem> {
    const MESSAGE: &str = "the time must be HH:MM, from 00:00 to 24:00.";
    let text = value.get_ref();
    let parsed = text.split_once(':').and_then(|(h, m)| {
        let two_digits = |s: &str| s.len() == 2 && s.bytes().all(|b| b.is_ascii_digit());
        if !two_digits(h) || !two_digits(m) {
            return None;
        }
        TimeOfDay::hm(h.parse().ok()?, m.parse().ok()?).ok()
    });
    parsed.ok_or_else(|| problem(value, MESSAGE))
}

/// The days of a list. No list means each day.
fn days(list: Option<&Value<Vec<Value<String>>>>) -> Result<Weekdays, Problem> {
    let Some(list) = list else {
        return Ok(Weekdays::EACH_DAY);
    };
    if list.get_ref().is_empty() {
        return Err(problem(list, "the days list must have at least one day."));
    }
    let mut set = Vec::with_capacity(7);
    for day in list.get_ref() {
        let weekday = match day.get_ref().as_str() {
            "sun" => Weekday::Sunday,
            "mon" => Weekday::Monday,
            "tue" => Weekday::Tuesday,
            "wed" => Weekday::Wednesday,
            "thu" => Weekday::Thursday,
            "fri" => Weekday::Friday,
            "sat" => Weekday::Saturday,
            _ => {
                return Err(problem(
                    day,
                    "the day must be sun, mon, tue, wed, thu, fri, or sat.",
                ));
            }
        };
        set.push(weekday);
    }
    Ok(Weekdays::of(&set))
}

#[cfg(test)]
mod tests {
    use super::*;
    use gameclock_core::Brightness;
    use gameclock_core::alert::{
        Alert, Alerts, Background, ClearRule, Font, Motion, Priority, TextColor, TextStyle,
    };
    use gameclock_core::embedded_graphics::pixelcolor::Rgb888;
    use gameclock_core::schedule::Schedule;
    use jiff::tz::TimeZone;
    use jiff::{SignedDuration, Timestamp, Zoned};

    const PATH: &str = "/tmp/gc.toml";

    fn ok(text: &str) -> Tables {
        parse(PATH, text).unwrap()
    }

    fn err(text: &str) -> ConfigError {
        parse(PATH, text).unwrap_err()
    }

    fn utc(s: &str) -> Zoned {
        s.parse::<Timestamp>().unwrap().to_zoned(TimeZone::UTC)
    }

    // 2026-11-16 is a Monday. 2026-11-20 is a Friday.
    const MONDAY_2305: &str = "2026-11-16T23:05:00Z";
    const FRIDAY_2305: &str = "2026-11-20T23:05:00Z";

    // A missing file or section uses the built-in tables.

    #[test]
    fn empty_file() {
        assert_eq!(ok(""), Tables::legacy());
    }

    #[test]
    fn only_a_schedule_section() {
        let t = ok("[schedule]\ndefault = { color = \"0000ff\", brightness = 60 }\n");
        for at in [MONDAY_2305, "2026-11-16T03:00:00Z", FRIDAY_2305] {
            let style = t.schedule.style_at(&utc(at));
            assert_eq!(style.color, Rgb888::new(0, 0, 255), "{at}");
            assert_eq!(style.brightness, Brightness::new(60), "{at}");
        }
        assert_eq!(t.alerts, Alerts::legacy());
        assert_eq!(t.triggers, Tables::legacy().triggers);
        assert_ne!(t.schedule, Schedule::legacy());
    }

    // The schedule section.

    const BEDTIME_RULE: &str = r#"
[schedule]
default = { color = "ff0000", brightness = 25 }

[[schedule.rule]]
days = ["sun", "mon", "tue", "wed", "thu"]
from = "23:00"
to = "23:15"
color = "ff8c00"
brightness = 100
"#;

    #[test]
    fn rule_on_bedtime_nights() {
        let t = ok(BEDTIME_RULE);
        let monday = t.schedule.style_at(&utc(MONDAY_2305));
        assert_eq!(monday.color, Rgb888::new(255, 140, 0));
        assert_eq!(monday.brightness, Brightness::new(100));
        let friday = t.schedule.style_at(&utc(FRIDAY_2305));
        assert_eq!(friday.color, Rgb888::new(255, 0, 0));
        assert_eq!(friday.brightness, Brightness::new(25));
    }

    #[test]
    fn rule_with_no_days() {
        let text =
            BEDTIME_RULE.replace("days = [\"sun\", \"mon\", \"tue\", \"wed\", \"thu\"]\n", "");
        let t = ok(&text);
        let friday = t.schedule.style_at(&utc(FRIDAY_2305));
        assert_eq!(friday.color, Rgb888::new(255, 140, 0));
    }

    // The alert section.

    const DINNER: &str = r#"
[[alert]]
name = "dinner"
text = "DINNER"
background = "black"
font = "small"
baseline = 20
color = "ffff00"
x = 11
seconds = 5
priority = 25
clear = "removable"
"#;

    fn dinner() -> Alert {
        Alert {
            name: "dinner".to_owned(),
            background: Background::Black,
            text: "DINNER".to_owned(),
            style: TextStyle {
                font: Font::Small,
                baseline: 20,
                color: TextColor::Fixed(Rgb888::new(255, 255, 0)),
                motion: Motion::Still { x: 11 },
            },
            duration: SignedDuration::from_secs(5),
            priority: Priority(25),
            clear: ClearRule::Removable,
        }
    }

    #[test]
    fn a_new_alert_from_the_file() {
        let t = ok(DINNER);
        assert_eq!(t.alerts.find("dinner").as_deref(), Some(&dinner()));
    }

    #[test]
    fn alerts_in_the_file_replace_the_built_in_alerts() {
        let t = ok(DINNER);
        assert!(t.alerts.find("panic").is_none());
        assert_eq!(t.alerts.names().to_string(), "dinner");
        // The built-in triggers need alerts that the table does not have.
        assert_eq!(t.triggers, Vec::new());
    }

    #[test]
    fn a_scrolling_alert() {
        let text = DINNER
            .replace("x = 11\n", "passes = 3\n")
            .replace("\"black\"", "\"color-wheel\"")
            .replace("\"small\"", "\"large\"")
            .replace("\"ffff00\"", "\"contrast\"")
            .replace("\"removable\"", "\"kept\"");
        let a = ok(&text).alerts.find("dinner").unwrap();
        assert_eq!(a.background, Background::ColorWheel);
        assert_eq!(a.style.font, Font::Large);
        assert_eq!(a.style.color, TextColor::Contrast);
        assert_eq!(a.style.motion, Motion::Scroll { passes: 3 });
        assert_eq!(a.clear, ClearRule::Kept);
    }

    // The trigger section.

    #[test]
    fn trigger_from_the_file() {
        let t =
            ok("[[trigger]]\nalert = \"panic\"\nfrom = \"07:00\"\nto = \"07:01\"\nevery = 30\n");
        assert_eq!(t.triggers.len(), 1);
        let rule = &t.triggers[0];
        assert_eq!(rule.alert.name, "panic");
        assert_eq!(rule.every, SignedDuration::from_secs(30));
        let first = rule.first_at_or_after(&utc("2026-11-16T06:59:00Z"));
        assert_eq!(first, Some("2026-11-16T07:00:00Z".parse().unwrap()));
        let second = rule.first_after(&utc("2026-11-16T07:00:00Z"));
        assert_eq!(second, Some("2026-11-16T07:00:30Z".parse().unwrap()));
        assert_eq!(
            rule.first_after(&utc("2026-11-16T07:00:30Z")),
            Some("2026-11-17T07:00:00Z".parse().unwrap())
        );
    }

    #[test]
    fn trigger_for_an_unknown_alert() {
        let e =
            err("[[trigger]]\nalert = \"fire\"\nfrom = \"07:00\"\nto = \"07:01\"\nevery = 30\n");
        assert_eq!(
            e.to_string(),
            "/tmp/gc.toml:2:9: the trigger names the alert 'fire', which is not in the alert table."
        );
    }

    // The daemon checks each value.

    /// The line and the column of an error, and its message.
    fn place(e: &ConfigError) -> (usize, usize, String) {
        match e {
            ConfigError::Parse {
                line,
                column,
                message,
                ..
            } => (*line, *column, message.clone()),
        }
    }

    #[test]
    fn brightness_out_of_range() {
        let text = "[schedule.default]\ncolor = \"ff0000\"\nbrightness = 120\n";
        assert_eq!(
            err(text).to_string(),
            "/tmp/gc.toml:3:14: the brightness must be from 0 to 100."
        );
    }

    #[test]
    fn unknown_key() {
        let text = format!("{BEDTIME_RULE}colour = \"00ff00\"\n");
        let (line, column, message) = place(&err(&text));
        assert_eq!((line, column), (11, 1), "{message}");
        assert!(message.contains("unknown field `colour`"), "{message}");
    }

    #[test]
    fn toml_syntax_error() {
        let (line, column, _) = place(&err("# a comment\n[schedule\n"));
        assert_eq!(line, 2);
        assert!(column > 1, "{column}");
    }

    /// The error of the `BEDTIME_RULE` text with `from` replaced by `to`.
    fn rule_error(from: &str, to: &str) -> (usize, usize, String) {
        place(&err(&BEDTIME_RULE.replace(from, to)))
    }

    #[test]
    fn range_values_in_the_schedule() {
        let cases = [
            (
                "to = \"23:15\"",
                "to = \"25:00\"",
                (8, 6),
                "the time must be HH:MM, from 00:00 to 24:00.",
            ),
            (
                "from = \"23:00\"",
                "from = \"23:30\"",
                (8, 6),
                "the start time must be before the end time.",
            ),
            (
                "color = \"ff8c00\"",
                "color = \"green\"",
                (9, 9),
                "the color must be six hexadecimal digits, for example ff8c00.",
            ),
            (
                "\"mon\"",
                "\"monday\"",
                (6, 16),
                "the day must be sun, mon, tue, wed, thu, fri, or sat.",
            ),
            (
                "brightness = 100",
                "brightness = -1",
                (10, 14),
                "the brightness must be from 0 to 100.",
            ),
        ];
        for (from, to, at, message) in cases {
            assert_eq!(
                rule_error(from, to),
                (at.0, at.1, message.to_owned()),
                "{to}"
            );
        }
    }

    /// The error of the `DINNER` text with `from` replaced by `to`.
    fn alert_error(from: &str, to: &str) -> (usize, usize, String) {
        place(&err(&DINNER.replace(from, to)))
    }

    #[test]
    fn range_values_in_an_alert() {
        let cases = [
            (
                "name = \"dinner\"",
                "name = \"stop\"",
                (3, 8),
                AlertError::Stop.to_string(),
            ),
            (
                "text = \"DINNER\"",
                format!("text = \"{}\"", "a".repeat(65)).leak() as &str,
                (4, 8),
                AlertError::Text.to_string(),
            ),
            (
                "x = 11",
                "passes = 0",
                (9, 10),
                AlertError::Passes.to_string(),
            ),
            (
                "seconds = 5",
                "seconds = 0",
                (10, 11),
                AlertError::Duration.to_string(),
            ),
            (
                "x = 11",
                "x = 11\npasses = 2",
                (3, 8),
                "an alert must have x or passes, but not both.".to_owned(),
            ),
            (
                "\"black\"",
                "\"blue\"",
                (5, 14),
                "the background must be black or color-wheel.".to_owned(),
            ),
        ];
        for (from, to, at, message) in cases {
            assert_eq!(alert_error(from, to), (at.0, at.1, message), "{to}");
        }
    }

    #[test]
    fn two_alerts_with_one_name() {
        let text = format!("{DINNER}{DINNER}");
        assert_eq!(
            place(&err(&text)),
            (15, 8, "two alerts have the name 'dinner'.".to_owned())
        );
    }

    #[test]
    fn trigger_period_out_of_range() {
        let text = "[[trigger]]\nalert = \"panic\"\nfrom = \"07:00\"\nto = \"07:01\"\nevery = 0\n";
        assert_eq!(
            place(&err(text)),
            (
                5,
                9,
                "the trigger period must be from 1 to 3600 seconds.".to_owned()
            )
        );
    }

    // The example file gives the legacy tables.

    #[test]
    fn the_example_file_gives_the_legacy_tables() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../deploy/config.toml");
        let text = std::fs::read_to_string(path).expect("deploy/config.toml");
        assert_eq!(parse(path, &text).unwrap(), Tables::legacy());
    }
}
