# Architecture

> The state after panel bring-up. Sections marked **Planned** give the agreed
> direction (ADRs 0004–0006). They become current as OpenSpec changes merge.

## Shape

```
                ┌──────────────── gameclockd (binary, on the Pi) ────────────────┐
  wall clock ──▶│                                                                │
  (tz-aware)    │   event loop: sleep until next visual change or input          │
                │        │                                                       │
  commands  ───▶│        ▼                                                       │
  (transport    │   gameclock-core (pure) ──renders──▶ Frame ──changed?──▶ Panel ──┼──▶ LED matrix
   TBD)         │   scenes · schedule ·                (64×32 RGB)          .present│
                │   composition                                                  │
                └────────────────────────────────────────────────────────────────┘
```

## Crates

- **`gameclock-core`** (exists): pure logic. It has no hardware, I/O, threads,
  or wall clock (ADR-0005). It holds `Frame`, the `Panel` trait,
  `Brightness`, and the diagnostic `test_pattern`.
- **`gameclockd`** (exists): the daemon. Now it checks arguments, handles
  signals, selects the panel by build feature, and runs the lifecycle below.
  Planned: the event loop, the time source, config, and command input.
- **`rgbmatrix-sys`** (exists): the LED driver, and the only crate with
  `unsafe` (ADR-0004, ADR-0009).
  - Always: the Pi driver defaults (`options`), the conversion from a frame to
    the driver buffer (`convert`), and `Error`. Tests run on any host.
  - With the `link` feature: `build.rs` compiles hzeller from the
    `vendor/rpi-rgb-led-matrix` submodule with zig. The crate adds the
    hand-written FFI (`ffi`), a build-time struct layout check, and
    `HzellerPanel`.
- **`TerminalPanel`** (exists, `gameclockd::sim`, `simulator` feature): shows
  frames in a 24-bit color terminal on a dev machine. A window is not planned.

## Key types

- **`Frame`**: the buffer that each scene draws into. Drawing outside the
  frame is clipped. `pixels()` gives the row-major buffer that a panel blits.
  The daemon compares frames to decide whether to present (ADR-0006).
- **`Panel`**: `dimensions()` and `present(&Frame, Brightness)`. A panel only
  blits pixels.
- **`Brightness`**: a percentage from 0 to 100.
- **`HzellerPanel`** (`rgbmatrix-sys`, `link` only): created from `--led-*`
  flags over the Pi defaults. `present` does these steps:
  1. Checks the frame's size.
  2. Converts the frame into a preallocated buffer.
  3. Sets the driver brightness.
  4. Copies the buffer to an offscreen canvas in one call.
  5. Swaps the canvas in on vsync.

  When `HzellerPanel` drops, hzeller stops its refresh thread and blanks the
  panel.
- **`TerminalPanel`** (`gameclockd`, `simulator` only): writes each frame to
  a terminal. Each cell is `▀`, with the upper pixel as the foreground color
  and the lower pixel as the background color. A 64×32 frame uses 64 columns
  and 16 lines. It scales colors by the brightness. When it drops, it resets
  the colors and shows the cursor again.
- **`test_pattern::render`**: draws the diagnostic test pattern into any
  `Rgb888` `DrawTarget`, usually a `Frame`. The result depends only on the
  target's size.

## Daemon lifecycle

`gameclockd` (ADR-0009, ADR-0010):

1. **Arguments.** `parse_args` accepts only hzeller `--led-*` flags. Any other
   argument exits 2 before the daemon touches the panel.
2. **Signals.** The daemon registers SIGTERM and SIGINT with `signal-hook`
   before the driver starts.
3. **Panel, selected at build time.** There are three builds:
   - `hzeller` (the Pi build, `scripts/build-pi.sh`): `HzellerPanel::new`
     receives the flags.
   - `simulator` (`cargo sim`): `TerminalPanel` draws on stdout.
   - Neither: `LogPanel` writes each present to stderr.

   Without `hzeller`, the daemon ignores `--led-*` flags with a notice. A
   build with `hzeller` and `simulator` together stops with a compile error.
4. **Run.** `run` renders `test_pattern` into a panel-sized `Frame` and
   presents it once at full brightness. Then it blocks on the signal iterator.
   Idle is a blocked read with no timer and no extra thread.
5. **Shutdown.** On a signal, the panel drops and the process exits 0. hzeller
   stops its refresh thread and blanks the LEDs. A panel error exits 1 with
   the message on stderr.

On the Pi, the daemon runs as the `gameclockd` systemd service
(`deploy/gameclockd.service`, installed by `scripts/deploy.sh`). The service
conflicts with the legacy `clock.svc.service` (ADR-0010).

## Data flow (planned)

1. The loop wakes at a deadline (the next visual change) or on input.
2. It gives injected time and input to the core. The core updates the scene
   state and renders a `Frame`.
3. If the frame is different from the last presented frame, the loop calls
   `panel.present(frame, brightness)`.
4. It calculates the next deadline and sleeps.

## Scene model (planned, through OpenSpec)

The legacy Clock, Message Board, Panic, and Bedtime become **scenes**. A scene
is a self-contained renderer with its own state. It reports when it next
changes and when it finishes.

A **compositor** decides what is visible: the default scene, short-lived or
priority content such as alerts, and transitions. Scenes never refer to each
other. Panic and Bedtime will likely be configured instances of one alert
capability, not special cases (see `roadmap.md`).
