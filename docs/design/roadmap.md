# Roadmap

The planned OpenSpec changes, in order (ADR-0008). Each item becomes
`openspec/changes/<name>/` through `/opsx:propose` when we start it. The order
and scope can change. Update this file when a change merges or a plan changes.

## Foundation: drive the panel

1. **`panel-bring-up`**: **done** (PR #4). `rgbmatrix-sys` (hzeller FFI and
   `HzellerPanel`), the zig cross build, the systemd unit, and the deploy and
   measurement scripts. The baseline numbers for both clocks are in
   `openspec/changes/archive/2026-11-09-panel-bring-up/measurements.md`. Open
   items: the shimmer glitches (try the GPIO4↔18 jumper) and the 9% refresh
   gap to the legacy driver, which gcc built.
2. **`dev-simulator`**: **done**. `TerminalPanel` draws frames in the terminal
   (`cargo sim`), so you can see scenes without the Pi. A window and PNG
   snapshots are not planned.
3. **`scene-model`**: **done**. The `Scene` contract, the `Compositor`, and
   the event-driven loop (ADR-0012, ADR-0013). The daemon injects a `jiff`
   time in the system time zone. The `Heartbeat` scene is the base scene
   until the clock scene replaces it.
4. **`clock-scene`**: the clock face. The time-of-day color and brightness
   schedule is data, not hardcoded hours. The legacy rules are the defaults.
   Consider `--led-pwm-bits=8` (measured at bring-up).

## Messages and alerts

5. **`command-input`**: how commands get to the daemon. Candidates: a Unix
   socket, HTTP in the daemon, and the legacy semaphore file. Blocking input
   keeps idle CPU at zero.
6. **`message-board`**: a scrolling text scene with a queue, a pass count, and
   a style.
7. **`alerts`**: Panic and Bedtime as **configured instances** of one alert
   capability, not special-case code. An alert has a background effect, text,
   style, duration, priority, and interrupt rules. It uses shared message and
   effect parts.

## Later candidates

- **`configurable-schedule`**: brightness, color, and bedtime rules from
  config, by weekday. This replaces the abandoned legacy admin page.
- **`web-control`**: HTTP in the daemon replaces the Node 10 server. This
  saves RAM and removes Node from the Pi.
- Priorities, layers, and overlays, for example a clock with a message ticker.
  Transitions between scenes.
- Data-driven scenes: calendar events (the legacy events sync), weather, and
  countdowns.
- A metrics endpoint (CPU, refresh rate, uptime) to keep the performance
  budget honest.
- BDF font loading, so we can use the legacy fonts (`7x13B`, `texgyre-27`)
  again.
