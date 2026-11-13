//! The scene contract and the compositor (`openspec/specs/scene-model`,
//! docs/adr/0012).

use embedded_graphics::pixelcolor::Rgb888;
use embedded_graphics::prelude::*;
use jiff::{SignedDuration, Timestamp, Zoned};

use crate::{Brightness, Frame};

/// The shortest time between two renders. It caps renders at 30 each second
/// (docs/adr/0006).
pub const MIN_INTERVAL: SignedDuration = SignedDuration::from_millis(33);

/// A self-contained panel behavior, for example a clock or a message.
///
/// The daemon injects the time. A scene never reads a clock (docs/adr/0005).
pub trait Scene {
    /// Draw the scene for `now` into `frame`. The frame is black when this
    /// starts.
    fn render(&mut self, now: &Zoned, frame: &mut Frame);

    /// The next time after `now` at which the image can change. `None` means
    /// that the image does not change. A scene that finishes at a time must
    /// give that time, or an earlier time.
    fn next_change(&self, now: &Zoned) -> Option<Timestamp>;

    /// True when the scene has nothing more to show at `now`.
    fn is_finished(&self, now: &Zoned) -> bool;

    /// The panel brightness for `now`. The default is full brightness
    /// (docs/adr/0014).
    fn brightness(&self, _now: &Zoned) -> Brightness {
        Brightness::MAX
    }
}

/// Decides which scene is visible and renders it.
///
/// It holds one base scene and a stack of foreground scenes. It renders only
/// the top scene. It gives a frame to present only when the image changed.
pub struct Compositor {
    base: Box<dyn Scene>,
    foreground: Vec<Box<dyn Scene>>,
    /// The frame that the top scene renders into.
    work: Frame,
    /// The last frame that `update` gave.
    shown: Frame,
    /// The brightness that `update` gave with `shown`.
    shown_brightness: Brightness,
    /// False until the first `update` gives a frame.
    started: bool,
}

impl core::fmt::Debug for Compositor {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Compositor")
            .field("foreground", &self.foreground.len())
            .field("width", &self.work.width())
            .field("height", &self.work.height())
            .finish_non_exhaustive()
    }
}

impl Compositor {
    /// A compositor that renders `base` into frames of `width` × `height`.
    #[must_use]
    pub fn new(base: Box<dyn Scene>, width: usize, height: usize) -> Self {
        Self {
            base,
            foreground: Vec::new(),
            work: Frame::new(width, height),
            shown: Frame::new(width, height),
            shown_brightness: Brightness::MAX,
            started: false,
        }
    }

    /// Show `scene` on top of all other scenes until it finishes.
    pub fn push(&mut self, scene: Box<dyn Scene>) {
        self.foreground.push(scene);
    }

    /// The time of the next render: the top scene's next change, and at
    /// least [`MIN_INTERVAL`] after `now`. `None` means that no render is
    /// necessary until the stack changes.
    #[must_use]
    pub fn deadline(&self, now: &Zoned) -> Option<Timestamp> {
        let top = self.foreground.last().unwrap_or(&self.base);
        let next = top.next_change(now)?;
        let earliest = now.timestamp().saturating_add(MIN_INTERVAL).ok()?;
        Some(next.max(earliest))
    }

    /// Replace the base scene with `base`. The foreground scenes stay. The
    /// next update gives a frame, even if the image did not change.
    pub fn set_base(&mut self, base: Box<dyn Scene>) {
        self.base = base;
        self.started = false;
    }

    /// The brightness of the last frame that `update` gave. It is full
    /// brightness before the first update.
    #[must_use]
    pub fn last_brightness(&self) -> Brightness {
        self.shown_brightness
    }

    /// The number of foreground scenes.
    #[must_use]
    pub fn foreground_len(&self) -> usize {
        self.foreground.len()
    }

