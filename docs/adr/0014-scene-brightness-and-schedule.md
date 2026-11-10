# ADR-0014: Scene brightness and the time-of-day schedule

- **Status:** Accepted
- **Date:** 2026-11-10
- **Related:** ADR-0005, ADR-0006, ADR-0012. OpenSpec change `clock-scene`.

## Context

The clock changes its color and its brightness with the time of day. The
legacy clock does this with hours in code, and the family edited the code to
tune them. Later alerts, for example Panic, need their own brightness over
the clock.

The `Scene` trait of ADR-0012 gives only pixels. The daemon presents each
frame at full brightness. A `Panel` already takes a brightness for each
present (ADR-0005).

## Decision

- **`Scene::brightness(&self, now) -> Brightness`**, with a default body that
  gives full brightness. A scene with no opinion does not change.
- **The compositor gives the top scene's brightness with the frame.** It
  gives a frame to present when the image or the brightness changed. The
  daemon presents at that brightness.
- **The schedule is data in `gameclock-core`.** A schedule is a list of rules
  and one default style. A rule has a start and an end time of day in whole
  minutes, a set of weekdays, a color, and a brightness. The first rule that
  matches the local time gives the style.
- **The legacy table is the default schedule.** It is built in code from
  these types. A later change can read the same types from a file.

## Consequences

- Each scene sets its own brightness. An alert can be brighter or darker than
  the clock, and the brightness comes back when the alert finishes.
- A brightness change with no pixel change still causes one present. The
  hzeller panel sets the driver's brightness only when the value changes.
- Rule times are whole minutes. So a style changes only at a minute boundary,
  and the clock needs no wake other than its minute wake.
- A rule cannot cross midnight. Such a period is two rules.
- The default body can hide a scene that forgets to set its brightness. Each
  scene spec names the brightness of the scene.

## Alternatives considered

- **The daemon applies the schedule.** The daemon then knows about the clock,
  and an alert cannot set its own brightness.
- **`render` gives the brightness.** Each scene then changes, and each scene
  must give a value.
- **The brightness is part of `Frame`.** A frame is only pixels, and a `Panel`
  takes the brightness as a separate value.
- **`jiff::civil::Time` for rule times.** It allows seconds, and it cannot
  show the end of the day as 24:00.
