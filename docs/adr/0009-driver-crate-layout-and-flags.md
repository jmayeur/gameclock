# ADR-0009: Driver crate layout, feature-gated C++ build, and pass-through driver flags

- **Status:** Accepted
- **Date:** 2026-11-09
- **Related:** Refines ADR-0004. Relies on ADR-0003, ADR-0005, ADR-0007.
  OpenSpec change `panel-bring-up`
  (`openspec/changes/archive/2026-11-09-panel-bring-up/`).

## Context

ADR-0004 chose our own FFI over hzeller's C API and named an `rgbmatrix-sys`
crate plus "a safe wrapper that implements `Panel`". It left three things open:

1. **Where `unsafe` lives.** Calling an `extern "C"` function is `unsafe` in the
   calling crate. If raw bindings and the safe wrapper are separate crates,
   both need `unsafe`, but `CLAUDE.md` confines it to one FFI crate.
2. **When the C++ gets built.** Every workspace command (`cargo clippy`,
   `cargo test`, and CI's `cargo check --target arm-unknown-linux-gnueabihf`)
   runs every member's build script. If the driver's `build.rs` compiled C++
   whenever it saw the ARM target, the plain ARMv6 type-check would need a C++
   cross-compiler, and a Mac checkout would need the submodule.
3. **How driver options are configured.** ADR-0004 says "through `gameclockd`
   configuration", but there is no config format yet, and bring-up needs to try
   many driver settings quickly to measure them.

Facts about the pinned upstream (`hzeller/rpi-rgb-led-matrix`
`3da7a3bdfca4637bb79ff038bc2bb26965197db5`, 2026-10-24):

- `led-matrix-c.h` has `led_canvas_set_pixels` (one bulk copy per frame) and
  `led_matrix_create_from_options(opts, &argc, &argv)`, which parses `--led-*`
  flags over the values in `opts` and removes the flags it consumed. Fields left
  at zero in `opts` keep hzeller's C++ defaults.
- `led_matrix_delete` stops the refresh thread and blanks the panel.
- The library now includes a Raspberry Pi 5 (RP1) backend that the core links
  against unconditionally. Its vendored `piolib` is GPL-2.0 (Raspberry Pi Ltd).
  On a Pi Zero it is compiled but inactive.

## Decision

- **One crate, `crates/rgbmatrix-sys`, holds the raw bindings and the safe
  `HzellerPanel`.** It is the only crate that allows `unsafe_code`, and every
  `unsafe` block carries a `// SAFETY:` comment. It depends on
  `gameclock-core` for `Frame`, `Panel`, and `Brightness`. The `-sys` name is
  kept from ADR-0004, even though the crate also has a safe layer.
- **The C++ build is opt-in through features, not target detection.**
  `rgbmatrix-sys` has a `link` feature, off by default. Only with `link` does
  `build.rs` compile the vendored library (with the `cc` crate) and do the FFI
  module and `HzellerPanel` exist. Pure parts (driver defaults, frame
  conversion, brightness mapping, errors) always compile and are unit-tested on
  any host. `gameclockd`'s `hzeller` feature turns `link` on. The Pi release
  build is the only build that enables it.
- **The build mirrors upstream's `lib/Makefile`:** its object list (including
  the RP1 sources), `-O3 -fno-exceptions -std=c++11`, and `DEFAULT_HARDWARE`,
  plus `-mcpu=arm1176jzf_s` for the Pi target. `-march=native` and LTO are left
  out because we cross-compile. A `layout_check.c` compiled alongside it
  statically asserts the sizes and offsets of the C structs we mirror by hand, so
  a header change that breaks our bindings fails the build.
- **Driver flags pass through.** `gameclockd` hands its `--led-*` arguments to
  `led_matrix_create_from_options`, over our defaults for this Pi (64×32,
  chain 1, parallel 1, `adafruit-hat`). hzeller's parser validates them. There
  is no driver section in a config file until a later change shows which knobs
  matter.
- **Licensing of the linked C/C++ runtime.** `zig c++` links LLVM's libc++,
  libc++abi, libunwind, and compiler-rt statically. They are
  Apache-2.0 WITH LLVM-exception. The LLVM exception exists so these runtimes
  can be combined with GPL-2.0 code, so a binary that also contains hzeller
  (GPL-2.0-only) is distributable. `cargo deny` does not see C/C++ code, so
  this ADR is the record.
- **New dependency:** `cc` (build-time only, MIT OR Apache-2.0, used under
  MIT).

## Consequences

- `cargo fmt/clippy/test/check` work everywhere with no C++ toolchain and no
  submodule checkout. Only the Pi build needs `zig`, `cargo-zigbuild`, and
  `git submodule update --init`.
- Host clippy doesn't see the `link`-gated code. `scripts/clippy-pi.sh`
  lints it for the Pi target with `cargo-zigbuild clippy` (the subcommand is
  only reachable as `cargo-zigbuild clippy`, not `cargo zigbuild clippy`), and
  `clippy::undocumented_unsafe_blocks` is denied in this crate.
- The layout check covers `struct Color` and `struct RGBLedMatrixOptions`, the
  structs we pass. `RGBLedRuntimeOptions` isn't bound: we create the matrix
  with `led_matrix_create_from_options`, which uses hzeller's runtime
  defaults plus any `--led-*` runtime flags.
- Bumping the submodule means re-reading `led-matrix-c.h` and
  `lib/Makefile`. The layout check catches struct drift, not new or renamed
  functions; those fail at link time.
- Driver tuning needs no rebuild (flags live in the service's environment
  file, ADR-0010), but flag names are hzeller's, so a future config file will
  have to map onto them or replace them.
- The RP1 backend's `piolib` finds its chips through a custom `piochips` ELF
  section and the `__start_`/`__stop_` symbols. LLD's `--gc-sections` drops
  that section (the link fails with `undefined symbol: __stop_piochips`), and
  zig rejects `-z nostart-stop-gc`. `ffi::retain_sections` references the
  section's entry (`__ptr_rp1_pio_chip`) so it stays. A submodule bump that
  renames it fails at link time.
- We compile RP1 code we never run on the Zero. It costs build time and some
  binary size, not runtime.
- **Fallback:** if the pinned commit misbehaves on the Zero or on Buster, pin
  the commit the legacy clock runs today (`a3eea99`, 2023-05-29) and record that
  here. At bring-up, our build of `a3eea99` was no better: 141 Hz against
  147 Hz, with the same glitches (`panel-bring-up` measurements). So the pin
  stays.

## Alternatives considered

- **Separate `rgbmatrix-sys` (raw) and `gameclock-hzeller` (safe) crates.**
  Idiomatic, but both would need `unsafe`, breaking the one-crate rule for no
  real gain at this size.
- **Build the C++ when the target is ARM Linux.** Breaks the existing ARMv6
  type-check in CI and forces the submodule on every checkout.
- **Our own typed driver options and a config file now.** Premature: we don't
  yet know which options matter, and the flags already exist with validation
  and documentation.
- **Build hzeller with its Makefile from `build.rs`.** The Makefile assumes a
  native build (`-march=native`, `gcc-ar`), so it would need patching. `cc`
  already handles cross-compilation through zig.
