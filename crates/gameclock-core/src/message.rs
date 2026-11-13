//! Text messages: the message, the scroll scene, and the queue
//! (`openspec/specs/message-board`, docs/adr/0017).

use core::fmt;
use std::collections::VecDeque;

use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::mono_font::ascii::FONT_7X13_BOLD;
use embedded_graphics::pixelcolor::Rgb888;
use embedded_graphics::prelude::*;
use embedded_graphics::text::{Baseline, Text};
use jiff::{SignedDuration, Timestamp, Zoned};

use crate::schedule::Schedule;
use crate::{Brightness, Frame, Scene};

/// The time for each 1-pixel step of a scroll (docs/adr/0017).
pub const STEP: SignedDuration = SignedDuration::from_millis(40);

/// The baseline of the text, as the clock draws it.
const BASELINE: i32 = 20;

/// A text that scrolls across the panel for a number of passes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    text: String,
    passes: u8,
    color: Rgb888,
}

impl Message {
    /// The pass count when the sender gives none.
    pub const DEFAULT_PASSES: u8 = 4;
    /// The color when the sender gives none: the legacy green.
    pub const DEFAULT_COLOR: Rgb888 = Rgb888::new(0, 150, 0);
    /// The most characters in a text.
    pub const MAX_CHARS: usize = 64;
    /// The most passes.
    pub const MAX_PASSES: u8 = 10;

    /// A message of `text` in `color` for `passes` passes.
    ///
    /// # Errors
    /// [`MessageError`] if the text is empty or has only spaces, if it has
    /// more than [`Self::MAX_CHARS`] characters, or if `passes` is not from 1
    /// to [`Self::MAX_PASSES`].
    pub fn new(text: impl Into<String>, passes: u8, color: Rgb888) -> Result<Self, MessageError> {
        let text = text.into();
        if text.trim().is_empty() {
            return Err(MessageError::NoText);
        }
        if text.chars().count() > Self::MAX_CHARS {
            return Err(MessageError::TooLong);
        }
        if !(1..=Self::MAX_PASSES).contains(&passes) {
            return Err(MessageError::Passes);
        }
        Ok(Self {
            text,
            passes,
            color,
        })
    }

    /// The text.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// The number of passes.
    #[must_use]
    pub fn passes(&self) -> u8 {
        self.passes
    }

    /// The text color.
    #[must_use]
    pub fn color(&self) -> Rgb888 {
        self.color
    }
}

/// A message that is not valid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageError {
    /// The text is empty or has only spaces.
    NoText,
    /// The text has more than [`Message::MAX_CHARS`] characters.
    TooLong,
    /// The pass count is not from 1 to [`Message::MAX_PASSES`].
    Passes,
}

impl fmt::Display for MessageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoText => f.write_str("the message has no text."),
            Self::TooLong => write!(
                f,
                "the message is longer than {} characters.",
                Message::MAX_CHARS
            ),
            Self::Passes => write!(
                f,
                "the pass count must be from 1 to {}.",
                Message::MAX_PASSES
            ),
        }
    }
}

impl std::error::Error for MessageError {}

/// A scene that scrolls one message from the right edge to the left edge.
///
/// The position comes from the time since `start`, so a late render does not
/// slow the scroll (docs/adr/0017).
#[derive(Debug, Clone)]
pub struct Scroll {
    message: Message,
    start: Timestamp,
    /// The x position of the text at the start of each pass.
    width: i64,
    /// The steps in one pass: the panel width and the text width.
    pass: i64,
    /// The steps in all passes.
    total: i64,
    schedule: Schedule,
}

impl Scroll {
    /// A scroll of `message` that starts at `start` on a panel `width`
    /// pixels wide. The brightness comes from `schedule`.
    #[must_use]
    pub fn new(message: Message, start: Timestamp, width: usize, schedule: Schedule) -> Self {
        let glyph = FONT_7X13_BOLD.character_size.width + FONT_7X13_BOLD.character_spacing;
        // A message has at most 64 characters, so these values are small.
        let chars = i64::try_from(message.text.chars().count()).unwrap_or(i64::MAX);
        let width = i64::try_from(width).unwrap_or(i64::MAX);
        let pass = width.saturating_add(chars.saturating_mul(i64::from(glyph)));
        let total = pass.saturating_mul(i64::from(message.passes));
        Self {
            message,
            start,
            width,
            pass,
            total,
            schedule,
        }
    }

    /// The number of whole steps from the start to `now`. It is 0 before the
    /// start.
    fn steps(&self, now: &Zoned) -> i64 {
        let elapsed = now.timestamp().duration_since(self.start);
        let steps = elapsed.as_millis().div_euclid(STEP.as_millis()).max(0);
        i64::try_from(steps).unwrap_or(i64::MAX)
    }
}