    /// Remove the finished foreground scenes, then render the top scene for
    /// `now`. Give the frame and the top scene's brightness if the frame or
    /// the brightness is different from the last ones that this function
    /// gave. The first call always gives a frame.
    pub fn update(&mut self, now: &Zoned) -> Option<(&Frame, Brightness)> {
        self.foreground.retain(|s| !s.is_finished(now));
        match self.work.clear(Rgb888::BLACK) {
            Ok(()) => {}
            Err(never) => match never {},
        }
        let top = match self.foreground.last_mut() {
            Some(scene) => scene.as_mut(),
            None => self.base.as_mut(),
        };
        top.render(now, &mut self.work);
        let brightness = top.brightness(now);
        if self.started && brightness == self.shown_brightness && self.work == self.shown {
            return None;
        }
        self.started = true;
        self.shown_brightness = brightness;
        core::mem::swap(&mut self.work, &mut self.shown);
        Some((&self.shown, brightness))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Brightness, Frame};
    use embedded_graphics::pixelcolor::Rgb888;
    use jiff::tz::TimeZone;
    use jiff::{Timestamp, Zoned};

    const W: usize = 8;
    const H: usize = 4;

    fn ts(s: &str) -> Timestamp {
        s.parse().unwrap()
    }

    fn at(s: &str) -> Zoned {
        ts(s).to_zoned(TimeZone::UTC)
    }

    /// Fills the frame with one color.
    struct Solid {
        color: Rgb888,
        next: Option<Timestamp>,
        finished_from: Option<Timestamp>,
    }

    impl Solid {
        fn plain(color: Rgb888) -> Box<dyn Scene> {
            Box::new(Self {
                color,
                next: None,
                finished_from: None,
            })
        }

        fn finished_from(color: Rgb888, from: &str) -> Box<dyn Scene> {
            Box::new(Self {
                color,
                next: Some(ts(from)),
                finished_from: Some(ts(from)),
            })
        }

        fn next_at(color: Rgb888, next: &str) -> Box<dyn Scene> {
            Box::new(Self {
                color,
                next: Some(ts(next)),
                finished_from: None,
            })
        }
    }

    impl Scene for Solid {
        fn render(&mut self, _now: &Zoned, frame: &mut Frame) {
            frame.clear(self.color).unwrap();
        }
        fn next_change(&self, _now: &Zoned) -> Option<Timestamp> {
            self.next
        }
        fn is_finished(&self, now: &Zoned) -> bool {
            self.finished_from.is_some_and(|t| now.timestamp() >= t)
        }
    }

    /// Draws only a white pixel at (0, 0).
    struct Dot;

    impl Scene for Dot {
        fn render(&mut self, _now: &Zoned, frame: &mut Frame) {
            Pixel(Point::zero(), Rgb888::WHITE).draw(frame).unwrap();
        }
        fn next_change(&self, _now: &Zoned) -> Option<Timestamp> {
            None
        }
        fn is_finished(&self, _now: &Zoned) -> bool {
            false
        }
    }

    /// Red before `at`, blue from `at`.
    struct Switch {
        at: Timestamp,
    }

    impl Scene for Switch {
        fn render(&mut self, now: &Zoned, frame: &mut Frame) {
            let color = if now.timestamp() < self.at {
                Rgb888::RED
            } else {
                Rgb888::BLUE
            };
            frame.clear(color).unwrap();
        }
        fn next_change(&self, now: &Zoned) -> Option<Timestamp> {
            (now.timestamp() < self.at).then_some(self.at)
        }
        fn is_finished(&self, _now: &Zoned) -> bool {
            false
        }
    }

    /// A finished base scene.
    struct Done;

    impl Scene for Done {
        fn render(&mut self, _now: &Zoned, frame: &mut Frame) {
            frame.clear(Rgb888::RED).unwrap();
        }
        fn next_change(&self, _now: &Zoned) -> Option<Timestamp> {
            None
        }
        fn is_finished(&self, _now: &Zoned) -> bool {
            true
        }
    }

    /// A red image that does not change, at one brightness before `at` and
    /// another from `at`.
    struct Dim {
        before: Brightness,
        after: Brightness,
        at: Timestamp,
    }

    impl Dim {
        fn fixed(percent: u8) -> Box<dyn Scene> {
            Box::new(Self {
                before: Brightness::new(percent),
                after: Brightness::new(percent),
                at: ts(T0),
            })
        }
    }

    impl Scene for Dim {
        fn render(&mut self, _now: &Zoned, frame: &mut Frame) {
            frame.clear(Rgb888::RED).unwrap();
        }
        fn next_change(&self, now: &Zoned) -> Option<Timestamp> {
            (now.timestamp() < self.at).then_some(self.at)
        }
        fn is_finished(&self, _now: &Zoned) -> bool {
            false
        }
        fn brightness(&self, now: &Zoned) -> Brightness {
            if now.timestamp() < self.at {
                self.before
            } else {
                self.after
            }
        }
    }

