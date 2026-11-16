# Design

## Context

- The compositor holds one base scene and a stack of foreground scenes as
  `Box<dyn Scene>`. After a push, the daemon has no handle to a scene
  (ADR-0012).
- `run` holds the message `Queue`. Its helper `show` removes finished scenes,
  starts the next message when the stack is empty, and presents a changed
  frame. The stack holds at most one `Scroll` (ADR-0017).
- `Scroll` holds the scroll math: `steps`, `pass`, `total`, `x`, and the next
  step. Its font is `FONT_7X13_BOLD` at baseline y = 20.
- `Schedule` has `Weekdays`, `TimeOfDay` (minutes of the day), and
  `Schedule::legacy()`, which builds a `Vec` of rules. Bedtime nights are
  Sunday to Thursday.
- `run` waits until `compositor.deadline(now)` (ADR-0013). With only the
  clock, it wakes once each minute.
- The legacy v2 code (`v2/clock.py`) gives the exact effects:
  - The panic wheel position is `int(t × 60) % 765`. It has three segments,
    and each goes from 0 to 255. The text is black if the luminance is more
    than 145. `PANIC!!` is at (9, 20). The alert shows for 20 s.
  - Bedtime scrolls 2 passes or 20 s, whichever is earlier.
  - The warning flashes `BED!!!!!` at (8, 20) when `second % 10 == 0`, in
    minutes 0–4 of 23:00.

## Goals / Non-Goals

**Goals:**

- One alert scene type for all alerts. A new alert is one more table entry.
- The scroll math and the effects are shared parts, not copies.
- The rules for what shows (queue, priorities, interrupts, clear) are pure
  and testable in the core with injected time.
- No heap allocation for each frame while an alert shows.
- No new wake outside the trigger windows.

**Non-Goals:**

- A config format for alerts or triggers.
- More than one visible item, or an alert that waits for the panel.
- Exact legacy timing for the wheel. The new wheel starts at position 0 at
  the alert start. The legacy wheel used the wall clock.

## Decisions

### 1. An alert is a static definition (ADR-0018)

A new module `gameclock_core::alert` has the definition type:

- `Alert`: `name`, `background`, `text`, `style`, `duration`, `priority`,
  and `clear`. All fields are `'static` data, so the built-in table is a
  `const` slice with no allocation.
- `Background`: `Black` or `ColorWheel`.
- `TextStyle`: `font` (`Small` = `FONT_7X13_BOLD`, `Large` = `FONT_10X20`),
  `baseline`, `color`, and `motion`.
- `TextColor`: `Fixed(Rgb888)` or `Contrast` (black or white on the
  background color).
- `Motion`: `Still { x }` or `Scroll { passes }`.
- `ClearRule`: `Removable` or `Kept`.
- `Priority(u8)`, with `Priority::BASE = 0` and `Priority::MESSAGE = 20`.

The built-in table, `alert::BUILT_IN`:

| Name | Background | Text | Style | Duration | Priority | Clear |
|---|---|---|---|---|---|---|
| `panic` | color wheel | `PANIC!!` | small, still x = 9, y = 20, contrast | 20 s | 40 | removable |
| `bedtime` | black | `GAME OVER - TURN IT OFF!` | large, scroll 2 passes, y = 25, lime | 20 s | 30 | kept |
| `bed-flash` | black | `BED!!!!!` | small, still x = 8, y = 20, red | 1 s | 10 | kept |

`alert::find(name)` looks up a definition. The error replies list the names
from the table, so a new entry also shows in the replies.

Alternative: an enum with one variant for each alert, and a `match` in the
renderer. Each new alert then needs new code.

### 2. One alert scene renders each definition

`AlertScene::new(alert: &'static Alert, start, width, schedule)` implements
`Scene`:

- `render` fills the background, then draws the text. A still text is drawn
  at `(x, baseline)`. A scrolling text uses the shared scroll path (decision
  3).
- `is_finished` is true at `start + duration`, or after the last pass of a
  scroll, whichever is earlier.
- `next_change` is the earliest of: the next wheel step (color wheel only),
  the next scroll step (scroll only), and the finish time. A black, still
  alert gives only its finish time, so `bed-flash` renders once.
- `brightness` is `schedule.style_at(now).brightness`, as for a message
  (ADR-0017).

### 3. Shared parts: the scroll path and the effects

- `message::ScrollPath` holds `width`, `pass`, and `total`. It gives
  `steps(start, now)`, `x(steps)`, and `next_step(start, steps)`. It takes
  the glyph width of the font, so the 10×20 font works. `Scroll` and
  `AlertScene` both use it. The `message-board` tests do not change.
- A new module `gameclock_core::effect` has:
  - `WHEEL_STEP = 50 ms` and `wheel_position(steps) = (3 × steps) % 765`.
    This gives the legacy speed of 60 positions each second.
  - `wheel(position) -> Rgb888`, with the legacy three segments.
  - `contrast(color) -> Rgb888`: black if `299R + 587G + 114B > 145,000`,
    otherwise white. This is the legacy rule in integers.

