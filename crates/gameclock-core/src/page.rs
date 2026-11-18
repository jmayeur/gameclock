//! The data pages: `weather`, `indoor`, and `outdoor`
//! (`openspec/specs/sensor-pages`, docs/adr/0021).
//!
//! The daemon fetches and parses the data. This module gets typed values and
//! the time, and makes the text and the scene of a page.

use embedded_graphics::mono_font::ascii::{FONT_6X10, FONT_7X13_BOLD};
use embedded_graphics::mono_font::{MonoFont, MonoTextStyle};
use embedded_graphics::pixelcolor::Rgb888;
use embedded_graphics::prelude::*;
use embedded_graphics::text::{Baseline, Text};
use jiff::{SignedDuration, Timestamp, Zoned};

use crate::message::ScrollPath;
use crate::schedule::Schedule;
use crate::{Brightness, Frame, Scene};

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

/// A page shows for this time after its data arrives.
pub const DURATION: SignedDuration = SignedDuration::from_secs(30);

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

/// The scene of a page. It makes its text once, when it starts, and ends
/// [`DURATION`] after its start.
#[derive(Debug, Clone)]
pub struct PageScene {
    page: PageName,
    text: PageText,
    end: Timestamp,
    schedule: Schedule,
    /// The text of the AQI value on the last line, made once.
    aqi_value: String,
    /// The x of the second weather line when it is still.
    still_x: i32,
    /// The path of the second weather line when it is too wide.
    scroll: Option<ScrollPath>,
}

/// The small font of the sensor pages.
const SMALL: &MonoFont<'static> = &FONT_6X10;

/// The font of the weather page.
const LARGE: &MonoFont<'static> = &FONT_7X13_BOLD;

/// The top of each line of a sensor page.
const SMALL_TOPS: [i32; 3] = [0, 11, 22];

/// The top of each line of the weather page.
const LARGE_TOPS: [i32; 2] = [1, 17];

/// The x of the left side of each line.
const LEFT: i32 = 1;

/// The word before the AQI value.
const AQI_WORD: &str = "AQI ";

impl PageScene {
    /// The scene of `page` with `data`, from `start`, on a panel `width`
    /// pixels wide. It takes its colors and brightness from `schedule`.
    #[must_use]
    pub fn new(
        page: PageName,
        data: &PageData,
        start: Timestamp,
        width: usize,
        schedule: Schedule,
    ) -> Self {
        let text = page_text(page, data, start);
        let end = start.saturating_add(DURATION).unwrap_or(start);
        let aqi_value = text.aqi.map(|v| v.to_string()).unwrap_or_default();
        let (still_x, scroll) = match (page, text.lines.get(1)) {
            (PageName::Weather, Some(line)) => second_line(line, start, width),
            _ => (LEFT, None),
        };
        Self {
            page,
            text,
            end,
            schedule,
            aqi_value,
            still_x,
            scroll,
        }
    }

    /// The name of the page.
    #[must_use]
    pub fn page(&self) -> PageName {
        self.page
    }

    fn render_reading(&self, color: Rgb888, frame: &mut Frame) {
        for (line, top) in self.text.lines.iter().zip(SMALL_TOPS) {
            match self.text.aqi {
                Some(value) if line.starts_with(AQI_WORD) => {
                    draw(AQI_WORD, SMALL, color, Point::new(LEFT, top), frame);
                    let x = LEFT + width_of(AQI_WORD, SMALL);
                    draw(
                        &self.aqi_value,
                        SMALL,
                        aqi_color(value),
                        Point::new(x, top),
                        frame,
                    );
                }
                _ => draw(line, SMALL, color, Point::new(LEFT, top), frame),
            }
        }
    }

    fn render_weather(&self, now: &Zoned, color: Rgb888, frame: &mut Frame) {
        let [first, second] = LARGE_TOPS;
        if let Some(line) = self.text.lines.first() {
            draw(line, LARGE, color, Point::new(LEFT, first), frame);
        }
        let x = match &self.scroll {
            Some(path) => path.x(now),
            None => Some(self.still_x),
        };
        if let (Some(line), Some(x)) = (self.text.lines.get(1), x) {
            draw(line, LARGE, color, Point::new(x, second), frame);
        }
    }
}

