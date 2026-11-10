# ADR-0003: License as GPL-2.0-or-later

- **Status:** Accepted
- **Date:** 2026-11-09
- **Related:** ADR-0004

## Context

The original intent was AGPL-3.0. While choosing a driver we found:

- hzeller's `rpi-rgb-led-matrix` is **GPL-2.0-only**. Its headers say
  "Free Software Foundation version 2" with no "or later" clause. GPL-2.0-only
  code can't be combined with GPL-3.0 or AGPL-3.0 code in a distributed binary.
- The pure-Rust port `rpi-led-panel` declares GPL-3.0, but it is a rewrite of
  that GPL-2.0-only code, so its provenance is unclear. It also describes itself
  as "mostly untested".
- The existing Rust bindings to hzeller are stale (`rpi-led-matrix`, 2022) or
  GPL-3.0 (`pi-led-matrix`), which is itself in conflict with GPL-2.0-only.

Performance is the primary goal (ADR-0002), and the fastest, most proven path
on a Pi Zero is to link hzeller's driver directly. The license should allow
that.

## Decision

License the project **GPL-2.0-or-later**.

- Linking hzeller is compliant. A distributed binary that includes it is
  effectively GPL-2.0.
- "Or later" keeps the option to move to GPL-3.0/AGPL-3.0 if the driver ever
  stops being GPL-2.0-only.
- Every dependency must be GPL-2.0-compatible. `deny.toml` enforces an allow
  list in CI:
  - **Apache-2.0-only** crates are rejected (incompatible with GPL-2.0).
    Dual-licensed `MIT OR Apache-2.0` crates are used under MIT.
  - **GPL-3.0, AGPL-3.0, and LGPL-3.0** crates are rejected.

## Consequences

- We can use the best-performing, most proven driver with no workarounds.
- We lose AGPL's network-use clause, which would have required anyone offering
  the web control as a service to share their changes. That's acceptable for a
  hobby device.
- The dependency pool is narrower. Apache-2.0-only crates need an alternative,
  or a new ADR.
- Source files don't need per-file headers. `LICENSE` plus the SPDX
  `license` field in `Cargo.toml` define the license.

## Alternatives considered

- **AGPL-3.0 with hzeller in a separate GPL-2.0 process**, talking over a pipe.
  The licensing is clean, but it adds an extra process and IPC on a single
  512 MB core, which works against the primary goal.
- **AGPL-3.0 with `rpi-led-panel`.** The license is compatible on paper, but
  the provenance is questionable and the crate admits it is mostly untested.
- **AGPL-3.0 with a clean-room Rust HUB75 driver.** The most work and the most
  risk, on the most timing-sensitive part of the system.
- **GPL-2.0-only.** Works today, but rules out a future move to (A)GPL-3.0.
