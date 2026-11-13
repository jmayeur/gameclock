# Architecture Decision Records

One file per significant decision: the context, what we chose, and what it
costs. See `CLAUDE.md` for when an ADR is required.

## Lifecycle

`Proposed` → `Accepted` → `Superseded by ADR-NNNN` or `Deprecated`.

ADRs are append-only. To change a decision, add a new ADR and update the old
one's status line. Small clarifications to an accepted ADR are fine.

## Template

Copy [`template.md`](template.md) to `NNNN-short-title.md` (next free number,
zero-padded) and add it to the index below.

## Index

| ADR | Decision | Status |
|-----|----------|--------|
| [0001](0001-record-decisions.md) | Record decisions as ADRs and keep design docs current | Accepted |
| [0002](0002-rust.md) | Write the daemon in Rust | Accepted |
| [0003](0003-license-gpl-2.0-or-later.md) | License as GPL-2.0-or-later | Accepted |
| [0004](0004-panel-driver-hzeller-ffi.md) | Drive the panel with hzeller's library via our own FFI | Accepted |
| [0005](0005-pure-core-frame-panel.md) | Pure core renders frames and panels only blit | Accepted |
| [0006](0006-render-on-change.md) | Render on change, within a measured CPU budget | Accepted |
| [0007](0007-target-and-cross-compilation.md) | Target ARMv6 / glibc 2.28 and cross-compile with cargo-zigbuild | Accepted |
| [0008](0008-openspec.md) | Drive features with OpenSpec | Accepted |
| [0009](0009-driver-crate-layout-and-flags.md) | Driver crate layout, feature-gated C++ build, pass-through driver flags | Accepted |
| [0010](0010-systemd-service-and-signals.md) | Run as a root systemd service with signal-driven shutdown | Accepted |
| [0011](0011-simplified-technical-english.md) | Write in Simplified Technical English | Accepted |
| [0012](0012-scenes-and-compositor.md) | Scenes and the compositor | Accepted |
| [0013](0013-time-and-deadline-wait.md) | Time with jiff, and the deadline wait | Accepted |
| [0014](0014-scene-brightness-and-schedule.md) | Scene brightness and the time-of-day schedule | Accepted |
| [0015](0015-driver-settings-for-the-clock.md) | Driver settings for the clock: the PWM mapping and 8 PWM bits | Accepted |
| [0016](0016-command-socket-and-event-channel.md) | Command input over a Unix socket, and an event channel for the loop | Accepted |
| [0017](0017-messages-scroll-from-time-and-daemon-queue.md) | Messages scroll from the injected time, and the daemon owns the queue | Accepted |
