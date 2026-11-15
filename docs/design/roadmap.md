# Roadmap

The planned OpenSpec changes, in order (ADR-0008). Each item becomes
`openspec/changes/<name>/` through `/opsx:propose` when we start it. The order
and scope can change. Update this file when a change merges or a plan changes.

## Foundation: drive the panel

1. **`panel-bring-up`**: **done** (PR #4). `rgbmatrix-sys` (hzeller FFI and
   `HzellerPanel`), the zig cross build, the systemd unit, and the deploy and
   measurement scripts. The baseline numbers for both clocks are in
   `openspec/changes/archive/2026-11-09-panel-bring-up/measurements.md`. Open
   items: the 9% refresh gap to the legacy driver, which gcc built. The
   shimmer glitches were fixed in `clock-scene`.
2. **`dev-simulator`**: **done**. `TerminalPanel` draws frames in the terminal
   (`cargo sim`), so you can see scenes without the Pi. A window and PNG
   snapshots are not planned.
3. **`scene-model`**: **done**. The `Scene` contract, the `Compositor`, and
   the event-driven loop (ADR-0012, ADR-0013). The daemon injects a `jiff`
   time in the system time zone.
4. **`clock-scene`**: **done**. The clock face in the legacy format, and the
   time-of-day schedule as data with the legacy rules as the default
   (ADR-0014). Scenes give a brightness. The heartbeat stays as
   `--test-pattern`. The `adafruit-hat-pwm` mapping and 8 PWM bits remove
   the moving lines and raise the refresh rate to 176 Hz (ADR-0015).

## Messages and alerts

5. **`command-input`**: **done**. A root-only Unix socket with a line
   protocol, `gameclockd ctl`, and the commands `clock`, `test-pattern`, and
   `status`. The loop waits on an event channel with a signal thread and a
   control thread (ADR-0016). The legacy semaphore file is not supported. The
   Node form keeps the legacy clock as the boot service until `web-control`.
6. **`message-board`**: **done**. A scrolling text scene with a pass count
   and a color, a queue of 8 messages, and the `message` and `clear`
   commands. The position comes from the time. A message uses the brightness
   of the clock schedule (ADR-0017).
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
