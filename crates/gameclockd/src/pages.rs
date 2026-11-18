//! The page data: the parse of each JSON body into core values
//! (`openspec/specs/sensor-pages`, docs/adr/0021).
//!
//! These are pure functions of the body and the time. The `fetch` thread gets
//! the body.

use gameclock_core::page::{Forecast, MAX_AGE, PageData, PageName, Period, Reading};
use jiff::Timestamp;
use serde::Deserialize;

/// The data of `page` from the result of its fetch at `now`. The error is
/// the reason for a warning line. The page then shows dashes.
///
/// # Errors
/// The reason, if the fetch failed, if the body is not JSON of the expected
/// form, or if a reading is older than [`MAX_AGE`].
pub fn page_data(
    page: PageName,
    body: Result<&[u8], &str>,
    now: Timestamp,
) -> (PageData, Option<String>) {
    let result = body.map_err(str::to_owned).and_then(|body| match page {
        PageName::Weather => forecast(body).map(|f| PageData::Forecast(Some(f))),
        PageName::Indoor | PageName::Outdoor => {
            reading(body, now).map(|r| PageData::Reading(Some(r)))
        }
    });
    match result {
        Ok(data) => (data, None),
        Err(reason) => (no_data(page), Some(reason)),
    }
}

/// The data of `page` when the daemon has none.
fn no_data(page: PageName) -> PageData {
    match page {
        PageName::Weather => PageData::Forecast(None),
        PageName::Indoor | PageName::Outdoor => PageData::Reading(None),
    }
}

/// The text of a body that is not JSON of the expected form.
const BAD_BODY: &str = "the body is not JSON of the expected form";

#[derive(Debug, Deserialize)]
struct Realtime {
    data: Vec<Row>,
}

#[derive(Debug, Deserialize)]
struct Row {
    time: i64,
    output_temp: f64,
    output_humidity: f64,
    #[serde(rename = "P25")]
    p25: f64,
    #[serde(rename = "P10")]
    p10: f64,
}

/// The newest reading of a `realtime.json` body, if it is not old at `now`.
fn reading(body: &[u8], now: Timestamp) -> Result<Reading, String> {
    let realtime: Realtime = serde_json::from_slice(body).map_err(|_| BAD_BODY.to_owned())?;
    let row = realtime.data.first().ok_or_else(|| BAD_BODY.to_owned())?;
    let time = Timestamp::from_second(row.time).map_err(|_| BAD_BODY.to_owned())?;
    let age = now.duration_since(time);
    if age > MAX_AGE {
        return Err(format!("the reading is {} min old", age.as_mins()));
    }
    Ok(Reading {
        time,
        temp_f: row.output_temp,
        humidity: row.output_humidity,
        p25: row.p25,
        p10: row.p10,
    })
}

#[derive(Debug, Deserialize)]
struct Nws {
    properties: Properties,
}

