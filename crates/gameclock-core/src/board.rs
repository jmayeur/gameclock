//! The board: the message queue, the item on the panel, and the rules for
//! what shows (`openspec/specs/alerts`, `openspec/specs/message-board`,
//! docs/adr/0018).
//!
//! The daemon holds the board, so the daemon owns the queue (docs/adr/0017).
//! The compositor stack holds at most one foreground scene, and the board
//! knows what it is.

use core::fmt;
use std::rc::Rc;

use jiff::Zoned;

use crate::Compositor;
use crate::alert::{Alert, AlertScene, ClearRule, Priority};
use crate::message::{Message, Queue, QueueFull, Scroll};
use crate::schedule::Schedule;

/// The item on the panel above the base scene.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Showing {
    Nothing,
    Message(Message),
    Alert(Rc<Alert>),
}

/// What blocked an alert.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Blocker {
    /// A message is on the panel.
    Message,
    /// This alert is on the panel.
    Alert(Rc<Alert>),
}

/// An alert did not start, because the item on the panel has the same or a
/// higher priority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refused {
    /// The alert that did not start.
    pub alert: Rc<Alert>,
    /// The item on the panel.
    pub by: Blocker,
}

impl fmt::Display for Refused {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "the alert '{}' did not start. ", self.alert.name)?;
        match &self.by {
            Blocker::Message => f.write_str("A message")?,
            Blocker::Alert(by) => write!(f, "The alert '{}'", by.name)?,
        }
        f.write_str(" on the panel has the same or a higher priority.")
    }
}

impl std::error::Error for Refused {}

/// Decides which message or alert shows on top of the base scene.
#[derive(Debug, Clone)]
pub struct Board {
    queue: Queue,
    showing: Showing,
    width: usize,
    schedule: Schedule,
}

impl Board {
    /// A board for a panel `width` pixels wide. Its scenes get their
    /// brightness from `schedule`.
    #[must_use]
    pub fn new(width: usize, schedule: Schedule) -> Self {
        Self {
            queue: Queue::new(),
            showing: Showing::Nothing,
            width,
            schedule,
        }
    }

    /// Remove the finished scenes. If nothing shows, start the first message
    /// of the queue at `now`.
    pub fn advance(&mut self, now: &Zoned, compositor: &mut Compositor) {
        compositor.remove_finished(now);
        if compositor.foreground_len() > 0 {
            return;
        }
        self.showing = Showing::Nothing;
        if let Some(message) = self.queue.pop() {
            // One clone for each message start, so that an alert can put the
            // message back.
            let scroll = Scroll::new(
                message.clone(),
                now.timestamp(),
                self.width,
                self.schedule.clone(),
            );
            compositor.push(Box::new(scroll));
            self.showing = Showing::Message(message);
        }
    }

    /// Put `message` at the end of the queue. It starts at once if nothing
    /// shows.
    ///
    /// # Errors
    /// [`QueueFull`] if the queue is full.
    pub fn message(
        &mut self,
        message: Message,
        now: &Zoned,
        compositor: &mut Compositor,
    ) -> Result<(), QueueFull> {
        self.queue.push(message)?;
        self.advance(now, compositor);
        Ok(())
    }

    /// Start `alert` at `now`, if its priority is higher than the priority of
    /// the item on the panel. An interrupted message goes back to the front
    /// of the queue. If `alert` shows now, nothing changes.
    ///
    /// # Errors
    /// [`Refused`] if the item on the panel has the same or a higher
    /// priority.
    pub fn alert(
        &mut self,
        alert: Rc<Alert>,
        now: &Zoned,
        compositor: &mut Compositor,
    ) -> Result<(), Refused> {
        self.advance(now, compositor);
        let (priority, by) = match &self.showing {
            Showing::Nothing => (Priority::BASE, None),
            Showing::Message(_) => (Priority::MESSAGE, Some(Blocker::Message)),
            Showing::Alert(shown) if shown.name == alert.name => return Ok(()),
            Showing::Alert(shown) => (shown.priority, Some(Blocker::Alert(Rc::clone(shown)))),
        };
        if let Some(by) = by
            && alert.priority <= priority
        {
            return Err(Refused { alert, by });
        }
        compositor.clear_foreground();
        let scene = AlertScene::new(
            Rc::clone(&alert),
            now.timestamp(),
            self.width,
            self.schedule.clone(),
        );
        if let Showing::Message(message) =
            core::mem::replace(&mut self.showing, Showing::Alert(alert))
        {
            self.queue.put_back(message);
        }
        compositor.push(Box::new(scene));
        Ok(())
    }

