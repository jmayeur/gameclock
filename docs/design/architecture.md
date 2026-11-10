# Architecture

> Current state as of the initial scaffold. Sections marked **Planned** describe
> the agreed direction (ADRs 0004–0006) and become current as OpenSpec changes
> land.

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

| Crate | Status | Responsibility |
|---|---|---|
| `gameclock-core` | **Exists** | Pure logic. `Frame` (a row-major RGB buffer that implements `embedded-graphics`' `DrawTarget`), the `Panel` trait, `Brightness`, and the diagnostic `test_pattern`. No hardware, I/O, threads, or wall clock (ADR-0005). |
| `gameclockd` | **Exists (stub)** | The daemon. Will own the event loop, time source, config, command input, and panel selection. |
| `rgbmatrix-sys` | **Exists** | The LED driver, and the only crate allowed `unsafe` (ADR-0004, ADR-0009). Always: Pi driver defaults (`options`), frame-to-driver-buffer conversion (`convert`), and `Error`, all host-tested. With the `link` feature: `build.rs` compiles the vendored hzeller submodule (`vendor/rpi-rgb-led-matrix`) with zig, hand-written FFI (`ffi`) with a build-time struct-layout check, and `HzellerPanel`, which implements `Panel`. |
| simulator `Panel` | **Planned** | Shows frames on a dev machine (terminal truecolor and/or a window). |

## Key types (current)

- **`Frame`**: what every scene draws into. Out-of-bounds drawing is clipped.
  `pixels()` exposes the row-major buffer that a panel blits. Frames are
  compared to decide whether to present (ADR-0006).
- **`Panel`**: `dimensions()` and `present(&Frame, Brightness)`. Implementations
  only blit pixels.
- **`Brightness`**: a percentage clamped to 0–100.
- **`HzellerPanel`** (`rgbmatrix-sys`, `link` only): created from `--led-*`
  flags over the Pi defaults. `present` checks the frame's size, converts it
  into a preallocated buffer, sets the driver brightness, copies the buffer to
  an offscreen canvas in one call, and swaps it in on vsync. Dropping it stops
  hzeller's refresh thread and blanks the panel.
- **`test_pattern::render`**: draws the diagnostic test pattern into any
  `Rgb888` `DrawTarget` (normally a `Frame`). Depends only on the target's size.

## Data flow (planned)

1. The loop wakes on a deadline (the next visual change) or on input.
2. It passes injected time and input to the core, which updates scene state and
   renders a `Frame`.
3. If the frame differs from the last one presented, it calls
   `panel.present(frame, brightness)`.
4. It computes the next deadline and sleeps.

## Scene model (planned, through OpenSpec)

The legacy Clock, Message Board, Panic, and Bedtime become **scenes**:
self-contained renderers with their own state, which report when they next
change and when they finish. A **compositor** decides what is visible: the
default scene, transient or priority content such as alerts, and transitions.
Scenes never reference each other. Panic and Bedtime are expected to be
configured instances of a general alert/message capability rather than special
cases (see `roadmap.md`).
