//! Alerts: the definition, the built-in table, and the alert scene
//! (`openspec/specs/alerts`, docs/adr/0018).

use embedded_graphics::mono_font::ascii::{FONT_7X13_BOLD, FONT_10X20};
use embedded_graphics::mono_font::{MonoFont, MonoTextStyle};
use embedded_graphics::pixelcolor::Rgb888;
use embedded_graphics::prelude::*;
use embedded_graphics::text::{Baseline, Text};
use jiff::{SignedDuration, Timestamp, Zoned};

use crate::effect::{WHEEL_STEP, contrast, wheel, wheel_position};
use crate::message::ScrollPath;
use crate::schedule::Schedule;
use crate::{Brightness, Frame, Scene};

/// The priority of an item on the panel. A higher value wins.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Priority(pub u8);

impl Priority {
    /// The priority of the base scene.
    pub const BASE: Self = Self(0);
    /// The priority of a message.
    pub const MESSAGE: Self = Self(20);
}

/// What an alert draws below its text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Background {
    /// A black panel.
    Black,
    /// All pixels in one color of the color wheel. The color moves with the
    /// time.
    ColorWheel,
}

/// The font of an alert text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Font {
    /// The 7×13 bold font of the clock.
    Small,
    /// The 10×20 font.
    Large,
}

impl Font {
    fn mono(self) -> &'static MonoFont<'static> {
        match self {
            Self::Small => &FONT_7X13_BOLD,
            Self::Large => &FONT_10X20,
        }
    }
}

/// The color of an alert text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextColor {
    /// One color.
    Fixed(Rgb888),
    /// Black or white, whichever is easier to read on the background.
    Contrast,
}

/// How an alert text moves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Motion {
    /// The text stays at this x position.
    Still {
        /// The x position of the text.
        x: i32,
    },
    /// The text scrolls from right to left, as a message does.
    Scroll {
        /// The number of passes.
        passes: u8,
    },
}

/// The style of an alert text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextStyle {
    /// The font.
    pub font: Font,
    /// The y position of the text baseline.
    pub baseline: i32,
    /// The text color.
    pub color: TextColor,
    /// The motion of the text.
    pub motion: Motion,
}

/// What the `clear` command does to an alert.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClearRule {
    /// `clear` removes the alert.
    Removable,
    /// `clear` does not remove the alert. `alert stop` removes it.
    Kept,
}

/// The definition of an alert. Each alert is data of this type, not code
/// (docs/adr/0018).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Alert {
    /// The name in the `alert` command.
    pub name: &'static str,
    /// What the alert draws below its text.
    pub background: Background,
    /// The text.
    pub text: &'static str,
    /// The style of the text.
    pub style: TextStyle,
    /// The longest time that the alert shows.
    pub duration: SignedDuration,
    /// The priority.
    pub priority: Priority,
    /// What `clear` does to the alert.
    pub clear: ClearRule,
}

/// The built-in alerts.
pub const BUILT_IN: &[Alert] = &[
    Alert {
        name: "panic",
        background: Background::ColorWheel,
        text: "PANIC!!",
        style: TextStyle {
            font: Font::Small,
            baseline: 20,
            color: TextColor::Contrast,
            motion: Motion::Still { x: 9 },
        },
        duration: SignedDuration::from_secs(20),
        priority: Priority(40),
        clear: ClearRule::Removable,
    },
    Alert {
        name: "bedtime",
        background: Background::Black,
        text: "GAME OVER - TURN IT OFF!",
        style: TextStyle {
            font: Font::Large,
            baseline: 25,
            color: TextColor::Fixed(Rgb888::new(50, 205, 50)),
            motion: Motion::Scroll { passes: 2 },
        },
        duration: SignedDuration::from_secs(20),
        priority: Priority(30),
        clear: ClearRule::Kept,
    },
    Alert {
        name: "bed-flash",
        background: Background::Black,
        text: "BED!!!!!",
        style: TextStyle {
            font: Font::Small,
            baseline: 20,
            color: TextColor::Fixed(Rgb888::new(255, 0, 0)),
            motion: Motion::Still { x: 8 },
        },
        duration: SignedDuration::from_secs(1),
        priority: Priority(10),
        clear: ClearRule::Kept,
    },
];