    /// Remove the alert on the panel, if one shows. Then start the next
    /// message.
    pub fn stop_alert(&mut self, now: &Zoned, compositor: &mut Compositor) {
        if matches!(self.showing, Showing::Alert(_)) {
            compositor.clear_foreground();
            self.showing = Showing::Nothing;
        }
        self.advance(now, compositor);
    }

    /// Empty the queue. Remove the message on the panel, or an alert that
    /// `clear` can remove.
    pub fn clear(&mut self, now: &Zoned, compositor: &mut Compositor) {
        self.queue.clear();
        let removable = match &self.showing {
            Showing::Nothing => false,
            Showing::Message(_) => true,
            Showing::Alert(alert) => alert.clear == ClearRule::Removable,
        };
        if removable {
            compositor.clear_foreground();
            self.showing = Showing::Nothing;
        }
        self.advance(now, compositor);
    }

    /// The number of messages on the panel and in the queue.
    #[must_use]
    pub fn messages(&self) -> usize {
        let showing = usize::from(matches!(self.showing, Showing::Message(_)));
        self.queue.len() + showing
    }

    /// The name of the alert on the panel.
    #[must_use]
    pub fn alert_name(&self) -> Option<&str> {
        match &self.showing {
            Showing::Alert(alert) => Some(&alert.name),
            Showing::Nothing | Showing::Message(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::alert::Alerts;
    use crate::clock::Clock;
    use crate::{Frame, Scene};
    use jiff::tz::TimeZone;
    use jiff::{SignedDuration, Timestamp};

    // 2026-11-16 is a Monday.
    const T: &str = "2026-11-16T12:00:00Z";

    fn ts(offset_ms: i64) -> Timestamp {
        T.parse::<Timestamp>().unwrap() + SignedDuration::from_millis(offset_ms)
    }

    fn at(offset_ms: i64) -> Zoned {
        ts(offset_ms).to_zoned(TimeZone::UTC)
    }

    fn setup() -> (Board, Compositor) {
        let compositor = Compositor::new(Box::new(Clock::new(Schedule::legacy())), 64, 32);
        (Board::new(64, Schedule::legacy()), compositor)
    }

    fn alert(name: &str) -> Rc<Alert> {
        Alerts::legacy().find(name).unwrap()
    }

    fn msg(text: &str) -> Message {
        Message::new(text, 4, Message::DEFAULT_COLOR).unwrap()
    }

    /// The frame of `scene` alone at `now`.
    fn alone(scene: Box<dyn Scene>, now: &Zoned) -> Frame {
        let mut c = Compositor::new(scene, 64, 32);
        c.update(now).map(|(f, _)| f.clone()).unwrap()
    }

    fn clock_at(now: &Zoned) -> Frame {
        alone(Box::new(Clock::new(Schedule::legacy())), now)
    }

    fn alert_at(name: &str, start: i64, now: &Zoned) -> Frame {
        let scene = AlertScene::new(alert(name), ts(start), 64, Schedule::legacy());
        alone(Box::new(scene), now)
    }

    fn scroll_at(text: &str, start: i64, now: &Zoned) -> Frame {
        let scene = Scroll::new(msg(text), ts(start), 64, Schedule::legacy());
        alone(Box::new(scene), now)
    }

    /// The frame on the panel at `now`, after the board advances. The image
    /// must change at `now`.
    fn shown(board: &mut Board, compositor: &mut Compositor, now: &Zoned) -> Frame {
        board.advance(now, compositor);
        compositor
            .update(now)
            .map(|(f, _)| f.clone())
            .expect("the image changes")
    }

    // An alert starts only above a lower priority.

    #[test]
    fn alert_over_the_clock() {
        let (mut b, mut c) = setup();
        b.advance(&at(0), &mut c);
        assert_eq!(b.alert(alert("panic"), &at(0), &mut c), Ok(()));
        assert_eq!(shown(&mut b, &mut c, &at(0)), alert_at("panic", 0, &at(0)));
        assert_eq!(b.alert_name(), Some("panic"));
    }

    #[test]
    fn panic_interrupts_bedtime() {
        let (mut b, mut c) = setup();
        b.alert(alert("bedtime"), &at(0), &mut c).unwrap();
        assert_eq!(b.alert(alert("panic"), &at(1000), &mut c), Ok(()));
        assert_eq!(b.alert_name(), Some("panic"));
        assert_eq!(shown(&mut b, &mut c, &at(21_000)), clock_at(&at(21_000)));
        assert_eq!(b.alert_name(), None);
    }

    #[test]
    fn lower_priority_is_refused() {
        let (mut b, mut c) = setup();
        b.alert(alert("panic"), &at(0), &mut c).unwrap();
        let err = b.alert(alert("bedtime"), &at(1000), &mut c).unwrap_err();
        assert_eq!(
            err.to_string(),
            "the alert 'bedtime' did not start. The alert 'panic' on the panel has the same or a higher priority."
        );
        assert_eq!(b.alert_name(), Some("panic"));
        assert_eq!(
            shown(&mut b, &mut c, &at(2000)),
            alert_at("panic", 0, &at(2000))
        );
    }

    #[test]
    fn bed_flash_does_not_interrupt_a_message() {
        let (mut b, mut c) = setup();
        b.message(msg("HI"), &at(0), &mut c).unwrap();
        let err = b.alert(alert("bed-flash"), &at(1000), &mut c).unwrap_err();
        assert_eq!(
            err.to_string(),
            "the alert 'bed-flash' did not start. A message on the panel has the same or a higher priority."
        );
        assert_eq!(b.alert_name(), None);
        assert_eq!(b.messages(), 1);
    }

    #[test]
    fn same_alert_again() {
        let (mut b, mut c) = setup();
        b.alert(alert("bedtime"), &at(0), &mut c).unwrap();
        assert_eq!(b.alert(alert("bedtime"), &at(5000), &mut c), Ok(()));
        assert_eq!(
            shown(&mut b, &mut c, &at(6000)),
            alert_at("bedtime", 0, &at(6000))
        );
    }

    // Messages wait during an alert, and an interrupted message restarts.

    #[test]
    fn message_waits_during_an_alert() {
        let (mut b, mut c) = setup();
        b.alert(alert("panic"), &at(0), &mut c).unwrap();
        assert_eq!(b.message(msg("hello"), &at(1000), &mut c), Ok(()));
        assert_eq!(b.alert_name(), Some("panic"));
        assert_eq!(b.messages(), 1);
        assert_eq!(
            shown(&mut b, &mut c, &at(1040)),
            alert_at("panic", 0, &at(1040))
        );
        // The daemon advances the board at the finish of `panic`.
        assert_eq!(shown(&mut b, &mut c, &at(20_000)), Frame::new(64, 32));
        assert_eq!(
            shown(&mut b, &mut c, &at(20_040)),
            scroll_at("hello", 20_000, &at(20_040))
        );
    }

    #[test]
    fn message_restarts_after_panic() {
        let (mut b, mut c) = setup();
        b.message(msg("HI"), &at(0), &mut c).unwrap();
        b.alert(alert("panic"), &at(1000), &mut c).unwrap();
        assert_eq!(b.messages(), 1);
        assert_eq!(shown(&mut b, &mut c, &at(21_000)), Frame::new(64, 32));
        assert_eq!(b.messages(), 1);
        assert_eq!(
            shown(&mut b, &mut c, &at(21_040)),
            scroll_at("HI", 21_000, &at(21_040))
        );
    }

    #[test]
    fn interrupted_message_goes_first() {
        let (mut b, mut c) = setup();
        b.message(msg("one"), &at(0), &mut c).unwrap();
        b.message(msg("two"), &at(0), &mut c).unwrap();
        b.alert(alert("panic"), &at(1000), &mut c).unwrap();
        assert_eq!(b.messages(), 2);
        assert_eq!(shown(&mut b, &mut c, &at(21_000)), Frame::new(64, 32));
        assert_eq!(
            shown(&mut b, &mut c, &at(21_400)),
            scroll_at("one", 21_000, &at(21_400))
        );
    }

    #[test]
    fn interrupted_message_with_a_full_queue() {
        let (mut b, mut c) = setup();
        b.message(msg("first"), &at(0), &mut c).unwrap();
        for n in 0..8 {
            b.message(msg(&n.to_string()), &at(0), &mut c).unwrap();
        }
        b.alert(alert("panic"), &at(1000), &mut c).unwrap();
        assert_eq!(b.messages(), 9);
        assert_eq!(b.message(msg("new"), &at(1000), &mut c), Err(QueueFull));
        assert_eq!(shown(&mut b, &mut c, &at(21_000)), Frame::new(64, 32));
        assert_eq!(
            shown(&mut b, &mut c, &at(21_400)),
            scroll_at("first", 21_000, &at(21_400))
        );
    }

    // Each alert has a clear rule.

    #[test]
    fn clear_removes_panic() {
        let (mut b, mut c) = setup();
        b.alert(alert("panic"), &at(0), &mut c).unwrap();
        b.message(msg("one"), &at(0), &mut c).unwrap();
        b.message(msg("two"), &at(0), &mut c).unwrap();
        b.clear(&at(1000), &mut c);
        assert_eq!(b.alert_name(), None);
        assert_eq!(b.messages(), 0);
        assert_eq!(shown(&mut b, &mut c, &at(1000)), clock_at(&at(1000)));
    }

    #[test]
    fn clear_keeps_bedtime() {
        let (mut b, mut c) = setup();
        b.alert(alert("bedtime"), &at(0), &mut c).unwrap();
        b.message(msg("one"), &at(0), &mut c).unwrap();
        b.message(msg("two"), &at(0), &mut c).unwrap();
        b.clear(&at(1000), &mut c);
        assert_eq!(b.alert_name(), Some("bedtime"));
        assert_eq!(b.messages(), 0);
        assert_eq!(
            shown(&mut b, &mut c, &at(1040)),
            alert_at("bedtime", 0, &at(1040))
        );
        assert_eq!(shown(&mut b, &mut c, &at(20_000)), clock_at(&at(20_000)));
    }

    #[test]
    fn clear_during_an_alert_that_clear_keeps() {
        let (mut b, mut c) = setup();
        b.alert(alert("bed-flash"), &at(0), &mut c).unwrap();
        b.message(msg("one"), &at(0), &mut c).unwrap();
        assert!(c.update(&at(0)).is_some());
        b.clear(&at(0), &mut c);
        assert!(c.update(&at(0)).is_none(), "the image does not change");
        assert_eq!(shown(&mut b, &mut c, &at(1000)), clock_at(&at(1000)));
        assert_eq!(b.messages(), 0);
    }

    #[test]
    fn stop_an_alert_that_clear_keeps() {
        let (mut b, mut c) = setup();
        b.alert(alert("bedtime"), &at(0), &mut c).unwrap();
        b.stop_alert(&at(1000), &mut c);
        assert_eq!(b.alert_name(), None);
        assert_eq!(shown(&mut b, &mut c, &at(1000)), clock_at(&at(1000)));
    }

    #[test]
    fn stop_starts_the_next_message() {
        let (mut b, mut c) = setup();
        b.alert(alert("bedtime"), &at(0), &mut c).unwrap();
        b.message(msg("hello"), &at(0), &mut c).unwrap();
        b.stop_alert(&at(1000), &mut c);
        assert_eq!(
            shown(&mut b, &mut c, &at(1040)),
            scroll_at("hello", 1000, &at(1040))
        );
    }

    #[test]
    fn stop_with_no_alert() {
        let (mut b, mut c) = setup();
        b.advance(&at(0), &mut c);
        assert!(c.update(&at(0)).is_some());
        b.stop_alert(&at(0), &mut c);
        assert!(c.update(&at(0)).is_none(), "the image does not change");
    }
}
