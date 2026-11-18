//! The data pages: `weather`, `indoor`, and `outdoor`
//! (`openspec/specs/sensor-pages`, docs/adr/0021).
//!
//! The daemon fetches and parses the data. This module gets typed values and
//! the time, and makes the text and the scene of a page.

use embedded_graphics::pixelcolor::Rgb888;
use jiff::{SignedDuration, Timestamp};

/// The name of a page.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PageName {
    /// The NWS forecast.
    Weather,
    /// The indoor sensor.
    Indoor,
    /// The outdoor sensor.
    Outdoor,
}

impl PageName {
    /// All pages, in the order of the web page buttons.
    pub const ALL: [Self; 3] = [Self::Weather, Self::Indoor, Self::Outdoor];

    /// The name in commands and in the config.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Weather => "weather",
            Self::Indoor => "indoor",
            Self::Outdoor => "outdoor",
        }
    }

    /// The page with the name `name`.
    #[must_use]
    pub fn parse(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|page| page.name() == name)
    }
}

/// A reading of a sensor page is old after this time, and shows dashes.
pub const MAX_AGE: SignedDuration = SignedDuration::from_mins(15);

/// The most characters of a forecast text.
pub const MAX_TEXT: usize = 64;

/// The newest reading of an enviro sensor.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Reading {
    /// The time of the reading.
    pub time: Timestamp,
    /// The temperature in °F.
    pub temp_f: f64,
    /// The relative humidity in percent.
    pub humidity: f64,
    /// PM2.5 in µg/m³.
    pub p25: f64,
    /// PM10 in µg/m³.
    pub p10: f64,
}

/// One period of a forecast.
#[derive(Debug, Clone, PartialEq)]
pub struct Period {
    /// The end of the period.
    pub end: Timestamp,
    /// The temperature in °F.
    pub temp_f: f64,
    /// The chance of rain in percent.
    pub rain: u8,
    /// The short forecast text.
    pub text: String,
}

/// A forecast: its periods in time order.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Forecast {
    /// The periods.
    pub periods: Vec<Period>,
}

/// The data of a page. `None` means that the daemon has no data.
#[derive(Debug, Clone, PartialEq)]
pub enum PageData {
    /// The data of `indoor` or `outdoor`.
    Reading(Option<Reading>),
    /// The data of `weather`.
    Forecast(Option<Forecast>),
}

/// The text lines of a page. `aqi` is the AQI on the last line, if the page
/// shows one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageText {
    /// The lines, from the top.
    pub lines: Vec<String>,
    /// The AQI value of the last line.
    pub aqi: Option<u16>,
}

/// The text of `page` with `data` at `now`.
#[must_use]
pub fn page_text(page: PageName, data: &PageData, now: Timestamp) -> PageText {
    match (page, data) {
        (PageName::Weather, PageData::Forecast(forecast)) => {
            weather_text(forecast.as_ref().and_then(|f| f.current(now)))
        }
        (PageName::Weather, PageData::Reading(_)) => weather_text(None),
        (PageName::Indoor | PageName::Outdoor, PageData::Reading(reading)) => {
            let fresh = reading.filter(|r| now.duration_since(r.time) <= MAX_AGE);
            reading_text(page, fresh)
        }
        (PageName::Indoor | PageName::Outdoor, PageData::Forecast(_)) => reading_text(page, None),
    }
}

impl Forecast {
    /// The first period that has not ended at `now`.
    #[must_use]
    pub fn current(&self, now: Timestamp) -> Option<&Period> {
        self.periods.iter().find(|p| p.end > now)
    }
}

/// The place word of a sensor page.
fn place(page: PageName) -> &'static str {
    match page {
        PageName::Indoor => "IN",
        PageName::Outdoor | PageName::Weather => "OUT",
    }
}

