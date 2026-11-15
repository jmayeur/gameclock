# Proposal

## Why

The legacy clock can scroll a text message across the panel. The new daemon
has the control socket (`command-input`), but no command shows text. Messages
are the next roadmap item. The `alerts` change needs the same scroll, color,
and pass-count parts.

## What Changes

- Add a **message scene** in the core. It scrolls one line of text from the
  right edge to the left edge, 1 pixel at a time, for a number of passes. It
  uses the 7×13 bold font at the clock's baseline. Its position comes from
  the injected time, not from a frame count.
- A message has a **text** (1 to 64 characters), a **pass count** (1 to 10,
  default 4), and a **color** (default legacy green `(0,150,0)`).
- A message shows at the **brightness of the clock schedule** for the current
  time, as in the legacy clock.
- Add a **message queue** of 8 messages, first in, first out. The next
  message starts when the current message finishes. The base scene shows
  again when the queue is empty.
- Add two **commands**:
  - `message [--passes=N] [--color=RRGGBB] <text>` puts a message in the
    queue. If no message is on the panel, it starts at once.
  - `clear` empties the queue and removes the message on the panel.
- `status` also gives the number of messages: `messages=<n>`.
- `gameclockd ctl` joins the words after the command name with one space.
  Then `gameclockd ctl message hello world` works with no quotes.
- The compositor can remove finished scenes before an update, and can remove
  all foreground scenes.
- **BREAKING** (protocol): the `status` reply has one more field. The
  unknown-command reply names five commands. `ctl` with more than one word is
  no longer a usage error.

## Capabilities

### New Capabilities

- `message-board`: the message, its scroll, its brightness, the queue, and
  the `message` and `clear` commands.

### Modified Capabilities

- `command-input`: the unknown-command reply lists the new commands. The
  `status` reply gives the number of messages. The `ctl` client joins its
  command words.
- `scene-model`: the compositor removes finished scenes when it is told to.
  It can also remove all foreground scenes.

## ADRs

- Uses ADR-0005 (pure core), ADR-0006 (render on change), ADR-0012 (scenes
  and compositor), ADR-0014 (scene brightness and schedule), and ADR-0016
  (command socket and event channel).
- **New ADR-0017: Messages scroll from the injected time, and the daemon owns
  the queue.** It adds command arguments to the line protocol of ADR-0016. It
  decides where the queue lives, how the scroll gets its position, and which
  brightness a message uses.

No new dependency. The font is `FONT_7X13_BOLD` from `embedded-graphics`,
which the clock already uses.

## Expected effect on the Pi

- While a message scrolls, the main thread renders and presents 25 times
  each second. Each frame clears 2,048 pixels, draws the text, compares two
  frames, and copies the frame to the driver. We expect about 5% CPU for the
  main thread. The budget is 15%.
- The refresh thread shares the core. Its rate must not drop below 170 Hz
  (176 Hz now) while a message scrolls.
- With no message, the main thread still wakes once each minute.
- RSS: the queue holds at most 9 messages of 64 characters. The effect is a
  few KiB.
- **Measurement:** `scripts/measure.sh` for 60 s while a long message
  scrolls: per-thread CPU %, voluntary context switches, and RSS. Then the
  refresh rate with `--led-show-refresh` while a message scrolls. The user
  looks at the panel for flicker and for a smooth scroll.

## Out of scope

- Alerts (Panic, Bedtime) and their effects (`alerts`).
- A message on top of the clock, as a ticker (later: layers and overlays).
- Other fonts, BDF fonts, and characters outside ASCII. A character that the
  font does not have shows as `?`.
- Speed, position, and brightness options for a message.
- The legacy semaphore file and the `+` to space rule of the web form.

## Impact

- `gameclock-core`: a new `message` module with the message, the scroll
  scene, and the queue. `Compositor::remove_finished` and
  `Compositor::clear_foreground`.
- `gameclockd`: the `message` and `clear` commands, the queue in `run`, the
  `messages=` field of `status`, and the `ctl` word join.
- Docs: ADR-0017, the ADR index, `architecture.md`, `glossary.md`,
  `roadmap.md`, `CHANGELOG.md`, and the README.
