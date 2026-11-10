# Design

## Context

See `proposal.md` for why. Today the workspace has `gameclock-core` (`Frame`,
`Panel`, `Brightness`) and a `gameclockd` stub that prints a line. No C++ is
built anywhere, CI only type-checks the ARMv6 target, and `zig` and
`cargo-zigbuild` are not installed on the dev machine. The Pi runs the legacy
Python clock as `clock.svc.service`, linking hzeller at `a3eea99` (2023). Specs:
`specs/test-pattern`, `specs/hzeller-panel`, `specs/daemon-lifecycle`,
`specs/pi-deployment`.

## Goals / Non-Goals

**Goals:**

- Workspace-wide `cargo fmt/clippy/test/check` keep working on a Mac and on CI
  with no C++ toolchain and no submodule checked out.
- The FFI surface is small, hand-written, and checked against the C header's
  struct layout at build time.
- The daemon's idle state is a blocked syscall on the main thread.
- Measurements isolate the driver's refresh thread from the app's own work.

**Non-Goals:**

- Frame diffing and next-change deadlines (they arrive with `scene-model`; with
  one present per run there is nothing to diff).
- A typed config surface for driver options.
- Running driver code in host tests. Everything that touches the C library is
  verified on the device.

## Decisions

### D1. One driver crate, `rgbmatrix-sys`, holds raw FFI and the safe panel

`crates/rgbmatrix-sys` has `ffi` (private: `extern "C"` declarations and
`#[repr(C)]` structs mirrored from `include/led-matrix-c.h` at the pinned
commit), `options` (built-in Pi defaults), `convert` (pure frame-to-driver-buffer
conversion and brightness mapping), and `HzellerPanel: gameclock_core::Panel`.
It is the only crate with `#![allow(unsafe_code)]`; every `unsafe` block has a
`// SAFETY:` comment. Rationale and the alternative (separate `-sys` and safe
crates, which would put `unsafe` calls in two crates) are in **ADR-0009**.

### D2. The C++ build is behind features, not target detection

- `rgbmatrix-sys` feature `link` (off by default): `build.rs` compiles the
  vendored library with the `cc` crate and links it. Without `link`, `build.rs`
  does nothing, and the `ffi` module and `HzellerPanel` are
  `#[cfg(feature = "link")]`; `options` and `convert` always compile and are
  unit-tested on the host.
- `gameclockd` feature `hzeller = ["dep:rgbmatrix-sys", "rgbmatrix-sys/link"]`.

So `cargo clippy --all-targets`, `cargo test`, and the existing
`cargo check --target arm-unknown-linux-gnueabihf` never run a C++ compiler.
Keying off the target instead would make that ARMv6 type-check need a C++
cross-compiler on CI. The link-gated code is linted by the CI Pi job
(`cargo zigbuild clippy --features hzeller` for the Pi target). See ADR-0009.

### D3. Building hzeller with `cc` under zig

`build.rs` (with `link`):

- Fails early with "run `git submodule update --init`" if
  `vendor/rpi-rgb-led-matrix/lib/led-matrix-c.cc` is missing.
- Compiles `lib/*.cc` (C++) and `lib/*.c` (C) as two `cc::Build`s, mirroring
  the defines and flags in the pinned `lib/Makefile` (`-O3`,
  `-fno-strict-aliasing`, `-D_FILE_OFFSET_BITS=64`, plus its default
  `DEFINES`), and adds `-mcpu=arm1176jzf_s` when the target is
  `arm-unknown-linux-gnueabihf`.
- Compiles a small `layout_check.c` that `_Static_assert`s
  `sizeof`/`offsetof` of `RGBLedMatrixOptions`, `RGBLedRuntimeOptions`, and
  `Color` against constants that Rust also asserts with `const` checks on the
  mirrored structs. A header change that breaks our bindings then fails the
  build instead of corrupting memory.
- Links `stdc++` (zig provides libc++ statically), `pthread`, `rt`, `m`.
- Emits `rerun-if-changed` for `lib/`, `include/`, and `layout_check.c`.