#[derive(Debug, Deserialize)]
struct Properties {
    periods: Vec<NwsPeriod>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct NwsPeriod {
    end_time: String,
    temperature: f64,
    temperature_unit: String,
    probability_of_precipitation: Option<Chance>,
    short_forecast: String,
}

#[derive(Debug, Deserialize)]
struct Chance {
    value: Option<f64>,
}

/// The periods of an NWS forecast body, with each temperature in °F.
fn forecast(body: &[u8]) -> Result<Forecast, String> {
    let nws: Nws = serde_json::from_slice(body).map_err(|_| BAD_BODY.to_owned())?;
    let periods = nws
        .properties
        .periods
        .into_iter()
        .map(period)
        .collect::<Option<Vec<_>>>()
        .ok_or_else(|| BAD_BODY.to_owned())?;
    Ok(Forecast { periods })
}

/// One period in core values. `None` for a time or a unit that is not
/// correct.
fn period(p: NwsPeriod) -> Option<Period> {
    let end: Timestamp = p.end_time.parse().ok()?;
    let temp_f = match p.temperature_unit.as_str() {
        "F" => p.temperature,
        "C" => p.temperature * 9.0 / 5.0 + 32.0,
        _ => return None,
    };
    let chance = p
        .probability_of_precipitation
        .and_then(|c| c.value)
        .unwrap_or(0.0)
        .round()
        .clamp(0.0, 100.0);
    // The clamp keeps the value from 0 to 100.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "0 to 100"
    )]
    let rain = chance as u8;
    Some(Period {
        end,
        temp_f,
        rain,
        text: p.short_forecast,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn now() -> Timestamp {
        "2026-11-17T20:00:00Z".parse().unwrap()
    }

    /// A reading body with its time `mins` minutes before `now`.
    fn body(mins: i64) -> String {
        let time = now().as_second() - mins * 60;
        format!(
            r#"{{"data":[{{"time":{time},"run_time":174997,"raw_temp":46.74,"output_temp":41.51,"raw_humidity":75.48,"output_humidity":92.15,"output_bar":1010.4,"P25":5.02,"P10":5.86,"p1":3.78}}]}}"#
        )
    }

    fn nws(periods: &str) -> String {
        format!(r#"{{"type":"Feature","properties":{{"units":"us","periods":[{periods}]}}}}"#)
    }

    fn nws_period(end: &str, temp: i32, unit: &str, chance: &str, text: &str) -> String {
        format!(
            r#"{{"number":1,"name":"Tonight","endTime":"{end}","temperature":{temp},"temperatureUnit":"{unit}","probabilityOfPrecipitation":{{"unitCode":"wmoUnit:percent","value":{chance}}},"shortForecast":"{text}","detailedForecast":"..."}}"#
        )
    }

    #[test]
    fn a_reading() {
        let (data, warning) = page_data(PageName::Outdoor, Ok(body(2).as_bytes()), now());
        assert_eq!(warning, None);
        let PageData::Reading(Some(r)) = data else {
            panic!("{data:?}");
        };
        assert_eq!(r.temp_f, 41.51);
        assert_eq!(r.humidity, 92.15);
        assert_eq!((r.p25, r.p10), (5.02, 5.86));
    }

    #[test]
    fn an_old_reading() {
        let (data, warning) = page_data(PageName::Indoor, Ok(body(20).as_bytes()), now());
        assert_eq!(data, PageData::Reading(None));
        assert_eq!(warning.as_deref(), Some("the reading is 20 min old"));
    }

    #[test]
    fn a_reading_with_a_missing_field() {
        let text = body(2).replace(r#""P25":5.02,"#, "");
        let (data, warning) = page_data(PageName::Indoor, Ok(text.as_bytes()), now());
        assert_eq!(data, PageData::Reading(None));
        assert_eq!(warning.as_deref(), Some(BAD_BODY));
        let (_, warning) = page_data(PageName::Indoor, Ok(br#"{"data":[]}"#), now());
        assert_eq!(warning.as_deref(), Some(BAD_BODY));
    }

    #[test]
    fn a_failed_fetch() {
        let (data, warning) = page_data(PageName::Outdoor, Err("curl: (28) timeout"), now());
        assert_eq!(data, PageData::Reading(None));
        assert_eq!(warning.as_deref(), Some("curl: (28) timeout"));
    }

    #[test]
    fn a_forecast() {
        let text = nws(&[
            nws_period("2026-11-17T18:00:00-08:00", 48, "F", "20", "Rain"),
            nws_period("2026-11-18T06:00:00-08:00", 40, "F", "null", "Cloudy"),
        ]
        .join(","));
        let (data, warning) = page_data(PageName::Weather, Ok(text.as_bytes()), now());
        assert_eq!(warning, None);
        let PageData::Forecast(Some(f)) = data else {
            panic!("{data:?}");
        };
        assert_eq!(f.periods.len(), 2);
        assert_eq!(f.periods[0].end, "2026-11-18T02:00:00Z".parse().unwrap());
        assert_eq!((f.periods[0].temp_f, f.periods[0].rain), (48.0, 20));
        assert_eq!(f.periods[0].text, "Rain");
        assert_eq!(f.periods[1].rain, 0, "no chance of rain");
    }

    #[test]
    fn celsius_from_the_service() {
        let text = nws(&nws_period(
            "2026-11-17T18:00:00-08:00",
            10,
            "C",
            "0",
            "Fog",
        ));
        let (data, _) = page_data(PageName::Weather, Ok(text.as_bytes()), now());
        let PageData::Forecast(Some(f)) = data else {
            panic!("{data:?}");
        };
        assert_eq!(f.periods[0].temp_f, 50.0);
    }

    #[test]
    fn a_body_that_is_not_json() {
        let (data, warning) = page_data(PageName::Weather, Ok(b"<html>"), now());
        assert_eq!(data, PageData::Forecast(None));
        assert_eq!(warning.as_deref(), Some(BAD_BODY));
        let text = nws(&nws_period("not a time", 10, "F", "0", "Fog"));
        let (_, warning) = page_data(PageName::Weather, Ok(text.as_bytes()), now());
        assert_eq!(warning.as_deref(), Some(BAD_BODY));
        let text = nws(&nws_period(
            "2026-11-17T18:00:00-08:00",
            10,
            "K",
            "0",
            "Fog",
        ));
        let (_, warning) = page_data(PageName::Weather, Ok(text.as_bytes()), now());
        assert_eq!(warning.as_deref(), Some(BAD_BODY));
    }
}
