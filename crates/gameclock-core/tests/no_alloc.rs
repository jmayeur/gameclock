//! Updates do not allocate heap memory (`openspec/specs/scene-model`,
//! docs/adr/0006, docs/adr/0012).
//!
//! `allocation-counter` installs a counting global allocator in this test
//! binary only.

use gameclock_core::Compositor;
use gameclock_core::alert::{AlertScene, Alerts};
use gameclock_core::clock::Clock;
use gameclock_core::heartbeat::Heartbeat;
use gameclock_core::message::{Message, STEP, Scroll};
use gameclock_core::page::{Forecast, PageData, PageName, PageScene, Period};
use gameclock_core::schedule::Schedule;
use jiff::tz::TimeZone;
use jiff::{SignedDuration, Timestamp};

#[test]
fn one_thousand_updates_allocate_nothing() {
    let start: Timestamp = "2026-01-01T12:00:00Z".parse().unwrap();
    let mut compositor = Compositor::new(Box::new(Heartbeat), 64, 32);
    assert!(compositor.update(&start.to_zoned(TimeZone::UTC)).is_some());

    // Make the times before the measurement. Only the updates are measured.
    let times: Vec<_> = (1..=1000)
        .map(|s| (start + SignedDuration::from_secs(s)).to_zoned(TimeZone::UTC))
        .collect();

    let mut presents = 0;
    let info = allocation_counter::measure(|| {
        for now in &times {
            if compositor.update(now).is_some() {
                presents += 1;
            }
            let _ = compositor.deadline(now);
        }
    });

    assert_eq!(presents, 1000, "each second moves the marker");
    assert_eq!(info.count_total, 0, "heap allocations in 1000 updates");
}

#[test]
fn one_thousand_clock_minutes_allocate_nothing() {
    let start: Timestamp = "2026-11-10T00:00:00Z".parse().unwrap();
    let clock = Clock::new(Schedule::legacy());
    let mut compositor = Compositor::new(Box::new(clock), 64, 32);
    assert!(compositor.update(&start.to_zoned(TimeZone::UTC)).is_some());

    // Make the times before the measurement. Only the updates are measured.
    let times: Vec<_> = (1..=1000)
        .map(|m| (start + SignedDuration::from_mins(m)).to_zoned(TimeZone::UTC))
        .collect();

    let mut presents = 0;
    let info = allocation_counter::measure(|| {
        for now in &times {
            if compositor.update(now).is_some() {
                presents += 1;
            }
            let _ = compositor.deadline(now);
        }
    });

    assert_eq!(presents, 1000, "each minute changes the text");
    assert_eq!(info.count_total, 0, "heap allocations in 1000 updates");
}

#[test]
fn one_thousand_scroll_steps_allocate_nothing() {
    let start: Timestamp = "2026-11-12T12:00:00Z".parse().unwrap();
    let message = Message::new("a".repeat(64), 10, Message::DEFAULT_COLOR).unwrap();
    let scroll = Scroll::new(message, start, 64, Schedule::legacy());
    let mut compositor = Compositor::new(Box::new(Clock::new(Schedule::legacy())), 64, 32);
    compositor.push(Box::new(scroll));
    assert!(compositor.update(&start.to_zoned(TimeZone::UTC)).is_some());

    // Make the times before the measurement. Only the updates are measured.
    let times: Vec<_> = (1..=1000)
        .map(|n| (start + STEP * n).to_zoned(TimeZone::UTC))
        .collect();

    let mut presents = 0;
    let info = allocation_counter::measure(|| {
        for now in &times {
            if compositor.update(now).is_some() {
                presents += 1;
            }
            let _ = compositor.deadline(now);
        }
    });

    assert_eq!(info.count_total, 0, "heap allocations in 1000 updates");
    assert_eq!(compositor.foreground_len(), 1, "the scroll is not finished");
    // Step 512 starts the second pass. Steps 511 and 512 are both black.
    assert_eq!(presents, 999, "each other step moves the text");
}

#[test]
fn one_thousand_alert_updates_allocate_nothing() {
    for alert in Alerts::legacy().iter() {
        let start: Timestamp = "2026-11-16T12:00:00Z".parse().unwrap();
        let scene = AlertScene::new(std::rc::Rc::clone(alert), start, 64, Schedule::legacy());
        let mut compositor = Compositor::new(Box::new(Clock::new(Schedule::legacy())), 64, 32);
        compositor.push(Box::new(scene));
        assert!(compositor.update(&start.to_zoned(TimeZone::UTC)).is_some());

        // 1000 times in the duration of the alert, before its finish.
        let gap = alert.duration / 1001;
        let times: Vec<_> = (1..=1000)
            .map(|n| (start + gap * n).to_zoned(TimeZone::UTC))
            .collect();

        let info = allocation_counter::measure(|| {
            for now in &times {
                let _ = compositor.update(now);
                let _ = compositor.deadline(now);
            }
        });

        assert_eq!(
            info.count_total, 0,
            "heap allocations in 1000 updates of {}",
            alert.name
        );
        assert_eq!(
            compositor.foreground_len(),
            1,
            "{} is not finished",
            alert.name
        );
    }
}

#[test]
fn page_scroll_steps_allocate_nothing() {
    let start: Timestamp = "2026-11-17T12:00:00Z".parse().unwrap();
    let data = PageData::Forecast(Some(Forecast {
        periods: vec![Period {
            end: start + SignedDuration::from_hours(6),
            temp_f: 48.0,
            rain: 20,
            text: "Chance Rain Showers then Mostly Cloudy".to_owned(),
        }],
    }));
    let page = PageScene::new(PageName::Weather, &data, start, 64, Schedule::legacy());
    let mut compositor = Compositor::new(Box::new(Clock::new(Schedule::legacy())), 64, 32);
    compositor.push(Box::new(page));
    assert!(compositor.update(&start.to_zoned(TimeZone::UTC)).is_some());

    // 700 steps are 28 s, inside the 30 s of the page.
    let times: Vec<_> = (1..=700)
        .map(|n| (start + STEP * n).to_zoned(TimeZone::UTC))
        .collect();

    let mut presents = 0;
    let info = allocation_counter::measure(|| {
        for now in &times {
            if compositor.update(now).is_some() {
                presents += 1;
            }
            let _ = compositor.deadline(now);
        }
    });

    assert_eq!(info.count_total, 0, "heap allocations in 700 updates");
    assert!(presents > 600, "the text moves: {presents} presents");
}