impl Scene for Scroll {
    fn render(&mut self, now: &Zoned, frame: &mut Frame) {
        let steps = self.steps(now);
        if steps >= self.total || self.pass <= 0 {
            return;
        }
        let Ok(x) = i32::try_from(self.width - steps % self.pass) else {
            return;
        };
        let style = MonoTextStyle::new(&FONT_7X13_BOLD, self.message.color);
        let text = Text::with_baseline(
            &self.message.text,
            Point::new(x, BASELINE),
            style,
            Baseline::Alphabetic,
        );
        match text.draw(frame) {
            Ok(_) => {}
            Err(never) => match never {},
        }
    }

    /// The time of the next step. The last step is the finish time.
    fn next_change(&self, now: &Zoned) -> Option<Timestamp> {
        let steps = self.steps(now);
        if steps >= self.total {
            return None;
        }
        let next = STEP.checked_mul(i32::try_from(steps + 1).ok()?)?;
        self.start.checked_add(next).ok()
    }

    fn is_finished(&self, now: &Zoned) -> bool {
        self.steps(now) >= self.total
    }

    fn brightness(&self, now: &Zoned) -> Brightness {
        self.schedule.style_at(now).brightness
    }
}

/// The messages that wait, first in, first out (docs/adr/0017).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Queue {
    waiting: VecDeque<Message>,
}

impl Queue {
    /// The most messages that can wait.
    pub const MAX_WAITING: usize = 8;

    /// An empty queue.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Put `message` at the end of the queue.
    ///
    /// # Errors
    /// [`QueueFull`] if [`Self::MAX_WAITING`] messages wait.
    pub fn push(&mut self, message: Message) -> Result<(), QueueFull> {
        if self.waiting.len() >= Self::MAX_WAITING {
            return Err(QueueFull);
        }
        self.waiting.push_back(message);
        Ok(())
    }

    /// Remove the first message and give it.
    pub fn pop(&mut self) -> Option<Message> {
        self.waiting.pop_front()
    }

    /// Remove all messages.
    pub fn clear(&mut self) {
        self.waiting.clear();
    }

    /// The number of messages that wait.
    #[must_use]
    pub fn len(&self) -> usize {
        self.waiting.len()
    }

    /// True when no message waits.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.waiting.is_empty()
    }
}

/// The queue has [`Queue::MAX_WAITING`] messages, and cannot take one more.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QueueFull;

impl fmt::Display for QueueFull {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "the message queue is full. {} messages wait.",
            Queue::MAX_WAITING
        )
    }
}

impl std::error::Error for QueueFull {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schedule::Schedule;
    use crate::{Brightness, Frame, Scene};
    use embedded_graphics::mono_font::MonoTextStyle;
    use embedded_graphics::mono_font::ascii::FONT_7X13_BOLD;
    use embedded_graphics::pixelcolor::Rgb888;
    use embedded_graphics::text::{Baseline, Text};
    use jiff::tz::TimeZone;
    use jiff::{SignedDuration, Timestamp, Zoned};

    // A message has a text, a pass count, and a color.

    #[test]
    fn defaults() {
        assert_eq!(Message::DEFAULT_PASSES, 4);
        assert_eq!(Message::DEFAULT_COLOR, Rgb888::new(0, 150, 0));
    }

    #[test]
    fn message_keeps_its_parts() {
        let m = Message::new("good night", 2, Rgb888::new(255, 140, 0)).unwrap();
        assert_eq!(m.text(), "good night");
        assert_eq!(m.passes(), 2);
        assert_eq!(m.color(), Rgb888::new(255, 140, 0));
    }

    #[test]
    fn spaces_in_the_text_stay() {
        let m = Message::new("a  b", 4, Message::DEFAULT_COLOR).unwrap();
        assert_eq!(m.text(), "a  b");
    }

    #[test]
    fn no_text() {
        for text in ["", "   "] {
            let err = Message::new(text, 4, Message::DEFAULT_COLOR).unwrap_err();
            assert_eq!(err, MessageError::NoText, "{text:?}");
        }
    }

    #[test]
    fn the_limit_is_64_characters() {
        let at_limit = "é".repeat(64);
        assert!(Message::new(at_limit, 4, Message::DEFAULT_COLOR).is_ok());
        let err = Message::new("a".repeat(65), 4, Message::DEFAULT_COLOR).unwrap_err();
        assert_eq!(err, MessageError::TooLong);
    }

    #[test]
    fn passes_from_1_to_10() {
        for passes in [1, 10] {
            assert!(Message::new("hi", passes, Message::DEFAULT_COLOR).is_ok());
        }
        for passes in [0, 11] {
            let err = Message::new("hi", passes, Message::DEFAULT_COLOR).unwrap_err();
            assert_eq!(err, MessageError::Passes, "{passes}");
        }
    }

    // The text scrolls from right to left.

    const T: &str = "2026-11-12T12:00:00Z";

    fn at(offset_ms: i64) -> Zoned {
        let t: Timestamp = T.parse().unwrap();
        (t + SignedDuration::from_millis(offset_ms)).to_zoned(TimeZone::UTC)
    }

    fn scroll(text: &str, passes: u8) -> Scroll {
        let message = Message::new(text, passes, Message::DEFAULT_COLOR).unwrap();
        Scroll::new(message, T.parse().unwrap(), 64, Schedule::legacy())
    }