/// The still x, or the scroll path, of the second weather line.
fn second_line(line: &str, start: Timestamp, width: usize) -> (i32, Option<ScrollPath>) {
    let panel = i32::try_from(width).unwrap_or(i32::MAX);
    let text = width_of(line, LARGE);
    if text <= panel {
        return ((panel - text) / 2, None);
    }
    // Enough passes to scroll until the page ends.
    let glyph = LARGE.character_size.width + LARGE.character_spacing;
    let chars = line.chars().count();
    let one = ScrollPath::new(start, width, chars, glyph, 1);
    let pass_ms = one
        .finish()
        .map_or(1, |end| end.duration_since(start).as_millis().max(1));
    let passes = (DURATION.as_millis() + pass_ms - 1) / pass_ms;
    let passes = u8::try_from(passes).unwrap_or(u8::MAX);
    (
        LEFT,
        Some(ScrollPath::new(start, width, chars, glyph, passes)),
    )
}

/// The width in pixels of `text` in `font`.
fn width_of(text: &str, font: &MonoFont<'_>) -> i32 {
    let glyph = font.character_size.width + font.character_spacing;
    let chars = u32::try_from(text.chars().count()).unwrap_or(u32::MAX);
    i32::try_from(glyph.saturating_mul(chars)).unwrap_or(i32::MAX)
}

/// Draw `text` with its top left at `at`.
fn draw(text: &str, font: &MonoFont<'_>, color: Rgb888, at: Point, frame: &mut Frame) {
    let style = MonoTextStyle::new(font, color);
    match Text::with_baseline(text, at, style, Baseline::Top).draw(frame) {
        Ok(_) => {}
        Err(never) => match never {},
    }
}

impl Scene for PageScene {
    fn render(&mut self, now: &Zoned, frame: &mut Frame) {
        let color = self.schedule.style_at(now).color;
        match self.page {
            PageName::Weather => self.render_weather(now, color, frame),
            PageName::Indoor | PageName::Outdoor => self.render_reading(color, frame),
        }
    }

    /// The next scroll step, or the end of the page.
    fn next_change(&self, now: &Zoned) -> Option<Timestamp> {
        if self.is_finished(now) {
            return None;
        }
        let step = self.scroll.as_ref().and_then(|path| path.next_step(now));
        Some(step.map_or(self.end, |step| step.min(self.end)))
    }

    fn is_finished(&self, now: &Zoned) -> bool {
        now.timestamp() >= self.end
    }

    fn brightness(&self, now: &Zoned) -> Brightness {
        self.schedule.style_at(now).brightness
    }
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

    // The scene tests use the legacy schedule: blue at 60 at 12:00, and red
    // at 25 at 02:00.
    const NOON: &str = "2026-11-17T12:00:00Z";
    const BLUE: Rgb888 = Rgb888::new(0, 0, 255);
    const RED: Rgb888 = Rgb888::new(255, 0, 0);
    const GREEN: Rgb888 = Rgb888::new(0, 228, 0);

    fn zoned(t: Timestamp, ms: i64) -> Zoned {
        (t + SignedDuration::from_millis(ms)).to_zoned(jiff::tz::TimeZone::UTC)
    }

    fn scene_at(page: PageName, data: &PageData, start: &str) -> (PageScene, Timestamp) {
        let start: Timestamp = start.parse().unwrap();
        (
            PageScene::new(page, data, start, 64, Schedule::legacy()),
            start,
        )
    }

    fn render(scene: &mut PageScene, now: &Zoned) -> Frame {
        let mut frame = Frame::new(64, 32);
        scene.render(now, &mut frame);
        frame
    }

    /// The colors in rows `rows` of `frame`, other than black.
    fn colors(frame: &Frame, rows: core::ops::Range<i32>) -> Vec<Rgb888> {
        let mut found = Vec::new();
        for y in rows {
            for x in 0..64 {
                let c = frame
                    .pixel(embedded_graphics::prelude::Point::new(x, y))
                    .unwrap();
                if c != Rgb888::new(0, 0, 0) && !found.contains(&c) {
                    found.push(c);
                }
            }
        }
        found
    }