/// The lines of a sensor page. No reading gives dashes.
fn reading_text(page: PageName, reading: Option<Reading>) -> PageText {
    let place = place(page);
    let Some(r) = reading else {
        return PageText {
            lines: vec![
                format!("{place} --"),
                "RH --".to_owned(),
                "AQI --".to_owned(),
            ],
            aqi: None,
        };
    };
    let value = aqi(r.p25, r.p10);
    PageText {
        lines: vec![
            format!("{place} {}F", whole(r.temp_f)),
            format!("RH {}%", whole(r.humidity)),
            format!("AQI {value}"),
        ],
        aqi: Some(value),
    }
}

/// The lines of the weather page. No period gives dashes.
fn weather_text(period: Option<&Period>) -> PageText {
    let lines = match period {
        Some(p) => vec![format!("{}F {}%", whole(p.temp_f), p.rain), clean(&p.text)],
        None => vec!["--F --%".to_owned(), "--".to_owned()],
    };
    PageText { lines, aqi: None }
}

/// `text` with at most [`MAX_TEXT`] characters. Each character that is not
/// printable ASCII becomes `?`.
fn clean(text: &str) -> String {
    text.chars()
        .take(MAX_TEXT)
        .map(|c| {
            if c == ' ' || c.is_ascii_graphic() {
                c
            } else {
                '?'
            }
        })
        .collect()
}

/// `value` rounded to the nearest integer. Half goes away from zero.
fn whole(value: f64) -> i64 {
    let value = value.round();
    if value.is_nan() {
        return 0;
    }
    // A sensor value is far inside the range of i64. `as` saturates.
    #[expect(clippy::cast_possible_truncation, reason = "sensor values are small")]
    let whole = value as i64;
    // `as` gives 0 for -0.0, so "-0F" cannot show.
    whole
}

/// The US EPA AQI for PM2.5 `p25` and PM10 `p10`, in µg/m³. It is the
/// higher of the two values, and 500 at most.
#[must_use]
pub fn aqi(p25: f64, p10: f64) -> u16 {
    // PM2.5 in units of 0.1 µg/m³, and PM10 in units of 1 µg/m³.
    let p25 = index(&PM25, truncate(p25 * 10.0));
    let p10 = index(&PM10, truncate(p10));
    p25.max(p10)
}

/// The color of the EPA category of `aqi`.
#[must_use]
pub fn aqi_color(aqi: u16) -> Rgb888 {
    CATEGORIES
        .iter()
        .find(|(top, _)| aqi <= *top)
        .map_or(MAROON, |(_, color)| *color)
}

/// One row of an EPA table: the low and high concentration, and the low and
/// high AQI.
type Breakpoint = (i64, i64, u16, u16);

/// PM2.5 in units of 0.1 µg/m³ (EPA, 2024).
const PM25: [Breakpoint; 6] = [
    (0, 90, 0, 50),
    (91, 354, 51, 100),
    (355, 554, 101, 150),
    (555, 1254, 151, 200),
    (1255, 2254, 201, 300),
    (2255, 3254, 301, 500),
];

/// PM10 in units of 1 µg/m³ (EPA).
const PM10: [Breakpoint; 6] = [
    (0, 54, 0, 50),
    (55, 154, 51, 100),
    (155, 254, 101, 150),
    (255, 354, 151, 200),
    (355, 424, 201, 300),
    (425, 604, 301, 500),
];

/// The highest AQI and the color of each category.
const CATEGORIES: [(u16, Rgb888); 5] = [
    (50, Rgb888::new(0, 228, 0)),
    (100, Rgb888::new(255, 255, 0)),
    (150, Rgb888::new(255, 126, 0)),
    (200, Rgb888::new(255, 0, 0)),
    (300, Rgb888::new(143, 63, 151)),
];

/// The color of the hazardous category.
const MAROON: Rgb888 = Rgb888::new(126, 0, 35);

/// The highest AQI.
const AQI_MAX: u16 = 500;