`cargo-zigbuild` sets `CC`/`CXX` for the target, so `cc` uses zig with glibc
2.28 headers. `scripts/build-pi.sh` is the one entry point:
`cargo zigbuild --release -p gameclockd --features hzeller --target arm-unknown-linux-gnueabihf.2.28`.

### D4. Pixel transfer: one bulk copy, one vsync swap, no allocation

`HzellerPanel` owns the matrix, one offscreen canvas, and a preallocated
`Box<[ffi::Color]>` of `width × height`. `present`:

1. Checks dimensions (`Error::DimensionMismatch { frame, panel }`), before
   touching the driver.
2. `convert::prepare(frame, brightness, &mut buf) -> Option<u8>` fills the
   buffer and returns the driver brightness (`None` for 0: buffer is all black,
   driver brightness left unchanged).
3. `led_matrix_set_brightness` (hzeller scales pixels as they are set, so this
   comes first), then `led_canvas_set_pixels(canvas, 0, 0, w, h, buf)` (one
   call, not 2048), then `led_matrix_swap_on_vsync`, keeping the returned
   canvas as the next offscreen. If the pinned header lacks
   `led_canvas_set_pixels`, fall back to `led_canvas_set_pixel` per pixel and
   note it in ADR-0009.

`Drop` calls `led_matrix_delete`, which stops the refresh thread and clears
the panel.

### D5. Driver options: our defaults in the struct, hzeller parses the flags

`options::DEFAULTS` holds 64 cols, 32 rows, chain 1, parallel 1, mapping
`adafruit-hat`, as a `'static` Rust value (the mapping is a `&'static CStr`).
`HzellerPanel::new(args)` fills `RGBLedMatrixOptions` and `RGBLedRuntimeOptions`
from it, then calls the C API constructor that parses `--led-*` flags from
`argc/argv` into those structs (`led_matrix_create_from_options_and_rt_options`
after hzeller's flag parse, or `led_matrix_create_from_options` with mutable
argv, whichever the pinned header offers). Flags therefore override defaults
using hzeller's own parser, with its validation and `--led-*` names. A NULL
return becomes `Error::Init`. Rationale (no config format until we know which
knobs matter) in ADR-0009. `daemon` stays 0; `drop_privileges` stays hzeller's
default (on).

### D6. Daemon structure

`gameclockd` gets a `lib.rs` so behavior is testable without spawning:

```
pub fn parse_args(args) -> Result<DriverArgs, UsageError>   // only --led-*, else exit 2
pub fn run<P: Panel>(panel: &mut P, wait_for_shutdown: impl FnOnce()) -> Result<(), RunError>
```

`run` renders `gameclock_core::test_pattern` into a `Frame` of
`panel.dimensions()`, presents once at `Brightness::MAX`, then calls
`wait_for_shutdown`. `main.rs`:

1. `parse_args` (usage error → exit 2).
2. Registers SIGTERM and SIGINT with `signal_hook::iterator::Signals` **before**
   creating the panel, so a stop during driver init isn't lost.
3. Builds the panel: `HzellerPanel::new` with the `hzeller` feature, else a
   `LogPanel` that prints one line per present to stderr and the
   "no LED driver" notice (plus an "ignoring `--led-*`" notice if any were given).
4. `run(&mut panel, || { signals.forever().next(); })`. The iterator blocks in a
   read on signal-hook's self-pipe on the main thread: no timer, no extra
   thread. ADR-0010 covers the dependency.
5. Panel drops (blanking the matrix); `main` returns `ExitCode::SUCCESS`, or
   prints the error and returns failure.

Errors are plain enums implementing `std::error::Error`; no new error crate.

### D7. Test pattern lives in the core

`gameclock_core::test_pattern::render(&mut impl DrawTarget<Color = Rgb888>)`
uses only `embedded-graphics` primitives. Layout is computed from the target's
size with saturating arithmetic, so tiny frames clip instead of panicking. It is
not a scene; when `scene-model` lands it can be wrapped as one.

### D8. Service and deploy

- `deploy/gameclockd.service`: `ExecStart=/usr/local/bin/gameclockd`, no
  `User=` (root), `Restart=on-failure`, `RestartSec=2`,
  `Conflicts=clock.svc.service`, `WantedBy=multi-user.target`. Extra driver
  flags go in an optional `EnvironmentFile=-/etc/default/gameclockd`
  (`GAMECLOCKD_ARGS=`), so tuning doesn't need a rebuild. ADR-0010.
