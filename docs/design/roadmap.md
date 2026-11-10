# Roadmap

Proposed OpenSpec changes, in order. Each becomes
`openspec/changes/<name>/` when we start it. Order and scope are open to change.
This file is updated as changes land or plans shift.

## Foundation (core of driving the panel)

1. **`panel-bring-up`**: `rgbmatrix-sys` FFI plus a hzeller `Panel`, the pinned
   submodule, the `cargo-zigbuild` cross build for ARMv6 / glibc 2.28, a systemd
   unit, and a deploy script. Exit criteria: a static test pattern on the real
   panel, plus **baseline measurements** (CPU %, refresh Hz, RSS) for both the
   legacy Python clock and the new daemon. Riskiest step, so it goes first.
2. **`dev-simulator`**: a laptop `Panel` so scenes can be seen without the Pi.
3. **`scene-model`**: the scene contract (render, next-change deadline,
   finished), the compositor, and the event-driven loop (ADR-0006). Time is
   injected; the timezone is handled properly with `jiff`.
4. **`clock-scene`**: the clock face, with the time-of-day color and brightness
   schedule as data rather than hardcoded hours. Uses the legacy rules as
   defaults.

## Separation of concerns: messages and alerts

5. **`command-input`**: how commands reach the daemon. Candidates: a Unix
   socket, in-daemon HTTP, and legacy semaphore-file compatibility. Blocking
   input keeps idle CPU at zero.
6. **`message-board`**: a scrolling text scene with a queue, pass count, and
   style.
7. **`alerts`**: Panic and Bedtime as **configured instances** of one alert
   capability (background effect, text, style, duration, priority, interrupt
   rules) built on shared message and effect parts, instead of special-case
   code.

## Advanced capabilities (candidates)

- **`configurable-schedule`**: brightness, color, and bedtime rules from config,
  by weekday. This revives the abandoned legacy admin page.
- **`web-control`**: replaces the Node 10 server with HTTP in the daemon. Saves
  RAM and removes Node from the Pi.
- Priorities, layering, and overlays (for example, a clock with a message
  ticker); transitions between scenes.
- Data-driven scenes: calendar/events (revives the legacy events sync), weather,
  countdowns.
- Metrics endpoint (CPU, refresh rate, uptime) for keeping the performance
  budget honest.
- BDF font loading so the legacy fonts (`7x13B`, `texgyre-27`) can be reused.
