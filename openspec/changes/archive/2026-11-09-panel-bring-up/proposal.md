# Proposal

## Why

Nothing in the new stack has touched the real panel yet. Driving hzeller's C++
driver from Rust, cross-compiled for an ARMv6 / glibc 2.28 Pi, is the riskiest
step in the roadmap, and every later change (scenes, clock, alerts) depends on
it working and fitting the CPU budget. We also have no numbers: before we claim
the Rust daemon is cheaper than the legacy Python clock, we need a baseline for
both, taken the same way on the same Pi (ADR-0006).

## What Changes

- Add the hzeller `rpi-rgb-led-matrix` library as a git submodule pinned to a
  specific upstream commit (ADR-0004).
- Add the `rgbmatrix-sys` crate: hand-written bindings to hzeller's C API, a
  `build.rs` that compiles the vendored library, and a safe `HzellerPanel` that
  implements `gameclock_core::Panel`. It is the only crate that allows
  `unsafe`. The C++ build is behind an opt-in `link` feature so host builds,
  clippy, tests, and the existing ARMv6 type-check never need a C++
  cross-compiler.
- Add a pure `test_pattern` renderer to `gameclock-core`: a border, distinct
  corner markers, and color bars that show orientation, channel order, and color
  depth on the real panel at a glance.
- Turn `gameclockd` from a stub into a minimal daemon: render the test pattern,
  present it once, then block (zero wakeups) until SIGTERM/SIGINT, then blank
  the panel and exit cleanly. The hzeller panel is selected by a `hzeller`
  cargo feature; without it the daemon uses a logging no-op panel.
- Pass hzeller's own `--led-*` command-line flags straight through to the
  driver, over daemon-supplied defaults for this Pi (64×32, `adafruit-hat`).
  This lets us tune and measure (`--led-pwm-bits`, `--led-slowdown-gpio`,
  `--led-show-refresh`, …) without a config format yet.
- Cross-build a release binary with `cargo-zigbuild` for
  `arm-unknown-linux-gnueabihf.2.28` (ADR-0007), and add that build to CI.
- Add a systemd unit (runs as root, `Conflicts=` the legacy `clock.svc.service`)
  and a deploy script that takes the target host from the environment.
- Add a measurement script (CPU %, RSS from `/proc`) and record **baseline
  measurements** for the legacy Python clock and the new daemon in this change.

## Capabilities

### New Capabilities

- `test-pattern`: the pure diagnostic image used to validate a panel's
  geometry, orientation, and color channels.
- `hzeller-panel`: displaying `Frame`s on the LED matrix through hzeller's
  driver: pixel transfer, brightness, dimension checks, driver options, and
  shutdown.
- `daemon-lifecycle`: how `gameclockd` starts, chooses a panel, presents, idles
  without polling, and shuts down on signals.
- `pi-deployment`: how the daemon is built for, installed on, and run on the Pi
  Zero W, and how its performance is measured there.

### Modified Capabilities

None. `openspec/specs/` is empty.

## Relies on

- ADR-0003 (GPL-2.0-or-later; new crates must pass `deny.toml`)
- ADR-0004 (hzeller via our own FFI over the C API, pinned submodule)
- ADR-0005 (pure core renders `Frame`s; panels only blit)
- ADR-0006 (render on change; measure CPU, refresh Hz, RSS on the device)
- ADR-0007 (ARMv6 / glibc 2.28 via `cargo-zigbuild`)

## New ADRs required

- **[ADR-0009](../../../../docs/adr/0009-driver-crate-layout-and-flags.md): Driver crate layout, feature gating, and driver flags.**
  Refines ADR-0004: one crate (`rgbmatrix-sys`) holds both the raw bindings and
  the safe `HzellerPanel`, so `unsafe` stays in a single crate; the C++ build
  and link are opt-in via features; hzeller's `--led-*` flags are passed
  through instead of building our own driver config for now. Adds the `cc`
  build dependency.
- **[ADR-0010](../../../../docs/adr/0010-systemd-service-and-signals.md): Run as a root systemd service with signal-driven shutdown.**
  Deployment assumptions (root start, hzeller's privilege drop, coexistence with
  the legacy service via `Conflicts=`, deploy over SSH with the host supplied
  out of the repo) and the `signal-hook` dependency for blocking, thread-free
  signal waiting.

## Expected runtime impact on the Pi

- **Daemon CPU while idle:** about 0%. After the one `present`, the main thread
  blocks in a signal wait; nothing polls. The refresh thread (hzeller's) is the
  only consumer, and its cost is governed by driver flags, not by us.
- **Refresh rate:** at least the legacy rate with the same driver flags, since
  the app competes for none of the core while idle.
- **RSS:** a few MB (static binary, one 6 KB frame), against tens of MB for
  Python plus Node in the legacy stack.

These are expectations, not results. They are measured on the Pi Zero W with
`scripts/measure.sh` (CPU % from `/proc/<pid>/stat` over a fixed window, RSS
from `/proc/<pid>/status`) and hzeller's `--led-show-refresh` for refresh Hz,
for both the legacy clock and `gameclockd`, with identical driver flags where
possible. Results go into `measurements.md` in this change.

## Out of scope

- The scene model, compositor, and event loop beyond "present once, wait for a
  signal" (`scene-model`).
- The clock face, schedules, fonts, messages, and alerts.
- A desktop simulator panel (`dev-simulator`); the no-op panel here only logs.
- A daemon config file or our own typed driver options; hzeller flags stand in.
- Command input of any kind.
- Reflashing the Pi to a newer OS (ADR-0007 open recommendation).
- Removing the legacy stack from the Pi; it stays installed for rollback.
- Tuning for best image quality; we record what flags we used, and tuning
  continues as later changes add real content.

## Impact

- **New:** `vendor/rpi-rgb-led-matrix` (submodule), `crates/rgbmatrix-sys`,
  `deploy/gameclockd.service`, `scripts/deploy.sh`, `scripts/measure.sh`.
- **Changed:** `gameclock-core` (new `test_pattern` module), `gameclockd`
  (real `main`, `hzeller` feature), workspace `Cargo.toml`, `deny.toml` if
  needed, CI (cross-build job), `.gitmodules`.
- **Dependencies:** `cc` (build-only, MIT OR Apache-2.0), `signal-hook`
  (MIT OR Apache-2.0). Dev tools: `zig`, `cargo-zigbuild`.
- **Docs:** ADR-0009, ADR-0010, ADR index, `architecture.md`, `glossary.md`,
  `roadmap.md`, `docs/hardware/pi-zero-w.md` open items, `CHANGELOG.md`,
  `CONTRIBUTING.md` (cross-build and deploy instructions).