/// The built-in alert with the name `name`.
#[must_use]
pub fn find(name: &str) -> Option<&'static Alert> {
    BUILT_IN.iter().find(|a| a.name == name)
}

/// A scene that shows one alert from its start until its finish.
#[derive(Debug, Clone)]
pub struct AlertScene {
    alert: &'static Alert,
    start: Timestamp,
    /// The path of a scrolling text. `None` for a still text.
    path: Option<ScrollPath>,
    /// The earlier of the end of the duration and the end of the last pass.
    finish: Timestamp,
    schedule: Schedule,
}

impl AlertScene {
    /// The scene of `alert` that starts at `start` on a panel `width` pixels
    /// wide. The brightness comes from `schedule`.
    #[must_use]
    pub fn new(alert: &'static Alert, start: Timestamp, width: usize, schedule: Schedule) -> Self {
        let path = match alert.style.motion {
            Motion::Still { .. } => None,
            Motion::Scroll { passes } => {
                let font = alert.style.font.mono();
                let glyph = font.character_size.width + font.character_spacing;
                let chars = alert.text.chars().count();
                Some(ScrollPath::new(start, width, chars, glyph, passes))
            }
        };
        let end = start.saturating_add(alert.duration).unwrap_or(start);
        let finish = match path.and_then(|p| p.finish()) {
            Some(last_pass) => end.min(last_pass),
            None => end,
        };
        Self {
            alert,
            start,
            path,
            finish,
            schedule,
        }
    }

    /// The alert of this scene.
    #[must_use]
    pub fn alert(&self) -> &'static Alert {
        self.alert
    }

    /// The number of whole wheel steps from the start to `now`. It is 0
    /// before the start.
    fn wheel_steps(&self, now: &Zoned) -> u64 {
        let elapsed = now.timestamp().duration_since(self.start);
        let steps = elapsed
            .as_millis()
            .div_euclid(WHEEL_STEP.as_millis())
            .max(0);
        u64::try_from(steps).unwrap_or(u64::MAX)
    }

    /// The time of the next wheel step after `now`.
    fn next_wheel_step(&self, now: &Zoned) -> Option<Timestamp> {
        let steps = self.wheel_steps(now).checked_add(1)?;
        let next = WHEEL_STEP.checked_mul(i32::try_from(steps).ok()?)?;
        self.start.checked_add(next).ok()
    }
}

impl Scene for AlertScene {
    fn render(&mut self, now: &Zoned, frame: &mut Frame) {
        let background = match self.alert.background {
            Background::Black => Rgb888::BLACK,
            Background::ColorWheel => {
                let color = wheel(wheel_position(self.wheel_steps(now)));
                match frame.clear(color) {
                    Ok(()) => {}
                    Err(never) => match never {},
                }
                color
            }
        };
        let x = match (self.alert.style.motion, self.path) {
            (Motion::Still { x }, _) => x,
            (Motion::Scroll { .. }, Some(path)) => match path.x(now) {
                Some(x) => x,
                None => return,
            },
            (Motion::Scroll { .. }, None) => return,
        };
        let color = match self.alert.style.color {
            TextColor::Fixed(color) => color,
            TextColor::Contrast => contrast(background),
        };
        let style = MonoTextStyle::new(self.alert.style.font.mono(), color);
        let text = Text::with_baseline(
            self.alert.text,
            Point::new(x, self.alert.style.baseline),
            style,
            Baseline::Alphabetic,
        );
        match text.draw(frame) {
            Ok(_) => {}
            Err(never) => match never {},
        }
    }

