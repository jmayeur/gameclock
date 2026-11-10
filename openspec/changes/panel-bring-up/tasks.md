# Tasks

## 1. Decisions and tooling

- [x] 1.1 Write `docs/adr/0009-driver-crate-layout-and-flags.md` (status Accepted; refines ADR-0004: single `rgbmatrix-sys` crate for raw FFI and safe panel, `link`/`hzeller` features, hzeller `--led-*` pass-through over Pi defaults, `cc` build dependency, statically linked libc++/compiler-rt under Apache-2.0 WITH LLVM-exception, pinned-commit fallback to `a3eea99`); link it to this change and back. Verify the file follows `docs/adr/template.md`.
- [x] 1.2 Write `docs/adr/0010-systemd-service-and-signals.md` (root start with hzeller's privilege drop, `Conflicts=clock.svc.service`, boot enablement as an explicit step, `EnvironmentFile` for driver flags, deploy over SSH with `GAMECLOCK_HOST`, `signal-hook` for thread-free blocking signal waits); link it to this change and back. Verify against the template.
- [x] 1.3 Add ADR-0009 and ADR-0010 to the index in `docs/adr/README.md`, and add "Refined by ADR-0009" to ADR-0004's Related line. Verify the links resolve.
- [x] 1.4 Install `zig` and `cargo-zigbuild` on the dev machine (pinned versions), and document both plus `git submodule update --init` in `CONTRIBUTING.md`. Verify `cargo zigbuild --version` and `zig version` run.

## 2. Test pattern (gameclock-core)

- [x] 2.1 Write failing tests in `crates/gameclock-core/src/test_pattern.rs` for every `specs/test-pattern` scenario (corners, border, bar order, monotonic gray ramp, determinism, 0×0/1×1/2×5/5×2 frames). Verify `cargo test -p gameclock-core` fails on them.
- [x] 2.2 Implement `test_pattern::render` with `embedded-graphics` primitives and saturating layout math, export it from `lib.rs`. Verify `cargo test -p gameclock-core` passes.
- [x] 2.3 Add "Test pattern" to `docs/design/glossary.md` and the core's types in `docs/design/architecture.md`. Verify both docs mention `test_pattern`.

## 3. Driver crate, host-testable parts (rgbmatrix-sys)

- [x] 3.1 Create `crates/rgbmatrix-sys` (workspace member, `#![allow(unsafe_code)]` at crate root with a comment pointing to ADR-0009, `link` feature off by default, `cc` as an optional build dependency in `[workspace.dependencies]`). Verify `cargo build` and `cargo deny check` pass.
- [x] 3.2 Write failing tests for the `options` defaults scenario ("Built-in defaults describe this Pi") and the `convert` scenarios (row-major conversion, brightness 1/40/100 mapping, brightness 0 blanks, dimension mismatch reported with both sizes and no buffer write). Verify they fail.
- [x] 3.3 Implement `options::DEFAULTS`, `convert::prepare`, and the `Error` enum (`DimensionMismatch`, `Init`). Verify `cargo test -p rgbmatrix-sys` passes and `cargo clippy --all-targets -- -D warnings` is clean.

## 4. Driver crate, linked parts (rgbmatrix-sys `link`)

- [x] 4.1 Add hzeller `rpi-rgb-led-matrix` as a submodule at `vendor/rpi-rgb-led-matrix`, pinned to current upstream `master`; record the commit hash in ADR-0009 and `docs/hardware/pi-zero-w.md`. Verify `git submodule status` shows the pinned hash.
- [x] 4.2 Red: add `scripts/build-pi.sh` (the single `cargo zigbuild --release -p gameclockd --features hzeller --target arm-unknown-linux-gnueabihf.2.28` entry point) and `scripts/check-pi-binary.sh` (ELF ARMv6 + hard-float attributes, no `GLIBC_` version above 2.28, no `libstdc++`/`libc++` in `NEEDED`). Verify the checker fails against a non-Pi binary (for example a host build of `gameclockd`) and that `build-pi.sh` fails before the driver is wired up.
- [x] 4.3 Write `build.rs`: missing-submodule error with the `git submodule update --init` hint, two `cc::Build`s mirroring the pinned `lib/Makefile` flags and defines, `-mcpu=arm1176jzf_s` for the Pi target, link `stdc++`/`pthread`/`rt`/`m`, `rerun-if-changed`. Verify red first: with an empty `vendor/rpi-rgb-led-matrix`, `scripts/build-pi.sh` fails with the hint ("Missing driver source fails clearly").
- [x] 4.4 Hand-write `ffi` from the pinned `include/led-matrix-c.h` (only the functions we call, plus the options/runtime-options/`Color` structs) and `layout_check.c` with matching `const` size/offset assertions in Rust. Red first: write the C and Rust layout assertions before the struct mirrors and see the build fail, then add the mirrors until it passes. Verify the build also fails when one asserted size is deliberately changed, then restore it.
- [x] 4.5 Implement `HzellerPanel` (`new(args)` via the header's flag-parsing constructor over `DEFAULTS`, preallocated buffer, `present` = check → prepare → set brightness → `led_canvas_set_pixels` → swap on vsync, `Drop` = `led_matrix_delete`), each `unsafe` block with `// SAFETY:`. Verify every `unsafe` block has a `// SAFETY:` comment, and that `scripts/build-pi.sh && scripts/check-pi-binary.sh` now pass (green for 4.2).
- [x] 4.6 Run clippy on the linked code for the Pi target via `scripts/clippy-pi.sh` (`cargo-zigbuild clippy`, recorded in ADR-0009), with `clippy::undocumented_unsafe_blocks` denied in `rgbmatrix-sys`. Verify it is clean, and that removing one `// SAFETY:` comment makes it fail.
- [x] 4.7 Update `docs/design/architecture.md` (crate table: `rgbmatrix-sys` exists, features, data flow of `present`). Verify it matches the code.

## 5. Daemon (gameclockd)

- [x] 5.1 Add `signal-hook` to `[workspace.dependencies]` and `gameclockd`, plus the `hzeller` feature. Verify `cargo deny check` passes.
- [x] 5.2 Split `gameclockd` into `lib.rs` + `main.rs`. Write failing tests with recording/failing test panels for "One present at startup", "Frame follows the panel's size", "No presents while waiting" (500 ms delayed shutdown), and "Present failure exits non-zero" (error carries the panel message). Verify they fail.
- [x] 5.3 Implement `run` and `LogPanel`. Verify the 5.2 tests pass.
- [x] 5.4 Write tests for `parse_args` (only `--led-*` accepted; `--frobnicate` is a usage error naming the argument) and see them fail. Then implement it. Verify the tests pass.
- [x] 5.5 Write integration tests in `crates/gameclockd/tests/` that spawn the host binary for "SIGTERM exits with success", "SIGINT exits with success" (both within 1 s), "Unknown argument is a usage error", "Host build announces the logging panel", and "Driver flags are ignored without the driver". Verify they fail against the current stub `main`.
- [x] 5.6 Implement `main`: parse args (exit 2 on usage error), register SIGTERM/SIGINT before building the panel, choose the panel by feature, print the "no LED driver" and "ignoring `--led-*`" notices on host builds, block on `Signals::forever()`, map errors to a non-zero exit. Verify the 5.5 tests pass.
- [x] 5.7 Update `docs/design/architecture.md` (daemon section: startup, panel choice by feature, blocking idle, shutdown) and `docs/design/glossary.md` (LogPanel, driver flags). Verify the docs match the code.

## 6. Deploy and measurement scripts

- [x] 6.1 Red: add a `scripts/test-unit.sh` check that runs `systemd-analyze verify` and asserts the unit has `Conflicts=clock.svc.service`, `Restart=on-failure`, no `User=`, and the `EnvironmentFile`; see it fail with no unit. Then add `deploy/gameclockd.service` per design D8 until it passes (Linux machine or CI).
- [x] 6.2 Write a test (see it fail first) for "Deploy without a host refuses" (stub `ssh`/`scp` on `PATH` that fail if called), then write `scripts/deploy.sh` with default, `--enable`, and `--rollback` modes. Verify the test and `shellcheck scripts/*.sh` pass.
- [x] 6.3 Write `scripts/test-measure.sh` (see it fail first) for "Idle process measures near zero", "Busy process measures near one core", and "Threads are reported separately", then write `scripts/measure.sh` (POSIX `sh`, total and per-thread CPU %, `VmRSS`, voluntary context switches). Verify the test passes on Linux (CI or a container).
- [x] 6.4 Document building, deploying, rolling back, and measuring in `CONTRIBUTING.md`. Verify the documented commands match the scripts' flags.

## 7. CI

- [x] 7.1 Add a `pi-build` job: checkout with submodules, install pinned `zig` and `cargo-zigbuild`, run `scripts/build-pi.sh`, `scripts/check-pi-binary.sh`, and the Pi-target clippy from 4.6 ("CI produces the Pi binary"). Verify it passes on the PR.
- [x] 7.2 Add a `scripts` job: `shellcheck`, `scripts/test-unit.sh`, the deploy test, and `scripts/test-measure.sh`. Verify it passes on the PR.

## 8. On-device validation (Pi Zero W)

- [ ] 8.1 Measure the legacy baseline before deploying: run `scripts/measure.sh` on the legacy clock's Python process for 60 s idle (no messages), then stop the service and run its exact `ExecStart` with `--led-show-refresh` to read refresh Hz. Record CPU % (total and per thread), RSS, refresh Hz, flags, and window in `openspec/changes/panel-bring-up/measurements.md`.
- [ ] 8.2 Deploy with `scripts/deploy.sh` (no `--enable`). Verify "Service replaces the legacy clock" and "Test pattern on the real panel": correct corners, full border, bars, ramp, no tearing.
- [ ] 8.3 Verify the driver scenarios on the device: "Driver flags reach the driver", "A flag overrides a default", "Invalid flags fail at startup", "Unprivileged start fails cleanly". Record outcomes in `measurements.md`.
- [ ] 8.4 Verify lifecycle and service scenarios on the device: "Idle daemon does not wake" (via `measure.sh` context switches and CPU time), "Panel is dark after stop", "Crash is restarted", "Legacy clock is a one-command rollback", "Plain deploy leaves boot unchanged", "Enable swaps the boot service" (then `--rollback`). Record outcomes.
- [ ] 8.5 Measure `gameclockd` with the same driver flags as the legacy run: `measure.sh` for 60 s idle, then refresh Hz with `--led-show-refresh` via `/etc/default/gameclockd`. Record in `measurements.md` next to the legacy numbers. Verify the table is complete ("Baseline table is complete").
- [ ] 8.6 Answer the hardware open items (GPIO mapping, PWM mod, hardware pulse) and update `docs/hardware/pi-zero-w.md`; link the measurements from ADR-0006's baseline note if it helps readers. Leave the legacy clock as the boot service (`scripts/deploy.sh --rollback`).

## 9. Wrap-up

- [ ] 9.1 Update `docs/design/roadmap.md` (panel bring-up done, with a link to the measurements) and `CHANGELOG.md` (driver crate, test pattern, daemon lifecycle, Pi build, deploy and measurement scripts, ADR-0009/0010). Verify both read correctly.
- [ ] 9.2 Run `cargo fmt --all`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `cargo deny check`, `cargo check --target arm-unknown-linux-gnueabihf`, `scripts/build-pi.sh && scripts/check-pi-binary.sh`, and `openspec validate --all --strict`. Verify all pass.

## Workflow follow-up

- Open one PR for this change from `change/panel-bring-up`.
- After merge, run `/opsx:archive` once docs/design and the ADR index match what was built and the roadmap is updated.
