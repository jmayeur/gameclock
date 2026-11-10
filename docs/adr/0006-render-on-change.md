# ADR-0006: Render on change, within a measured CPU budget

- **Status:** Accepted
- **Date:** 2026-11-09

## Context

Performance is the primary goal. On the single-core Pi Zero W, hzeller's refresh
thread needs as much of the CPU as it can get. The legacy loop woke every
30–150 ms whether or not anything had changed, even though the clock face only
changes once a minute.

## Decision

- **Event-driven loop.** `gameclockd` sleeps until the next scheduled visual
  change (the next minute boundary, the next animation frame, a schedule
  transition) or until input arrives. Idle means blocked, not polling.
- **Present only real changes.** Render into a `Frame`, compare it with the
  last presented frame, and skip `present` when they're identical.
- **Animations declare a frame rate.** Scrolling and similar effects run at an
  explicit, capped rate (initially ≤ 30 fps), and only while they're active.
- **No per-frame heap allocation** on the hot path. Fonts and resources load
  once at startup.
- **Measure, don't assume.** We track these numbers on the device:
  - daemon CPU % while idle and while animating;
  - panel refresh rate (hzeller reports it);
  - resident memory.

  The first baseline is the legacy Python clock on the same Pi, taken during
  panel bring-up (`openspec/changes/archive/2026-11-09-panel-bring-up/measurements.md`). Benchmarks for the core's render functions run on the dev
  machine.

## Consequences

- An idle clock costs about one wakeup a minute instead of 7–30 a second.
- Input latency depends on how commands arrive. Blocking reads (socket or HTTP)
  give immediate wakeups; polling a file would need a timer. That is one reason
  the command transport gets its own OpenSpec change.
- Scenes have to say when they next change. That's a small addition to the
  scene contract.

## Alternatives considered

- **Fixed-rate loop**, as in the legacy code. Simpler, but it pays continuously
  for nothing.
- **Fixed rate with frame diffing only.** Saves the blit but not the wakeups or
  the rendering work.