    fn all(frame: &Frame, color: Rgb888) -> bool {
        frame.pixels().iter().all(|&p| p == color)
    }

    const T0: &str = "2026-01-01T12:00:00Z";

    // The top scene is visible.

    #[test]
    fn base_scene_with_an_empty_stack() {
        let mut c = Compositor::new(Solid::plain(Rgb888::RED), W, H);
        let frame = c.update(&at(T0)).expect("first update gives a frame").0;
        assert!(all(frame, Rgb888::RED));
    }

    #[test]
    fn newest_foreground_scene_is_on_top() {
        let mut c = Compositor::new(Solid::plain(Rgb888::RED), W, H);
        c.push(Solid::plain(Rgb888::GREEN));
        c.push(Solid::plain(Rgb888::BLUE));
        let frame = c.update(&at(T0)).unwrap().0;
        assert!(all(frame, Rgb888::BLUE));
    }

    // Each render starts from a black frame.

    #[test]
    fn no_pixels_remain_from_the_scene_below() {
        let mut c = Compositor::new(Solid::plain(Rgb888::RED), W, H);
        assert!(all(c.update(&at(T0)).unwrap().0, Rgb888::RED));
        c.push(Box::new(Dot));
        let frame = c.update(&at(T0)).unwrap().0;
        assert_eq!(frame.pixel(Point::zero()), Some(Rgb888::WHITE));
        let black = frame.pixels()[1..].iter().all(|&p| p == Rgb888::BLACK);
        assert!(black, "all pixels except (0, 0) must be black");
    }

    // Finished foreground scenes go away.

    fn with_finishing_green() -> Compositor {
        let mut c = Compositor::new(Solid::plain(Rgb888::RED), W, H);
        c.push(Solid::finished_from(Rgb888::GREEN, "2026-01-01T12:00:05Z"));
        c
    }

    #[test]
    fn base_scene_returns_after_the_foreground_scene_finishes() {
        let mut c = with_finishing_green();
        let frame = c.update(&at("2026-01-01T12:00:05Z")).unwrap().0;
        assert!(all(frame, Rgb888::RED));
        assert_eq!(c.foreground_len(), 0);
    }

    #[test]
    fn unfinished_foreground_scene_stays() {
        let mut c = with_finishing_green();
        let frame = c.update(&at("2026-01-01T12:00:04Z")).unwrap().0;
        assert!(all(frame, Rgb888::GREEN));
        assert_eq!(c.foreground_len(), 1);
    }

    // The base scene always stays.

    #[test]
    fn finished_base_scene_is_still_shown() {
        let mut c = Compositor::new(Box::new(Done), W, H);
        let frame = c.update(&at(T0)).unwrap().0;
        assert!(all(frame, Rgb888::RED));
    }

    // Only a changed frame is presented.

    #[test]
    fn first_update_gives_a_frame() {
        let mut c = Compositor::new(Solid::plain(Rgb888::RED), W, H);
        assert!(c.update(&at(T0)).is_some());
    }

    #[test]
    fn first_update_gives_a_black_frame_too() {
        let mut c = Compositor::new(Solid::plain(Rgb888::BLACK), W, H);
        assert!(c.update(&at(T0)).is_some());
    }

    #[test]
    fn same_image_gives_no_frame() {
        let mut c = Compositor::new(Solid::plain(Rgb888::RED), W, H);
        assert!(c.update(&at(T0)).is_some());
        assert!(c.update(&at(T0)).is_none());
    }

    #[test]
    fn changed_image_gives_a_frame() {
        let switch = Box::new(Switch {
            at: ts("2026-01-01T12:00:01Z"),
        });
        let mut c = Compositor::new(switch, W, H);
        assert!(all(c.update(&at(T0)).unwrap().0, Rgb888::RED));
        let second = c.update(&at("2026-01-01T12:00:01Z"));
        assert!(all(second.expect("changed image").0, Rgb888::BLUE));
    }

    #[test]
    fn changed_brightness_gives_a_frame() {
        let dim = Box::new(Dim {
            before: Brightness::new(25),
            after: Brightness::new(60),
            at: ts("2026-01-01T12:00:01Z"),
        });
        let mut c = Compositor::new(dim, W, H);
        assert!(c.update(&at(T0)).is_some());
        let second = c.update(&at("2026-01-01T12:00:01Z"));
        let (frame, brightness) = second.expect("changed brightness");
        assert!(all(frame, Rgb888::RED));
        assert_eq!(brightness, Brightness::new(60));
    }

