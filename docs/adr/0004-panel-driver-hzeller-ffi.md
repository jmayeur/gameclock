# ADR-0004: Drive the panel with hzeller's library via our own FFI

- **Status:** Accepted
- **Date:** 2026-11-09
- **Related:** ADR-0003, ADR-0005, ADR-0007. Refined by ADR-0009. Implemented
  by the `panel-bring-up` OpenSpec change.

## Context

The legacy Pi runs hzeller's `rpi-rgb-led-matrix` at `a3eea99` (May 2023),
installed in place by Adafruit's old installer script. Upstream is still active
(latest commit Oct 2026). It supports the Pi Zero's BCM2835 and the
`adafruit-hat` GPIO mapping, and it has years of tuning for slow Pis. The
existing Rust bindings are unusable for us (ADR-0003).

## Decision

- Vendor hzeller's library as a **git submodule pinned to a specific upstream
  commit**. Updating it is a deliberate, reviewed bump.
- Add a `rgbmatrix-sys` crate. Its `build.rs` compiles the library, and it
  holds hand-written bindings to the **C API** (`include/led-matrix-c.h`, about
  30 functions). Using the C API avoids C++ ABI binding and keeps the unsafe
  surface small enough to audit.
- Add a safe wrapper that implements `gameclock_core::Panel`. It copies a
  `Frame` into an offscreen canvas and swaps on vsync. This crate is the only
  place `unsafe` is allowed, and every block carries a `// SAFETY:` comment.
- Expose the runtime options we need through `gameclockd` configuration:
  `gpio_mapping`, `slowdown_gpio`, `pwm_bits`, `pwm_lsb_nanoseconds`,
  `limit_refresh_rate_hz`, and `hardware_pulse`. Defaults come from
  `docs/hardware/pi-zero-w.md`.
- The driver builds only for the Pi target. Laptop builds and CI use test or
  simulator panels.

## Consequences

- The driver is current, not frozen at 2023, and we control exactly which
  commit ships.
- No stale third-party binding sits between us and the driver.
- We have to keep our bindings in sync with `led-matrix-c.h` when we bump the
  submodule. It is a small, stable header.
- We have to cross-compile C++ for ARMv6 (ADR-0007).
- Bring-up is validated on the device, with refresh rate and CPU measured
  (ADR-0006).

## Alternatives considered

- **`rpi-led-matrix` / `pi-led-matrix` crates.** Stale, or GPL-3.0
  (ADR-0003).
- **`rpi-led-panel` (pure Rust).** Questionable provenance and self-described
  as untested. It could be revisited as a benchmarked alternative if its
  licensing becomes clear.
- **`bindgen` against the C++ API.** The API surface is larger and the bindings
  more fragile, for no benefit over the C API.
- **Keep using the in-place 2023 install on the Pi.** Not reproducible, and it
  isn't the "modern, updated library" the project is after.
