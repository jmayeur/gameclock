# Design

## Context

- The compositor holds one base scene and a stack of foreground scenes. It
  renders only the top scene. `update` removes finished scenes, then renders
  (ADR-0012). Scenes go on the stack as `Box<dyn Scene>`. After a push, the
  daemon has no handle to the scene.
- `run` owns the compositor. It handles each command in the loop and replies
  after it presents (ADR-0016). `Command` is `Copy` and has no data.
- `parse_command` matches the whole trimmed line. `ctl` takes exactly one
  command argument.
- The clock draws `FONT_7X13_BOLD` at baseline y = 20 (ADR-0014). Each glyph
  of that font is 7 pixels wide, with no space between glyphs. A character
  that the font does not have is drawn as `?`.
- `Schedule::legacy()` gives the color and the brightness for a time. It
  builds a `Vec` of rules.
- The compositor caps renders at 30 each second (`MIN_INTERVAL`, 33 ms).
- On the Pi, the refresh thread uses 49% CPU, and the system is 45% idle
  (`command-input` measurements).

## Goals / Non-Goals

**Goals:**

- A smooth scroll at a fixed speed, also when a wake is late.
- No heap allocation for each frame while a message scrolls.
- Message, scroll, and queue parts that `alerts` can use again.
- No change to the idle behavior: with no message, the main thread wakes once
  each minute.

**Non-Goals:**

- A message on top of the clock (a ticker), or more than one visible scene.
- Priorities in the queue. That is for `alerts`.
- Options for speed, position, font, or brightness.

## Decisions

### 1. The position comes from the time (ADR-0017)

The scroll step is `STEP = 40 ms` for each pixel. A `Scroll` holds its start
time. For a time `now`:

- `steps = (now - start) / STEP`, rounded down.
- `pass = width + text_width`. `width` is the panel width, and `text_width`
  is 7 pixels for each character.
- `x = width - (steps % pass)`. The text is drawn at `(x, 20)`.
- The scroll is finished when `steps >= passes × pass`.
- `next_change` is `start + (steps + 1) × STEP`, or the finish time if that
  is earlier.

A late wake does not slow the scroll. A wake that is 40 ms late or more
skips a pixel. A shorter delay skips nothing: 40 ms is longer than
`MIN_INTERVAL`, so the next step can still render in time. The legacy clock
moved 1 pixel for each loop of 30 ms plus the draw time.

Alternative: move 1 pixel for each render, as the legacy clock did. Then the
speed depends on the CPU load, and a test needs a count of renders.

### 2. The message parts are in the core

A new module `gameclock_core::message` has:

- `Message`: the text (a `String`), the pass count, and the color.
  `Message::new` checks the limits and gives a `MessageError`.
  `Message::DEFAULT_PASSES` is 4, and `Message::DEFAULT_COLOR` is
  `(0,150,0)`.
- `Scroll`: a `Scene` made from a `Message`, a start time, the panel width,
  and a `Schedule`. Its brightness is `schedule.style_at(now).brightness`.
- `Queue`: a first-in, first-out queue of at most 8 `Message`s, on a
  `VecDeque`. `push` gives `QueueFull` when 8 messages wait.

`render` draws `&self.text` with `Text::with_baseline`. It allocates
nothing. The `String` and the `Schedule` are allocated once for each message,
not for each frame.

### 3. The daemon owns the queue (ADR-0017)

`run` holds a `Queue` next to the compositor. The stack holds at most one
`Scroll`. Each loop does these steps in order:

1. Read `now`.
2. `compositor.remove_finished(&now)`.
3. If the stack is empty, take the first message from the queue. Push a
   `Scroll` that starts at `now`.
4. `update` and present.
5. Wait until the deadline or an event.

Step 2 lets the next message start at the same update as the finish. The
panel then shows no clock frame between two messages.

A `message` command with an empty stack does steps 3 and 4, then replies.
With a message on the panel, it puts the message in the queue, or replies
with the queue-full error. `clear` empties the queue, calls
`compositor.clear_foreground()`, updates, presents if there is a frame, and
replies.

Alternative: one long-lived board scene on the stack that holds the queue.
The daemon then needs a shared handle (`Rc<RefCell<…>>`) to add messages to
a scene that the compositor owns.

### 4. Two new compositor operations

- `remove_finished(&mut self, now: &Zoned)` removes the finished foreground
  scenes. `update` calls it too, so `update` does not change.
- `clear_foreground(&mut self)` removes all foreground scenes. The next
  update renders the base scene. That frame is different from the message
  frame, so it is given.

### 5. Message brightness follows the legacy schedule (ADR-0017)

Each `Scroll` gets `Schedule::legacy()`. The test-pattern mode does not
change this. When `configurable-schedule` comes, the daemon gives the
configured schedule to the clock and to each `Scroll`.

### 6. Command syntax

`Command` gets two cases: `Message(Message)` and `Clear`. `Command` is no
longer `Copy`.

- The first word of the trimmed line selects the command. `clock`,
  `test-pattern`, `status`, and `clear` take no more words. Any other text
  after them is an unknown command, as now.
- `message` reads options while the next word starts with `--`:
  `--passes=N` and `--color=RRGGBB`. A later option of the same name wins.
  The rest of the line is the text, with its inner spaces. The length is in
  characters, not bytes.
- Each error has a fixed reply (the `message-board` spec). The reply to a
  correct `message` is `ok`.

### 7. The status count

`messages=<n>` is `queue.len() + compositor.foreground_len()`. Only
messages go on the stack in this change.

### 8. The `ctl` word join

`parse_ctl` reads `--control-socket=<path>` options until the first other
argument. It joins that argument and all later arguments with one space.
No argument after the options is a usage error, as now.

## Risks / Trade-offs

- **Scroll speed.** 40 ms is a guess from the legacy loop. The user checks it
  on the panel. A change is one constant.
- **CPU while a message scrolls.** 25 renders and presents each second share
  the core with the refresh thread. Each present copies 2,048 pixels and
  waits for the vertical sync. The on-device measurement checks the 15%
  budget and the refresh rate. If the budget fails, a frame budget for a
  smaller dirty area is the next step.
- **Late wakes.** A wake that is 40 ms late or more skips a pixel. The user
  looks for jerks on the panel.
- **`clear` removes all foreground scenes.** In this change, only messages
  are foreground scenes. `alerts` must decide if `clear` also removes an
  alert.
- **Legacy schedule in the message.** The brightness comes from the
  hard-coded schedule until `configurable-schedule`.
- **Characters outside ASCII** show as `?`. A message in another script is
  not readable. BDF fonts are a later candidate.
