# Proposal

## Why

The legacy clock has two alerts: a full-screen **panic** and a **bedtime**
alert. It also warns on bedtime nights from 23:00 to 23:15. The legacy code
has a special mode for each alert. The new daemon must give these behaviors
from one alert capability, so that a new alert is data and not new code.

## What Changes

- Add an **alert definition** in the core. An alert has:
  - a **background effect**: black, or a color wheel
  - a **text** with a **style**: the font, the position, the color, and a
    still or scrolling motion
  - a **duration**: the longest time that the alert shows. A scrolling
    alert also finishes after its passes, if that is earlier.
  - a **priority**
  - **interrupt rules**: which items on the panel it can interrupt, and if
    `clear` removes it
- Add one generic **alert scene**. It renders any alert definition. It uses
  the scroll and the text parts of `message-board`, and a new shared
  **effect** part (the color wheel and the contrast text color).
- Three alerts are **built-in data** in a table, as `Schedule::legacy()` is:
  - `panic`: a color-wheel fill with `PANIC!!` in black or white for
    contrast, for 20 s. Highest priority. `clear` removes it.
  - `bedtime`: `GAME OVER - TURN IT OFF!` scrolls in lime `(50,205,50)` in
    the 10×20 font for 2 passes, or 20 s at most. `clear` does not remove it.
  - `bed-flash`: `BED!!!!!` in red for 1 s. Its priority is below a message,
    so it does not interrupt a message. `clear` does not remove it.
- **Priorities.** A message has a fixed priority. An alert starts only if
  its priority is higher than the priority of the item on the panel. An
  alert with a lower or the same priority gets an error reply. If the same
  alert shows already, nothing changes.
- **An interrupted message restarts.** It goes back to the front of the
  queue. It starts again from the right edge when the alert finishes.
  Messages that arrive during an alert wait in the queue.
- Add **timed triggers**, also as built-in data. On bedtime nights (Sunday
  to Thursday), the daemon starts `bed-flash` each 10 s from 23:00 to 23:05,
  and `bedtime` each 10 s from 23:05 to 23:16. The daemon wakes at the next
  trigger time.
- Add the **`alert` command**:
  - `alert <name>` starts the named alert.
  - `alert stop` removes the alert on the panel, also an alert that `clear`
    does not remove.
- `clear` empties the queue and removes the message. It removes an alert
  only if the alert's rule permits it.
- A pure core **board** holds the queue and knows what is on the panel. It
  decides what shows next. `run` holds the board, so the daemon still owns
  the queue (ADR-0017).
- **BREAKING** (protocol): the `status` reply has one more field,
  `alert=<name>` or `alert=none`. The unknown-command reply names six
  commands.

## Capabilities

### New Capabilities

- `alerts`: the alert definition, the alert scene and its effects, the
  built-in alerts, priorities and interrupt rules, the timed triggers, and
  the `alert` command.

### Modified Capabilities

- `message-board`: messages wait while an alert shows. An interrupted message
  restarts. `clear` keeps an alert that its rule protects.
- `command-input`: the unknown-command reply lists `alert`. The `status`
  reply gives the alert on the panel.

## ADRs

- Uses these ADRs:
  - ADR-0005 (pure core) and ADR-0006 (render on change)
  - ADR-0012 (scenes and compositor) and ADR-0013 (time and deadline wait)
  - ADR-0014 (scene brightness and schedule)
  - ADR-0016 (command socket) and ADR-0017 (messages and the daemon queue)
- **New ADR-0018: Alerts are data, with priorities and timed triggers.** It
  decides that alerts are definitions of one generic type, not code for each
  alert. It decides the priority and interrupt model, the restart of an
  interrupted message, and the pure board that `run` holds. It also decides
  that the daemon wakes for timed triggers.

No new dependency. `FONT_10X20` is in `embedded-graphics`, which the project
uses now.

## Expected effect on the Pi

- `panic` fills all 2,048 pixels and renders each 50 ms (20 times each
  second). `bedtime` scrolls each 40 ms (25 times each second), as a message
  does. We expect the main thread CPU to be near the message measurement
  (6.3%). The budget is 15%.
- The refresh rate must stay at 170 Hz or more during each alert.
- Outside the trigger windows, the triggers add no wake: the next trigger is
  hours away. With the clock only, the main thread still wakes once each
  minute.
- RSS: the definitions are static data. We expect no measurable change.
- **Measurement:** `scripts/measure.sh` for 60 s while `panic` repeats, and
  again while `bedtime` repeats: per-thread CPU %, voluntary context
  switches, and RSS. The refresh rate with `--led-show-refresh` during each
  alert. The user looks at the panel for flicker, the wheel, and the text.

## Out of scope

- Alert definitions and trigger rules from a config file. They move to
  config with `configurable-schedule`.
- The legacy `texgyre-27` font. `bedtime` uses `FONT_10X20` until BDF font
  loading.
- The legacy `brightness` command, and the legacy command names `panic88`
  and `bedtime` in the semaphore file.
- An alert on top of the clock, layers, and transitions.
- Alerts that wait for the panel. A refused alert is not queued.

## Impact

- `gameclock-core`: new modules `effect` (color wheel, contrast color),
  `alert` (definition, scene, built-in table, triggers), and `board` (queue,
  current item, priorities). The scroll position math of `message` becomes
  a shared part.
- `gameclockd`: the `alert` command and its errors, the board in `run`, the
  trigger wake, and the `alert=` field of `status`.
- Docs: ADR-0018, the ADR index, `architecture.md`, `glossary.md`,
  `roadmap.md`, `CHANGELOG.md`,
  and the README.
