# ADR-0007: Target ARMv6 / glibc 2.28; cross-compile with cargo-zigbuild

- **Status:** Accepted
- **Date:** 2026-11-09
- **Related:** ADR-0004

## Context

The device is a Raspberry Pi Zero W Rev 1.1 (BCM2835, ARM1176JZF-S, ARMv6
hard-float, 512 MB) running **Raspbian 10 Buster**, which ships glibc 2.28.
Buster is end-of-life. Building Rust and C++ on the Zero itself takes a very
long time. Standard `armv7`/`aarch64` cross toolchains produce binaries the
ARMv6 core can't run.

## Decision

- Rust target: **`arm-unknown-linux-gnueabihf`** (ARMv6, hard-float), pinned in
  `rust-toolchain.toml`.
- Cross-compile from the dev machine with **`cargo-zigbuild`**, targeting glibc
  **2.28** (`arm-unknown-linux-gnueabihf.2.28`) so binaries run on Buster as-is.
  Zig also cross-compiles the C++ driver (ADR-0004) for ARMv6
  (`-mcpu=arm1176jzf_s`).
- Deploy a single binary plus a systemd unit. No runtime dependencies on the Pi.
- CI type-checks the ARMv6 target on every push. Full cross builds with the C++
  driver join CI as part of panel bring-up.

## Consequences

- Fast, reproducible builds from a laptop or CI. The Pi doesn't need a
  toolchain.
- Raising the glibc target later (after an OS upgrade) is a one-line change.
- Zig becomes a build dependency for release builds.
- **Open recommendation:** reflash to current Raspberry Pi OS Lite (32-bit,
  which still supports ARMv6) for security updates. This is the user's call and
  is tracked in `docs/hardware/pi-zero-w.md`. Nothing here depends on it.

## Alternatives considered

- **Native builds on the Zero.** Very slow, and the device needs a toolchain.
- **`cross` (Docker images).** Its ARMv6 images exist, but pinning glibc 2.28
  and building C++ for ARMv6 is less direct than with zig.
- **`armv7` target.** Doesn't run on the Zero's ARMv6 core.