    // Scenes give a brightness.

    #[test]
    fn default_brightness_is_full() {
        let mut c = Compositor::new(Solid::plain(Rgb888::RED), W, H);
        assert_eq!(c.update(&at(T0)).unwrap().1, Brightness::MAX);
    }

    #[test]
    fn brightness_of_the_top_scene() {
        let mut c = Compositor::new(Dim::fixed(25), W, H);
        c.push(Dim::fixed(80));
        assert_eq!(c.update(&at(T0)).unwrap().1, Brightness::new(80));
    }

    // The base scene can be replaced.

    #[test]
    fn new_base_scene_shows_at_the_next_update() {
        let mut c = Compositor::new(Solid::plain(Rgb888::RED), W, H);
        assert!(c.update(&at(T0)).is_some());
        c.set_base(Solid::plain(Rgb888::BLUE));
        let frame = c.update(&at(T0)).expect("a frame after the replacement").0;
        assert!(all(frame, Rgb888::BLUE));
    }

    #[test]
    fn foreground_scenes_stay_on_top() {
        let mut c = Compositor::new(Solid::plain(Rgb888::RED), W, H);
        c.push(Solid::plain(Rgb888::GREEN));
        c.set_base(Solid::plain(Rgb888::BLUE));
        assert!(all(c.update(&at(T0)).unwrap().0, Rgb888::GREEN));
        assert_eq!(c.foreground_len(), 1);
    }

    #[test]
    fn same_image_still_gives_a_frame() {
        let mut c = Compositor::new(Solid::plain(Rgb888::RED), W, H);
        assert!(c.update(&at(T0)).is_some());
        c.set_base(Solid::plain(Rgb888::RED));
        assert!(c.update(&at(T0)).is_some());
        assert!(
            c.update(&at(T0)).is_none(),
            "only the first update after it"
        );
    }

    // The compositor tells the last brightness.

    #[test]
    fn last_brightness_after_an_update() {
        let mut c = Compositor::new(Dim::fixed(25), W, H);
        c.update(&at(T0));
        assert_eq!(c.last_brightness(), Brightness::new(25));
    }

    #[test]
    fn last_brightness_before_an_update() {
        let c = Compositor::new(Dim::fixed(25), W, H);
        assert_eq!(c.last_brightness(), Brightness::MAX);
    }

    // The deadline is the top scene's next change.

    #[test]
    fn static_scene_has_no_deadline() {
        let mut c = Compositor::new(Solid::plain(Rgb888::RED), W, H);
        let now = at(T0);
        c.update(&now);
        assert_eq!(c.deadline(&now), None);
    }

    #[test]
    fn deadline_follows_the_top_scene() {
        let mut c = Compositor::new(Solid::next_at(Rgb888::RED, "2026-01-01T12:01:00Z"), W, H);
        c.push(Solid::next_at(Rgb888::GREEN, "2026-01-01T12:00:10Z"));
        let now = at(T0);
        c.update(&now);
        assert_eq!(c.deadline(&now), Some(ts("2026-01-01T12:00:10Z")));
    }

    // A scene's finish time is a change.

    #[test]
    fn deadline_is_the_finish_time() {
        let mut c = with_finishing_green();
        let now = at(T0);
        c.update(&now);
        assert_eq!(c.deadline(&now), Some(ts("2026-01-01T12:00:05Z")));
    }

    // Renders are at most 30 per second.

    #[test]
    fn too_early_change_is_delayed() {
        let mut c = Compositor::new(
            Solid::next_at(Rgb888::RED, "2026-01-01T12:00:00.001Z"),
            W,
            H,
        );
        let now = at(T0);
        c.update(&now);
        assert_eq!(c.deadline(&now), Some(ts("2026-01-01T12:00:00.033Z")));
    }

    #[test]
    fn past_change_is_delayed() {
        let mut c = Compositor::new(Solid::next_at(Rgb888::RED, "2026-01-01T11:59:59Z"), W, H);
        let now = at(T0);
        c.update(&now);
        assert_eq!(c.deadline(&now), Some(ts("2026-01-01T12:00:00.033Z")));
    }
}
