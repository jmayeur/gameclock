# Proposal

## Why

To see a frame today, we must deploy to the Pi. The host build's `LogPanel`
only prints "presented a 64×32 frame". Each new scene (clock, messages,
alerts) needs fast visual checks on the dev machine. A terminal panel gives
that with no new dependencies, and it works over SSH.

## What Changes

- Add a terminal `Panel` to `gameclockd`. It draws each frame in the terminal
  with 24-bit color and the upper half block character (`▀`). Each character
  cell shows two vertical pixels, so a 64×32 frame uses 64 columns and 16
  lines.
- The panel scales colors by the brightness, the same as on the LED matrix.
- The panel draws each frame at the same place in the terminal. It hides the
  cursor while it runs and restores the terminal when it drops.
- A new cargo feature `simulator` in `gameclockd` selects the terminal panel.
  The `hzeller` and `simulator` features cannot be used together.
- A cargo alias `cargo sim` runs the daemon with the simulator.
- Without a feature, the host build keeps `LogPanel`. The Pi build does not
  change.

## Capabilities

### New Capabilities

- `terminal-simulator`: how the terminal panel shows frames, brightness, and
  terminal state on a dev machine.

### Modified Capabilities

- `daemon-lifecycle`: the requirement "Panel is chosen at build time" adds the
  simulator build.

## Relies on

- ADR-0005: panels only blit. A simulator `Panel` is part of that plan.
- ADR-0009: features select the panel at build time. The C++ driver is only in
  the `hzeller` build.
- ADR-0011: messages and comments are in strict STE.

## New ADRs required

None. The change adds no dependency, crate, trait, process, or protocol. It
adds one more `Panel` implementation, as ADR-0005 planned. It uses the
feature-based panel choice from ADR-0009.

## Expected runtime impact on the Pi

None. The Pi build does not enable `simulator`, so the Pi binary does not
change. The pi-build CI job and `scripts/check-pi-binary.sh` show this.

## Out of scope

- A desktop window or PNG snapshots.
- Terminal resize handling, or a check that the terminal is large enough.
- Animation timing. The daemon still presents once (see `scene-model`).
- Keyboard input.
- Terminals without 24-bit color.

## Impact

- **New:** a `sim` module in `gameclockd` with `TerminalPanel`, and
  `.cargo/config.toml` with the `sim` alias.
- **Changed:** `crates/gameclockd/Cargo.toml` (feature), `main.rs` (panel
  choice), CI (lint and test the `simulator` build).
- **Docs:** `architecture.md`, `glossary.md`, `roadmap.md`, `README.md`,
  `CONTRIBUTING.md`, and `CHANGELOG.md`.
