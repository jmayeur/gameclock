# Proposal

## Why

The daemon now shows the heartbeat, a diagnostic image. The panel's job is to
show the time. The clock is the base scene of all later work: messages and
alerts come on top of it. The legacy clock sets its color and brightness from
hours in code, and the family edited the code to tune them. The new clock
keeps the legacy look, but the schedule becomes data.

## What Changes

- Add a **clock scene** to `gameclock-core`. It shows the local time as
  `09:05PM` in a 7×13 bold font at x=8, baseline y=20, as the legacy clock
  does. It changes only at each new local minute.
- Add a **time-of-day schedule** to `gameclock-core`. It is a list of rules.
  Each rule has a start and end time of day, an optional set of weekdays, a
  text color, and a brightness. The first rule that matches the local time is
  used. The legacy table is the default schedule, with its bedtime-night rule
  (Sunday to Thursday, 23:00 to 23:14, brightness 100).
- **Scenes give a brightness.** The `Scene` contract gets a brightness for a
  time, with full brightness as the default. The compositor gives the
  brightness with the frame. It gives a frame to present when the image or the
  brightness changes.
- The daemon presents each frame at the **brightness that the compositor
  gives**, not always at full brightness.
- The clock is the daemon's **base scene**. The new `--test-pattern` argument
  selects the heartbeat instead, to check the panel.
- **BREAKING** (behavior): the daemon shows the clock, not the heartbeat. It
  accepts one argument that is not a `--led-*` flag.
- The service unit passes **`--led-pwm-bits=8`** if the on-device
  measurements and a look at the clock colors at low brightness support it.

## Capabilities

### New Capabilities

- `clock-scene`: the clock face, the time format, the time-of-day schedule
  with its legacy defaults, and when the clock image changes.

### Modified Capabilities

- `scene-model`: scenes give a brightness. The compositor gives a frame to
  present when the image or the brightness changes.
- `daemon-lifecycle`: the clock is the base scene, the daemon presents at the
  scene's brightness, and `--test-pattern` selects the heartbeat. The
  argument rule then accepts `--test-pattern`.
- `heartbeat-scene`: the heartbeat is a diagnostic mode, not a temporary base
  scene. It always uses full brightness.
- `pi-deployment`: the service shows the clock. The unit passes the PWM-bits
  value that the measurements select.

## ADRs

- Uses ADR-0005 (pure core), ADR-0006 (render on change), ADR-0009 and
  ADR-0010 (deployment and the service unit), ADR-0012 (scenes), and ADR-0013
  (injected time).
- **New ADR-0014: Scene brightness and the time-of-day schedule.** It changes
  the `Scene` trait boundary, and it makes the schedule data in the core.
- **New ADR-0015: PWM bits for the clock.** It records a hardware and
  deployment choice from on-device numbers. If the numbers do not support 8
  bits, the ADR records that choice, and the unit stays as it is.

No new dependency. The font is `FONT_7X13_BOLD` from `embedded-graphics`,
which is already a dependency.

## Expected effect on the Pi

- The main thread wakes once each minute, not once each second. Its CPU use
  then goes down from 0.2% to near 0%. Its voluntary context switches go down
  from about 120 to about 1 to 5 in 60 s.
- RSS and binary size change by a small amount: one font table and the
  schedule.
- With `--led-pwm-bits=8`, bring-up measured 157 Hz instead of 147 Hz, and
  49.5% process CPU instead of 51.9%.
- **Measurement:** `scripts/measure.sh` for 60 s with the clock, with the
  default PWM bits and with 8 bits. Record CPU %, per-thread CPU %, voluntary
  context switches, RSS, refresh Hz, and binary size in the change.

## Out of scope

- The `BED!!!!!` flash and the scrolling bedtime alert. They come with
  `alerts`.
- A schedule from a configuration file, and an admin page. They come with
  `configurable-schedule`.
- A wrong time at boot before the first NTP sync. The clock shows the system
  time.
- A faster correction when the system clock steps (ADR-0013, `timerfd`).
- Other time formats, fonts, and positions.

## Impact

- `gameclock-core`: new `clock` and `schedule` modules. `Scene` and
  `Compositor` change. `Heartbeat` gets the default brightness.
- `gameclockd`: `run` presents at the compositor's brightness. `parse_args`
  and `main` select the base scene.
- `deploy/gameclockd.service`: maybe `--led-pwm-bits=8`.
- Docs: ADR-0014, ADR-0015, the ADR index, `architecture.md`, `glossary.md`,
  `roadmap.md`, `docs/hardware/pi-zero-w.md`, `CHANGELOG.md`, and the README
  status line.