    fn render(scene: &mut Scroll, now: &Zoned) -> Frame {
        let mut f = Frame::new(64, 32);
        scene.render(now, &mut f);
        f
    }

    /// The reference image: `text` in the 7×13 bold font at `x`, baseline
    /// y=20, in the default color, on black.
    fn drawn_at(text: &str, x: i32) -> Frame {
        let mut f = Frame::new(64, 32);
        let style = MonoTextStyle::new(&FONT_7X13_BOLD, Message::DEFAULT_COLOR);
        Text::with_baseline(text, Point::new(x, 20), style, Baseline::Alphabetic)
            .draw(&mut f)
            .unwrap();
        f
    }

    fn black() -> Frame {
        Frame::new(64, 32)
    }

    #[test]
    fn text_enters_at_the_right_edge() {
        let mut s = scroll("HI", 4);
        assert_eq!(render(&mut s, &at(0)), black());
        assert_eq!(render(&mut s, &at(40)), drawn_at("HI", 63));
    }

    #[test]
    fn position_comes_from_the_time() {
        let mut s = scroll("HI", 4);
        assert_eq!(render(&mut s, &at(1000)), drawn_at("HI", 39));
        // A render at an earlier time does not change the result.
        assert_eq!(render(&mut s, &at(39)), black());
        assert_eq!(render(&mut s, &at(1039)), drawn_at("HI", 39));
    }

    #[test]
    fn the_next_pass_starts_at_the_right_edge() {
        let mut s = scroll("HI", 2);
        assert_eq!(render(&mut s, &at(3120)), black());
        assert!(!s.is_finished(&at(3120)));
        assert_eq!(render(&mut s, &at(3160)), drawn_at("HI", 63));
    }

    #[test]
    fn character_that_the_font_does_not_have() {
        let mut odd = scroll("é", 4);
        let mut known = scroll("?", 4);
        for ms in [40, 200, 1000, 2000] {
            assert_eq!(
                render(&mut odd, &at(ms)),
                render(&mut known, &at(ms)),
                "{ms}"
            );
        }
        assert_ne!(render(&mut odd, &at(200)), black());
    }

    // A message finishes after its passes.

    #[test]
    fn finish_after_the_passes() {
        let s = scroll("HI", 4);
        assert!(!s.is_finished(&at(12_479)));
        assert!(s.is_finished(&at(12_480)));
    }

    #[test]
    fn next_change_is_the_next_step() {
        let s = scroll("HI", 4);
        let expected: Timestamp = "2026-11-12T12:00:01.04Z".parse().unwrap();
        assert_eq!(s.next_change(&at(1010)), Some(expected));
        let start: Timestamp = T.parse().unwrap();
        assert_eq!(
            s.next_change(&at(0)),
            Some(start + SignedDuration::from_millis(40))
        );
    }

    #[test]
    fn last_change_is_the_finish() {
        let s = scroll("HI", 1);
        let finish: Timestamp = "2026-11-12T12:00:03.12Z".parse().unwrap();
        assert_eq!(s.next_change(&at(3100)), Some(finish));
        assert_eq!(s.next_change(&at(3120)), None);
    }

    // A message uses the brightness of the clock schedule.

    fn utc(s: &str) -> Zoned {
        s.parse::<Timestamp>().unwrap().to_zoned(TimeZone::UTC)
    }

    #[test]
    fn message_at_night() {
        let s = scroll("HI", 4);
        assert_eq!(
            s.brightness(&utc("2026-11-12T01:00:00Z")),
            Brightness::new(25)
        );
    }

    #[test]
    fn message_in_the_morning() {
        let s = scroll("HI", 4);
        assert_eq!(
            s.brightness(&utc("2026-11-12T07:00:00Z")),
            Brightness::new(100)
        );
    }

    // Messages wait in a queue.

    fn msg(text: &str) -> Message {
        Message::new(text, 4, Message::DEFAULT_COLOR).unwrap()
    }

    #[test]
    fn queue_is_first_in_first_out() {
        let mut q = Queue::new();
        q.push(msg("one")).unwrap();
        q.push(msg("two")).unwrap();
        assert_eq!(q.len(), 2);
        assert_eq!(q.pop().map(|m| m.text().to_owned()), Some("one".into()));
        assert_eq!(q.pop().map(|m| m.text().to_owned()), Some("two".into()));
        assert_eq!(q.pop(), None);
        assert!(q.is_empty());
    }

    #[test]
    fn queue_holds_8_messages() {
        let mut q = Queue::new();
        for n in 0..8 {
            q.push(msg(&n.to_string())).unwrap();
        }
        assert_eq!(q.push(msg("nine")), Err(QueueFull));
        assert_eq!(q.len(), 8);
        assert_eq!(
            QueueFull.to_string(),
            "the message queue is full. 8 messages wait."
        );
    }

    #[test]
    fn clear_empties_the_queue() {
        let mut q = Queue::new();
        q.push(msg("one")).unwrap();
        q.push(msg("two")).unwrap();
        q.clear();
        assert_eq!(q.len(), 0);
        assert_eq!(q.pop(), None);
    }
}