- `scripts/deploy.sh`: requires `GAMECLOCK_HOST` (an `ssh` destination; the
  repo never names a device), copies the binary and unit over `scp` to a temp
  dir, then over `ssh` runs `sudo install`, `systemctl daemon-reload`, and
  `systemctl restart gameclockd`. `--enable` additionally runs
  `systemctl disable clock.svc.service` and `systemctl enable gameclockd`.
  `--rollback` runs `systemctl disable --now gameclockd` and
  `systemctl enable --now clock.svc.service`.
- `deploy.sh` is checked by `shellcheck` in CI and has a host test for the
  "no host" path (it must exit before any `ssh`/`scp`; the test puts failing
  `ssh`/`scp` stubs first on `PATH`).

### D9. Measurement

`scripts/measure.sh <pid> [seconds]` (POSIX `sh`, runs on Buster): reads
`utime + stime` from `/proc/<pid>/stat` and from each
`/proc/<pid>/task/<tid>/stat` (with `comm`) at the start and end of the
window, divides by `getconf CLK_TCK` and the window, and prints total and
per-thread CPU % plus `VmRSS` and `voluntary_ctxt_switches` from `status`. A
CI test (`scripts/test-measure.sh`, Linux only) checks it against a sleeping
process, a spinning process, and a two-thread process.

Refresh rate comes from hzeller's `--led-show-refresh` for both processes. The
legacy clock is measured by stopping its service and running its exact
`ExecStart` command by hand with `--led-show-refresh` added, then repeating
without the flag for CPU (the flag's printing costs CPU). Results and flags go
in `openspec/changes/panel-bring-up/measurements.md`.

## Risks / Trade-offs

- [Hand-mirrored structs drift from the header] → the build-time layout check
  (D3), and binding updates are part of every submodule bump (ADR-0004).
- [Current upstream hzeller misbehaves on Buster or the Zero] → on-device
  validation is in the task list; fall back to pinning the known-good
  `a3eea99`, recorded in ADR-0009.
- [zig emits ARMv7 code for the C++ objects] → explicit `-mcpu=arm1176jzf_s`,
  and the CI binary check fails if the merged ELF attributes are not ARMv6
  (`specs/pi-deployment`). An illegal instruction on the device is the last
  line of defense.
- [Statically linked libc++/compiler-rt are Apache-2.0 WITH LLVM-exception] →
  the LLVM exception exists for GPLv2 combination; ADR-0009 records it.
  `cargo deny` doesn't see C/C++ code, so the ADR is the record.
- [`cargo zigbuild clippy` can't lint the `link` code in CI] → fall back to
  `cargo zigbuild check` plus clippy on the device-free code, and note it.
- [The refresh thread dominates process CPU, hiding app cost] → per-thread
  reporting (D9).
- [The daemon always presents at 100% brightness] → acceptable for a bring-up
  build that only shows a test pattern; brightness schedules come with
  `clock-scene`.
- [`--led-show-refresh` output fills the journal] → used only during
  measurement runs, not in the unit.

## Migration Plan

1. Deploy with `scripts/deploy.sh` (no `--enable`). Starting `gameclockd`
   stops the legacy clock via `Conflicts=`.
2. Validate the on-device scenarios and take measurements.
3. Roll back with `scripts/deploy.sh --rollback` (or
   `sudo systemctl start clock.svc.service`). The legacy clock stays the boot
   service until a later change replaces it with a real clock.

## Open Questions

These change flag values, not the design:

- `adafruit-hat` or `adafruit-hat-pwm` (is the GPIO4↔18 mod present)? Does
  hardware pulse work with audio off? Answered on the device, recorded in
  `docs/hardware/pi-zero-w.md`.
- Which `--led-pwm-bits` / `--led-slowdown-gpio` /
  `--led-limit-refresh-rate-hz` values to recommend: recorded with the
  measurements and set in `/etc/default/gameclockd`, not hardcoded.
- Which upstream commit to pin: latest `master` at apply time, unless D3's
  fallback triggers.
