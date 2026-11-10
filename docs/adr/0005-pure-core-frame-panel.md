# ADR-0005: Pure core renders frames; panels only blit

- **Status:** Accepted
- **Date:** 2026-11-09

## Context

In the legacy renderer, Clock, Message Board, Panic, and Bedtime are branches of
one loop. They share mutable locals, call the driver's draw functions directly,
read the clock and the command file inline, and pace themselves with `sleep()`.
None of it can be tested without a Pi, and adding a capability means adding
another branch.

We want separation of concerns, reuse, and room for more advanced capabilities.

## Decision

Two layers, with one data type between them:

1. **`gameclock-core`** is pure logic. It renders into a **`Frame`**: an
   in-memory, row-major RGB buffer that implements `embedded-graphics`'
   `DrawTarget`. It has no hardware, I/O, threads, or wall clock. Time and
   inputs are passed in. Scenes, scheduling, and composition live here (they
   arrive through OpenSpec changes).
2. **Panels** implement `Panel::present(&Frame, Brightness)`. They only blit
   pixels: the hzeller driver (ADR-0004), a desktop simulator, a test recorder.

`gameclockd` sits between the two. It owns the wall clock, input, config, and
the render loop, and it calls `present` only when the frame changed (ADR-0006).

We use `embedded-graphics` (MIT OR Apache-2.0) for drawing: primitives, mono
fonts, text layout, and images. It is mature, `no_std`-capable, allocation-free,
and makes the core testable at the pixel level.

## Consequences

- Every scene can be tested on a laptop by asserting on `Frame` pixels, or with
  golden-image snapshots.
- Swapping or adding a driver touches one crate. Scenes never know about
  hardware.
- Diffing frames is trivial, which makes render-on-change cheap (ADR-0006).
- One copy per frame from `Frame` into the driver canvas: 64×32×3 ≈ 6 KB, which
  is negligible next to the refresh thread.
- We're tied to `embedded-graphics` conventions. It is the de facto standard in
  embedded Rust, which we accept.

## Alternatives considered

- **Scenes call the driver directly**, as the legacy code did. This couples
  everything to hardware and makes the code untestable.
- **Our own drawing trait instead of `DrawTarget`.** We'd reinvent fonts,
  primitives, and clipping for nothing.
- **Render straight into the driver's offscreen canvas.** This avoids one 6 KB
  copy but makes the core depend on FFI and loses frame diffing.