/// `value` without its fraction, and 0 for a negative value or no number.
fn truncate(value: f64) -> i64 {
    // The small addition keeps a product such as 2.9999999 at 3.
    let value = (value + 1e-9).floor();
    if value.is_nan() || value < 0.0 {
        return 0;
    }
    if value > 1e9 {
        return i64::from(u32::MAX);
    }
    // The checks above keep the value in the range of i64.
    #[expect(
        clippy::cast_possible_truncation,
        reason = "the value is from 0 to 1e9"
    )]
    let whole = value as i64;
    whole
}

/// The AQI of the concentration `c` in `table`.
fn index(table: &[Breakpoint], c: i64) -> u16 {
    let Some(&(c_low, c_high, i_low, i_high)) = table.iter().find(|(_, high, _, _)| c <= *high)
    else {
        return AQI_MAX;
    };
    let span = f64::from(i_high - i_low) / to_f64(c_high - c_low);
    let value = (span * to_f64(c - c_low)).round();
    // The value is from 0 to the span of one row, so it fits in u16.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "0 to 200"
    )]
    let step = value as u16;
    i_low + step
}

/// `value` as f64. The table values are small, so no precision is lost.
fn to_f64(value: i64) -> f64 {
    #[expect(clippy::cast_precision_loss, reason = "the values are below 2^52")]
    let value = value as f64;
    value
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: &str = "2026-11-17T20:00:00Z";

    fn now() -> Timestamp {
        NOW.parse().unwrap()
    }

    fn ago(mins: i64) -> Timestamp {
        now() - SignedDuration::from_mins(mins)
    }

    fn reading(mins_ago: i64, temp_f: f64, humidity: f64, p25: f64, p10: f64) -> PageData {
        PageData::Reading(Some(Reading {
            time: ago(mins_ago),
            temp_f,
            humidity,
            p25,
            p10,
        }))
    }

    fn lines(page: PageName, data: &PageData) -> Vec<String> {
        page_text(page, data, now()).lines
    }

    fn period(ends_in_mins: i64, temp_f: f64, rain: u8, text: &str) -> Period {
        Period {
            end: now() + SignedDuration::from_mins(ends_in_mins),
            temp_f,
            rain,
            text: text.to_owned(),
        }
    }

    fn forecast(periods: Vec<Period>) -> PageData {
        PageData::Forecast(Some(Forecast { periods }))
    }

    #[test]
    fn outdoor_page() {
        let data = reading(2, 41.51, 92.15, 5.02, 5.86);
        let text = page_text(PageName::Outdoor, &data, now());
        assert_eq!(text.lines, ["OUT 42F", "RH 92%", "AQI 28"]);
        assert_eq!(text.aqi, Some(28));
    }

    #[test]
    fn indoor_page() {
        let data = reading(1, 67.79, 44.26, 1.23, 1.23);
        assert_eq!(
            lines(PageName::Indoor, &data),
            ["IN 68F", "RH 44%", "AQI 7"]
        );
    }

    #[test]
    fn below_zero() {
        let data = reading(1, -3.4, 50.0, 1.0, 1.0);
        assert_eq!(lines(PageName::Outdoor, &data)[0], "OUT -3F");
        let data = reading(1, -0.4, 50.0, 1.0, 1.0);
        assert_eq!(lines(PageName::Outdoor, &data)[0], "OUT 0F");
    }

    #[test]
    fn old_reading() {
        let data = reading(20, 67.79, 44.26, 1.23, 1.23);
        let text = page_text(PageName::Indoor, &data, now());
        assert_eq!(text.lines, ["IN --", "RH --", "AQI --"]);
        assert_eq!(text.aqi, None);
        // 15 min is not old yet.
        let data = reading(15, 67.79, 44.26, 1.23, 1.23);
        assert_eq!(lines(PageName::Indoor, &data)[0], "IN 68F");
    }

    #[test]
    fn no_reading() {
        let data = PageData::Reading(None);
        assert_eq!(
            lines(PageName::Outdoor, &data),
            ["OUT --", "RH --", "AQI --"]
        );
    }

    #[test]
    fn weather_page() {
        let data = forecast(vec![period(60, 48.0, 20, "Rain")]);
        let text = page_text(PageName::Weather, &data, now());
        assert_eq!(text.lines, ["48F 20%", "Rain"]);
        assert_eq!(text.aqi, None);
    }

    #[test]
    fn no_chance_of_rain() {
        let data = forecast(vec![period(60, 48.0, 0, "Sunny")]);
        assert_eq!(lines(PageName::Weather, &data)[0], "48F 0%");
    }

    #[test]
    fn skip_an_old_period() {
        let data = forecast(vec![
            period(-60, 40.0, 90, "Rain"),
            period(60, 45.0, 10, "Cloudy"),
        ]);
        assert_eq!(lines(PageName::Weather, &data), ["45F 10%", "Cloudy"]);
    }

    #[test]
    fn no_current_period() {
        let data = forecast(vec![period(-60, 40.0, 90, "Rain")]);
        assert_eq!(lines(PageName::Weather, &data), ["--F --%", "--"]);
        let data = PageData::Forecast(None);
        assert_eq!(lines(PageName::Weather, &data), ["--F --%", "--"]);
    }

    #[test]
    fn text_that_is_not_ascii() {
        let data = forecast(vec![period(60, 48.0, 20, "Fog\u{b0}")]);
        assert_eq!(lines(PageName::Weather, &data)[1], "Fog?");
        let long = "a".repeat(80);
        let data = forecast(vec![period(60, 48.0, 20, &long)]);
        assert_eq!(lines(PageName::Weather, &data)[1].len(), MAX_TEXT);
    }

    #[test]
    fn data_of_the_wrong_kind() {
        let data = forecast(vec![period(60, 48.0, 20, "Rain")]);
        assert_eq!(lines(PageName::Indoor, &data), ["IN --", "RH --", "AQI --"]);
    }

    #[test]
    fn aqi_values() {
        let cases = [
            ((5.02, 5.86), 28),
            ((1.23, 1.23), 7),
            ((40.0, 20.0), 112),
            ((20.0, 200.0), 123),
            ((600.0, 0.0), 500),
        ];
        for ((p25, p10), expected) in cases {
            assert_eq!(aqi(p25, p10), expected, "P25 {p25}, P10 {p10}");
        }
    }

    #[test]
    fn aqi_edges() {
        assert_eq!(aqi(0.0, 0.0), 0);
        assert_eq!(aqi(9.0, 0.0), 50);
        assert_eq!(aqi(9.1, 0.0), 51);
        // PM2.5 is truncated to 0.1, so 9.09 is 9.0.
        assert_eq!(aqi(9.09, 0.0), 50);
        assert_eq!(aqi(325.4, 0.0), 500);
        assert_eq!(aqi(0.0, 54.9), 50);
        assert_eq!(aqi(0.0, 604.0), 500);
        assert_eq!(aqi(0.0, 9999.0), 500);
        assert_eq!(aqi(-1.0, -1.0), 0);
    }

    #[test]
    fn category_colors() {
        let cases = [
            (28, Rgb888::new(0, 228, 0)),
            (75, Rgb888::new(255, 255, 0)),
            (112, Rgb888::new(255, 126, 0)),
            (175, Rgb888::new(255, 0, 0)),
            (250, Rgb888::new(143, 63, 151)),
            (400, Rgb888::new(126, 0, 35)),
        ];
        for (value, color) in cases {
            assert_eq!(aqi_color(value), color, "AQI {value}");
        }
        assert_eq!(aqi_color(50), Rgb888::new(0, 228, 0));
        assert_eq!(aqi_color(51), Rgb888::new(255, 255, 0));
    }

    #[test]
    fn page_names() {
        for page in PageName::ALL {
            assert_eq!(PageName::parse(page.name()), Some(page));
        }
        assert_eq!(PageName::Weather.name(), "weather");
        assert_eq!(PageName::Indoor.name(), "indoor");
        assert_eq!(PageName::Outdoor.name(), "outdoor");
        assert_eq!(PageName::parse("garage"), None);
        assert_eq!(PageName::parse("Outdoor"), None);
    }
}