    /// The x of the leftmost lit pixel in rows `rows`.
    fn leftmost(frame: &Frame, rows: core::ops::Range<i32>) -> Option<i32> {
        (0..64).find(|&x| {
            rows.clone().any(|y| {
                frame
                    .pixel(embedded_graphics::prelude::Point::new(x, y))
                    .unwrap()
                    != Rgb888::new(0, 0, 0)
            })
        })
    }

    fn noon_reading() -> PageData {
        PageData::Reading(Some(Reading {
            time: NOON.parse::<Timestamp>().unwrap() - SignedDuration::from_mins(2),
            temp_f: 41.51,
            humidity: 92.15,
            p25: 5.02,
            p10: 5.86,
        }))
    }

    fn noon_forecast(text: &str) -> PageData {
        let end = NOON.parse::<Timestamp>().unwrap() + SignedDuration::from_hours(6);
        PageData::Forecast(Some(Forecast {
            periods: vec![Period {
                end,
                temp_f: 48.0,
                rain: 20,
                text: text.to_owned(),
            }],
        }))
    }

    #[test]
    fn page_ends_after_30_s() {
        let (scene, t) = scene_at(PageName::Outdoor, &noon_reading(), NOON);
        assert!(!scene.is_finished(&zoned(t, 29_999)));
        assert!(scene.is_finished(&zoned(t, 30_000)));
        assert_eq!(scene.next_change(&zoned(t, 0)), Some(t + DURATION));
        assert_eq!(scene.page(), PageName::Outdoor);
    }

    #[test]
    fn reading_page_has_text_and_the_aqi_color() {
        let (mut scene, t) = scene_at(PageName::Outdoor, &noon_reading(), NOON);
        let frame = render(&mut scene, &zoned(t, 0));
        assert_eq!(colors(&frame, 0..22), [BLUE]);
        assert!(colors(&frame, 22..32).contains(&BLUE), "the AQI word");
        assert!(colors(&frame, 22..32).contains(&GREEN), "the AQI value");
        assert_eq!(scene.brightness(&zoned(t, 0)), Brightness::new(60));
    }

    #[test]
    fn page_at_night() {
        let t: Timestamp = "2026-11-17T02:00:00Z".parse().unwrap();
        let data = PageData::Reading(Some(Reading {
            time: t,
            temp_f: 67.79,
            humidity: 44.26,
            p25: 1.23,
            p10: 1.23,
        }));
        let mut scene = PageScene::new(PageName::Indoor, &data, t, 64, Schedule::legacy());
        let frame = render(&mut scene, &zoned(t, 0));
        assert_eq!(scene.brightness(&zoned(t, 0)), Brightness::new(25));
        assert_eq!(colors(&frame, 0..22), [RED]);
    }

    #[test]
    fn short_text_is_still() {
        let (mut scene, t) = scene_at(PageName::Weather, &noon_forecast("Rain"), NOON);
        let first = render(&mut scene, &zoned(t, 1000));
        assert!(!colors(&first, 17..32).is_empty(), "the text shows");
        assert_eq!(first, render(&mut scene, &zoned(t, 20_000)));
        assert_eq!(scene.next_change(&zoned(t, 1000)), Some(t + DURATION));
    }

    #[test]
    fn long_text_scrolls() {
        let text = "Chance Rain Showers then Mostly Cloudy";
        let (mut scene, t) = scene_at(PageName::Weather, &noon_forecast(text), NOON);
        let a = leftmost(&render(&mut scene, &zoned(t, 1000)), 17..32).unwrap();
        let b = leftmost(&render(&mut scene, &zoned(t, 1400)), 17..32).unwrap();
        assert_eq!(a - b, 10);
        // The first line does not move.
        assert_eq!(
            colors(&render(&mut scene, &zoned(t, 1000)), 0..16),
            colors(&render(&mut scene, &zoned(t, 1400)), 0..16),
        );
        assert_eq!(
            scene.next_change(&zoned(t, 1000)),
            Some(t + SignedDuration::from_millis(1040))
        );
        // The text continues until the page ends.
        assert!(leftmost(&render(&mut scene, &zoned(t, 29_000)), 17..32).is_some());
        assert_eq!(scene.next_change(&zoned(t, 29_990)), Some(t + DURATION));
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