The wheel steps each 50 ms, so `panic` renders 20 times each second. A step
of 33 ms would give the legacy 30 renders, at more CPU cost. We measure 50 ms
first (task 9).

### 4. A pure board decides what shows (ADR-0018)

A new module `gameclock_core::board` has `Board`. `run` holds it, so the
daemon still owns the queue (ADR-0017). It holds:

- the message `Queue`
- `showing`: `Nothing`, `Message(Message)`, or `Alert(&'static Alert)`
- the panel width and the `Schedule` for new scenes

Each method takes `&mut Compositor` and `now`. The compositor stack holds at
most one scene, and `showing` describes it:

- `advance`: remove finished scenes. If the stack is empty, set `showing` to
  `Nothing` and start the first queued message.
- `message(m)`: put `m` in the queue, then `advance`.
- `alert(a)`: `advance` first. If `a` is the alert that shows, do nothing.
  If the priority of `a` is not higher than the priority of `showing`, give
  `Refused { alert, by }`. Otherwise, remove the scene on the stack. If it
  was a message, put the message back at the front of the queue
  (`Queue::put_back`, which can exceed 8). Then push the alert scene.
- `stop_alert`: if an alert shows, remove it and `advance`.
- `clear`: empty the queue. Remove a message, or an alert whose rule is
  `Removable`. Then `advance`.
- `messages()` and `alert_name()` for `status`.

`Board` keeps a clone of the `Message` that shows, so that it can put it
back. The clone is made once for each message start, not for each frame.

The daemon's `show` helper becomes `board.advance` and then the present.

Alternative: keep the logic in `run`, with a second `Option` for the alert.
The priority and restart rules then have tests only through the daemon
loop.

### 5. Timed triggers are data, and the daemon wakes for them (ADR-0018)

`alert::Trigger` has `days: Weekdays`, `from` and `to: TimeOfDay`, `every:
SignedDuration`, and `alert: &'static Alert`. A fire time is `from + k ×
every` that is before `to`, on a day in `days`, in the local zone of `now`.
`Weekdays::of` is not `const`, so `Trigger::legacy()` builds a `Vec`, as
`Schedule::legacy()` does. It has two rules:

- bedtime nights, 23:00 to 23:05, each 10 s: `bed-flash`
- bedtime nights, 23:05 to 23:16, each 10 s: `bedtime`

`alert::Triggers` holds the rules and the next fire time of each rule:

- `Triggers::new(rules, now)` sets each next time to the first fire time at
  or after `now`.
- `due(now)` gives each alert whose next time is at or before `now`. It then
  sets that rule's next time to the first fire time after `now`. A late wake
  so gives one start for each rule, not one for each missed time.
- `next()` gives the earliest next time.

The `run` loop for each wake:

1. `board.advance(now)`. A finished `bedtime` is then gone before its next
   trigger, so the next `bedtime` starts with no gap.
2. For each alert from `triggers.due(now)`, call `board.alert`. A refusal
   changes nothing and gets no log line.
3. Present a changed frame.
4. Wait until the earlier of `compositor.deadline(now)` and
   `triggers.next()`.

The triggers run in both modes, `clock` and `test-pattern`.

Alternative: a trigger scene in the compositor. A scene cannot start
another scene, and the board must see the trigger to apply the priorities.

### 6. The command protocol

- `alert <name>` and `alert stop` in `parse_command`. The rest of the line
  after `alert` is the name, so `alert fire now` is the unknown name `fire
  now`.
- `Command::Alert(&'static Alert)` and `Command::StopAlert`.
- `CommandError::NoAlertName` and `CommandError::UnknownAlert(String)`.
- A refusal gives `error: the alert '<a>' did not start. <item> on the panel
  has the same or a higher priority.` The item is `A message` or `The alert
  '<b>'`.
- `status_reply` gets the alert name. The reply ends with `alert=<name>` or
  `alert=none`.

## Risks / Trade-offs

- [On a bedtime night, a message from 23:05 to 23:16 waits. It starts when
  the last `bedtime` finishes, at about 23:16:10] → This is the intent of
  the warning. `alert stop` removes the current `bedtime`, but the next trigger
  starts it again in 10 s or less.
- [The board refuses `bed-flash` while a message scrolls. A long message
  so hides the flash] → This is the legacy behavior: the legacy flash was part of the
  clock render.
- [9 messages can wait after an interrupt] → `put_back` permits it. A new
  `message` then gets the queue-full reply. The limit stays at 8 for new
  messages.
- [A time zone that skips or repeats 23:00 to 23:16] → The fire times use
  jiff's compatible conversion. No zone in use has a change at that time.
- [The wheel at 20 renders each second costs more CPU than a message] → The
  on-device measurement (task 9) verifies the budget. If it fails, we make the
  step longer before we merge.

## Migration Plan

- The Pi keeps the legacy clock as the boot service. `scripts/deploy.sh`
  installs the new binary for the measurements, as for `message-board`.
- Rollback: deploy the earlier binary. No state is stored.
