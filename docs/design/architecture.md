# Architecture

> The state after `scene-model`. Sections marked **Planned** give the agreed
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
  `Brightness`, the diagnostic `test_pattern`, the `Scene` trait, the
  `Compositor`, and the `Heartbeat` scene (ADR-0012). Time comes in as a
  `jiff::Zoned` (ADR-0013).
- **`gameclockd`** (exists): the daemon. It checks arguments, handles signals,
  reads the time zone, selects the panel by build feature, and runs the event
  loop below. Planned: config and command input.
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
- **`Scene`** (ADR-0012): `render(now, frame)` draws into a black frame.
  `next_change(now)` gives the next time the image can change, or `None`.
  `is_finished(now)` tells when the scene has nothing more to show.
- **`Compositor`** (ADR-0012): one base scene and a stack of foreground
  scenes. `update(now)` removes finished foreground scenes and renders the
  top scene into a preallocated frame. It gives the frame only if it differs
  from the last one. `deadline(now)` gives the top scene's next change, at
  least 33 ms after `now` (30 renders each second at most). Updates allocate
  no heap memory. A test in `gameclock-core/tests/no_alloc.rs` checks this.
- **`Heartbeat`**: the base scene for now. It is the test pattern with one
  white pixel on row 1. The pixel moves one column each second. The clock
  scene will replace it.
- **`Clock`, `Wait`** (`gameclockd`, ADR-0013): the injected time source and
  the deadline wait. `SystemClock` gives the system time in the time zone
  from startup. `SignalWait` reads the signal pipe with a timeout until the
  deadline. Tests use a fake clock and a scripted wait.

## Daemon lifecycle

`gameclockd` (ADR-0009, ADR-0010):

1. **Arguments.** `parse_args` accepts only hzeller `--led-*` flags. Any other
   argument exits 2 before the daemon touches the panel.
2. **Signals and time.** The daemon registers SIGTERM and SIGINT with
   `signal-hook` before the driver starts. Each signal writes a byte to a
   `UnixStream` pair (ADR-0013). Then the daemon reads the system time zone
   once and names it on stderr. If no time zone is available, it uses UTC
   and writes a warning.
3. **Panel, selected at build time.** There are three builds:
   - `hzeller` (the Pi build, `scripts/build-pi.sh`): `HzellerPanel::new`
     receives the flags.
   - `simulator` (`cargo sim`): `TerminalPanel` draws on stdout.
   - Neither: `LogPanel` writes each present to stderr.

   Without `hzeller`, the daemon ignores `--led-*` flags with a notice. A
   build with `hzeller` and `simulator` together stops with a compile error.
4. **Run.** `run` puts the heartbeat in a panel-sized `Compositor`. Then it
   loops:
   1. It gets the time from the clock and updates the compositor.
   2. If the update gives a frame, it presents the frame at full brightness.
   3. It waits until the compositor's deadline or a stop signal. With no
      deadline, the wait has no timeout. The wait is a blocked read with no
      extra thread.
5. **Shutdown.** On a signal, the panel drops and the process exits 0. hzeller
   stops its refresh thread and blanks the LEDs. A panel error exits 1 with
   the message on stderr.

On the Pi, the daemon runs as the `gameclockd` systemd service
(`deploy/gameclockd.service`, installed by `scripts/deploy.sh`). The service
conflicts with the legacy `clock.svc.service` (ADR-0010).

## Data flow

1. The loop wakes at a deadline (the next visual change) or on a stop signal.
   Planned: also on input.
2. It gives the injected time to the compositor. The compositor renders the
   top scene into a `Frame`.
3. If the frame is different from the last presented frame, the loop calls
   `panel.present(frame, brightness)`.
4. It gets the next deadline from the compositor and waits.

The wait uses a duration. When the system clock steps (for example at the
first NTP sync), the wake can be late by up to one interval (ADR-0013).

## Scene model

A **scene** is a self-contained renderer with its own state. It reports when
it next changes and when it finishes (ADR-0012). Now the only scene in the
daemon is the heartbeat.

The **compositor** decides what is visible: the base scene, or the newest
foreground scene. Scenes never refer to each other.

Planned: the legacy Clock, Message Board, Panic, and Bedtime become scenes.
Panic and Bedtime will likely be configured instances of one alert
capability, not special cases. Priorities, layers, and transitions are not
in the compositor yet (see `roadmap.md`).