    /// The earliest of the next wheel step, the next scroll step, and the
    /// finish time. `None` after the finish.
    fn next_change(&self, now: &Zoned) -> Option<Timestamp> {
        if self.is_finished(now) {
            return None;
        }
        let mut next = self.finish;
        if self.alert.background == Background::ColorWheel
            && let Some(step) = self.next_wheel_step(now)
        {
            next = next.min(step);
        }
        if let Some(step) = self.path.and_then(|p| p.next_step(now)) {
            next = next.min(step);
        }
        Some(next)
    }

    fn is_finished(&self, now: &Zoned) -> bool {
        now.timestamp() >= self.finish
    }

    fn brightness(&self, now: &Zoned) -> Brightness {
        self.schedule.style_at(now).brightness
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use embedded_graphics::pixelcolor::RgbColor;
    use jiff::tz::TimeZone;

    const T: &str = "2026-11-16T12:00:00Z";

    fn start() -> Timestamp {
        T.parse().unwrap()
    }

    fn at(offset_ms: i64) -> Zoned {
        (start() + SignedDuration::from_millis(offset_ms)).to_zoned(TimeZone::UTC)
    }

    fn scene(name: &str) -> AlertScene {
        AlertScene::new(find(name).unwrap(), start(), 64, Schedule::legacy())
    }

    fn render(scene: &mut AlertScene, now: &Zoned) -> Frame {
        let mut f = Frame::new(64, 32);
        scene.render(now, &mut f);
        f
    }

    /// The reference image: `background`, then `text` in `font` at
    /// `(x, baseline)` in `color`.
    fn drawn(
        background: Rgb888,
        text: &str,
        font: &MonoFont<'_>,
        x: i32,
        baseline: i32,
        color: Rgb888,
    ) -> Frame {
        let mut f = Frame::new(64, 32);
        f.clear(background).unwrap();
        let style = MonoTextStyle::new(font, color);
        Text::with_baseline(text, Point::new(x, baseline), style, Baseline::Alphabetic)
            .draw(&mut f)
            .unwrap();
        f
    }

    fn panic_frame(background: Rgb888, text: Rgb888) -> Frame {
        drawn(background, "PANIC!!", &FONT_7X13_BOLD, 9, 20, text)
    }

    // The panic alert fills the panel with a color wheel.

    #[test]
    fn wheel_at_the_start() {
        let mut s = scene("panic");
        assert_eq!(
            render(&mut s, &at(0)),
            panic_frame(Rgb888::new(0, 0, 255), Rgb888::WHITE)
        );
    }

    #[test]
    fn wheel_position_comes_from_the_time() {
        let mut s = scene("panic");
        assert_eq!(
            render(&mut s, &at(5000)),
            panic_frame(Rgb888::new(211, 44, 0), Rgb888::WHITE)
        );
        assert_eq!(
            render(&mut s, &at(10_000)),
            panic_frame(Rgb888::new(0, 167, 88), Rgb888::WHITE)
        );
    }

    #[test]
    fn next_change_is_the_next_wheel_step() {
        let s = scene("panic");
        assert_eq!(
            s.next_change(&at(1010)),
            Some(start() + SignedDuration::from_millis(1050))
        );
    }

    #[test]
    fn panic_finishes_after_20_s() {
        let s = scene("panic");
        assert!(!s.is_finished(&at(19_999)));
        assert!(s.is_finished(&at(20_000)));
        assert_eq!(
            s.next_change(&at(19_990)),
            Some(start() + SignedDuration::from_secs(20))
        );
        assert_eq!(s.next_change(&at(20_000)), None);
    }

    // The panic text contrasts with the wheel.

    #[test]
    fn white_text_on_a_dark_color() {
        let mut s = scene("panic");
        assert_eq!(
            render(&mut s, &at(0)),
            panic_frame(Rgb888::new(0, 0, 255), Rgb888::new(255, 255, 255))
        );
    }

    #[test]
    fn black_text_on_a_light_color() {
        let mut s = scene("panic");
        assert_eq!(
            render(&mut s, &at(8350)),
            panic_frame(Rgb888::new(10, 245, 0), Rgb888::BLACK)
        );
    }

    // The bedtime alert scrolls a large text.

    const LIME: Rgb888 = Rgb888::new(50, 205, 50);
    const BEDTIME: &str = "GAME OVER - TURN IT OFF!";

    fn bedtime_at(x: i32) -> Frame {
        drawn(Rgb888::BLACK, BEDTIME, &FONT_10X20, x, 25, LIME)
    }

    #[test]
    fn bedtime_text_enters_at_the_right_edge() {
        let mut s = scene("bedtime");
        assert_eq!(render(&mut s, &at(0)), Frame::new(64, 32));
        assert_eq!(render(&mut s, &at(1000)), bedtime_at(39));
    }

    #[test]
    fn second_pass_starts_at_the_right_edge() {
        let mut s = scene("bedtime");
        assert_eq!(render(&mut s, &at(12_160)), Frame::new(64, 32));
        assert!(!s.is_finished(&at(12_160)));
        assert_eq!(render(&mut s, &at(12_200)), bedtime_at(63));
    }

    #[test]
    fn duration_ends_the_bedtime_alert_before_its_passes() {
        let s = scene("bedtime");
        assert!(!s.is_finished(&at(19_999)));
        assert!(s.is_finished(&at(20_000)));
        assert_eq!(s.next_change(&at(20_000)), None);
    }

    // The bed-flash alert shows a still text for 1 s.

    #[test]
    fn bed_flash_frame() {
        let mut s = scene("bed-flash");
        let expected = drawn(
            Rgb888::BLACK,
            "BED!!!!!",
            &FONT_7X13_BOLD,
            8,
            20,
            Rgb888::new(255, 0, 0),
        );
        assert_eq!(render(&mut s, &at(0)), expected);
        assert_eq!(render(&mut s, &at(500)), expected);
    }

    #[test]
    fn bed_flash_next_change_is_its_finish() {
        let s = scene("bed-flash");
        assert_eq!(
            s.next_change(&at(0)),
            Some(start() + SignedDuration::from_secs(1))
        );
        assert!(!s.is_finished(&at(999)));
        assert!(s.is_finished(&at(1000)));
    }

    // An alert is a definition, not code.

    #[test]
    fn two_definitions_with_the_same_parts_give_the_same_frames() {
        static COPY: Alert = Alert {
            name: "copy",
            ..BUILT_IN[2]
        };
        assert_eq!(BUILT_IN[2].name, "bed-flash");
        let mut copy = AlertScene::new(&COPY, start(), 64, Schedule::legacy());
        let mut flash = scene("bed-flash");
        for ms in [0, 400, 999] {
            assert_eq!(
                render(&mut copy, &at(ms)),
                render(&mut flash, &at(ms)),
                "{ms}"
            );
        }
    }

    #[test]
    fn find_each_built_in_alert() {
        for name in ["panic", "bedtime", "bed-flash"] {
            assert_eq!(find(name).map(|a| a.name), Some(name));
        }
        assert_eq!(find("fire"), None);
        assert_eq!(BUILT_IN.len(), 3);
    }

    // An alert uses the brightness of the clock schedule.

    fn utc(s: &str) -> Zoned {
        s.parse::<Timestamp>().unwrap().to_zoned(TimeZone::UTC)
    }

    #[test]
    fn panic_at_night() {
        let s = scene("panic");
        assert_eq!(
            s.brightness(&utc("2026-11-16T01:00:00Z")),
            Brightness::new(25)
        );
    }

    #[test]
    fn bedtime_on_a_bedtime_night() {
        // 2026-11-16 is a Monday.
        let s = scene("bedtime");
        assert_eq!(
            s.brightness(&utc("2026-11-16T23:05:00Z")),
            Brightness::new(100)
        );
    }
}
