# Design

## Context

- `scene-model` gave the `Scene` trait, the `Compositor`, and the event loop
  (ADR-0012, ADR-0013). The scene contract has no brightness. `run` presents
  each frame at `Brightness::MAX`.
- `Panel::present` already takes a `Brightness`. The hzeller panel calls the
  driver's brightness function only when the value changes
  (`rgbmatrix-sys/src/convert.rs`). So a brightness per frame costs nothing
  extra on the hot path.
- `embedded-graphics` 0.8.2 has `FONT_7X13_BOLD`, the same face as the legacy
  BDF font `7x13B`. No new dependency is necessary.
- hzeller reads its flags in order, and a later flag overwrites an earlier one
  (`vendor/rpi-rgb-led-matrix/lib/options-initialize.cc`, `ConsumeIntFlag`).
  So a default in the unit can come before the device's own flags.

## Goals / Non-Goals

**Goals:**

- The schedule is plain data in the core. `configurable-schedule` can then
  read the same types from a file, with no change to the clock.
- Each scene can set its brightness. An alert such as Panic can then use its
  own brightness over the clock.
- The clock renders and presents once each minute, and it allocates nothing
  for each frame.

**Non-Goals:**

- No configuration file, and no parser for the schedule.
- No transitions or fades between styles.
- No change to the wait or to the time source.

## Decisions

### 1. Brightness is a default method on `Scene` (ADR-0014)

`fn brightness(&self, now: &Zoned) -> Brightness` with a default body that
gives `Brightness::MAX`. The heartbeat and the test scenes do not change. The
compositor asks the top scene after it renders. `update` gives the frame and
the brightness together, for example `Option<(&Frame, Brightness)>`. It keeps
the last brightness next to the `shown` frame. It gives a frame when the
image or the brightness changed.

Alternatives:

- **The daemon applies the schedule itself.** Then the daemon knows about
  the clock, and an alert cannot set its own brightness.
- **`render` returns the brightness.** Each scene then changes, and a scene
  with no opinion must still say "full".
- **Brightness in `Frame`.** A `Panel` receives the brightness as a separate
  value now, and a frame is only pixels (ADR-0005).

### 2. The schedule types (ADR-0014)

All in a new `gameclock-core::schedule` module:

- `TimeOfDay`: minutes after midnight, `0..=1440`. `TimeOfDay::hm(h, m)`
  gives an error for a time that is not valid. `24:00` is the end of the day.
  Whole minutes keep each style change on a minute boundary. The clock then
  needs no extra wakes for the schedule.
- `Weekdays`: a bit set of the seven days. The empty set means each day.
- `Style { color: Rgb888, brightness: Brightness }`.
- `Rule::new(days, start, end, style)` gives an error if `start >= end`. A
  rule that crosses midnight is two rules.
- `Schedule { rules: Vec<Rule>, default: Style }`. `style_at(&Zoned)` scans
  the rules in order and gives the first match. The list has fewer than ten
  rules, so a linear scan is the fastest choice. The `Vec` is allocated once,
  when the daemon builds the schedule.
- `Schedule::legacy()` gives the table of `openspec/specs/clock-scene`.

Alternatives: `jiff::civil::Time` for rule times (it allows seconds and
cannot show `24:00`). A `match` on the hour (that is the hardcoded form that
the roadmap rejects).

### 3. The clock scene

`Clock { schedule: Schedule }` in `gameclock-core::clock`.

- `render`: get the style for `now`. Write the seven characters of
  `hh:mmAM` into a `[u8; 7]` on the stack. The hour is
  `(hour + 11) % 12 + 1`. Draw the text with `MonoTextStyle` and
  `Text::with_baseline` at (8, 20), `Baseline::Alphabetic`. No `String`, no
  `format!`, and no jiff `strftime`, because they allocate.
- `next_change`: the next whole minute of the timestamp. All current time
  zones have offsets in whole minutes, so this is also the next local minute.
  Timestamp arithmetic needs no time-zone lookup.
- `brightness`: the brightness of the style for `now`.
- `is_finished`: always false.

### 4. The daemon selects the base scene

`parse_args` gives a struct with the mode (clock or test pattern) and the
driver flags in their order. `main` builds `Clock::new(Schedule::legacy())`
or `Heartbeat`, writes one line that names the base scene, and calls `run`.
`run` keeps its signature and presents at the compositor's brightness.

### 5. PWM bits in the unit (ADR-0015)

`ExecStart=/usr/local/bin/gameclockd --led-pwm-bits=8 $GAMECLOCKD_ARGS`. A
device can set `--led-pwm-bits=11` in `/etc/default/gameclockd`, and the
later flag wins. ADR-0015 records the measured numbers. The numbers or the look
at low brightness can fail to support 8 bits. Then the ADR records that. The
`pi-deployment` delta and the unit then change back before the archive.

## Risks / Trade-offs

- [A time zone with an offset that is not whole minutes] → The clock wakes
  some seconds away from the local minute. No current tzdb zone has one. A
  test documents the assumption.
- [Fewer PWM bits make dark colors less smooth] → The clock uses a few
  saturated colors. The on-device scenario checks red and mellow at
  brightness 25 before ADR-0015 is accepted.
- [The time is wrong before the first NTP sync] → Out of scope. The clock
  shows the system time, and it is correct after the sync and the next
  minute.
- [The default method hides a missing brightness in a new scene] → Full
  brightness is the safe value for a diagnostic scene. Each real scene has a
  spec that names its brightness.

## Migration Plan

1. Merge the PR. Deploy with `scripts/deploy.sh`. The legacy clock stops when
   `gameclockd` starts.
2. Watch the clock for one schedule boundary. Compare it with the legacy
   clock's look.
3. Rollback: start the legacy clock service. The unit's `Conflicts=` stops
   `gameclockd`.
4. Enabling at boot stays a separate, explicit choice (`pi-deployment`). This
   change does not enable it.

## Open Questions

- Keep the heartbeat after `alerts`, or replace it with a full diagnostic
  scene? This does not change this change.
