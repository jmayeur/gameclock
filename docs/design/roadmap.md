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
7. **`alerts`**: **done**. Panic, bedtime, and the bedtime flash are data of
   one alert type, rendered by one alert scene. An alert has a background
   effect, a text and its style, a duration, a priority, and a clear rule. A
   board applies the priorities. An interrupted message restarts. Timed
   triggers start the bedtime warning (ADR-0018).

## Configuration

8. **`configurable-schedule`**: **done**. A TOML config file has the
   schedule, the alert definitions, and the trigger rules. Each section is
   optional, and a missing section uses its built-in table. `reload` reads
   the file again. A file with an error stops the start (ADR-0019,
   `docs/config.md`). This replaces the abandoned legacy admin page.

## Web control

9. **`web-control`**: **done**. `gameclockd` serves the web page on port 80
   with its own small HTTP server. The page sends messages and alerts, and
   it has clear and stop-alert buttons. The legacy buttons continue to work.
   The deploy stops nginx, PM2, and Node, and the Pi gets back about 106 MB
   of RSS (ADR-0020).

## Data on the panel

10. **`sensor-pages`**: **done**. The web page has Weather, Indoor, and
    Outdoor buttons. Each button shows a data page on the panel for 30 s.
    The data comes from the two enviro sensor Pis and the NWS forecast. The
    daemon downloads it with the system `curl` only on a request, and
    `serde_json` parses it (ADR-0021). The `[pages]` config section gives
    the URLs.

## Later candidates

- Remove nginx, PM2, and Node from the Pi after some weeks of use. The
  rollback needs them until then.
- `text-fonts`: fonts chosen by name, and a renderer for proportional
  fonts. Fun fonts for alerts and messages, for example from the
  `u8g2-fonts` crate. The legacy `7x13B` font is already in use. Only
  `texgyre-27` is missing.
- `clock-face`: a larger clock with digits about 20 px tall, which uses
  more of the panel.
- `icons`: one-color glyph icons, and small color sprites made from PNGs at
  build time, for weather and sensor pages.
- `panel-layout`: pages that rotate, or regions, so that the clock and the
  data can show together. Priorities, layers, and overlays, for example a
  clock with a message ticker. Transitions between scenes.
- Data-driven scenes: calendar events (the legacy events sync) and
  countdowns.
- A metrics endpoint (CPU, refresh rate, uptime) to keep the performance
  budget honest.
