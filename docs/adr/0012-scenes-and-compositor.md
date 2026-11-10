# ADR-0012: Scenes and the compositor

- **Status:** Accepted
- **Date:** 2026-11-10
- **Related:** ADR-0005, ADR-0006, ADR-0013. OpenSpec change `scene-model`.

## Context

ADR-0005 puts scenes and composition in `gameclock-core`. ADR-0006 requires
the daemon to sleep until the next visual change. It also caps animations at
30 frames each second and forbids heap allocation for each frame. The clock,
the message board, and the alerts need one contract that gives the loop all
of this. Then a new scene does not change the loop.

## Decision

- **`Scene` trait** in `gameclock-core`, object-safe:
  - `render(&mut self, now, frame)` draws into a `Frame` that is black.
  - `next_change(&self, now)` gives the next time the image can change, or
    `None` for a static image.
  - `is_finished(&self, now)` tells when the scene has nothing more to show.
  A scene that finishes at a time reports that time as a change.
- **`Compositor`** in `gameclock-core`:
  - It holds one base scene and a stack of foreground scenes as
    `Box<dyn Scene>`. It renders only the top scene.
  - It removes finished foreground scenes. It never removes the base scene.
  - It owns two preallocated frames. It gives a frame to present only when
    the new frame is different from the last one.
  - Its deadline is the top scene's next change, and at least 33 ms after the
    update. This is the 30 fps cap of ADR-0006.
- **No-allocation test.** An integration test in `gameclock-core` counts heap
  allocations over 1000 updates. It uses the dev-dependency
  `allocation-counter` (MIT OR Apache-2.0, used under MIT). The crate installs
  its own counting allocator, so the test needs no `unsafe` code.

## Consequences

- A new scene is one type that implements `Scene`. The loop and the
  compositor do not change.
- Each scene gets a clean black frame, so scenes cannot leave pixels for each
  other.
- A scene with a bug that reports a past time cannot make the loop spin.
- A box is allocated only when a scene is added, not for each frame.
- Priorities, layers, and transitions are not in this model. A later change
  can add them in the compositor without a change to `Scene`.
- `allocation-counter` replaces the global allocator in `gameclock-core`'s
  test binaries only. The Pi binary does not contain it.

## Alternatives considered

- **An `enum` of all scenes.** Static dispatch, but each new scene changes the
  compositor. One call for each wake makes the dynamic dispatch cost too small
  to measure.
- **Each scene owns its frame.** Each scene then needs a buffer, and the
  compositor cannot clear and compare in one place.
- **A counting allocator of our own.** It needs `unsafe impl GlobalAlloc`,
  which CLAUDE.md permits only in the driver's FFI crate.
