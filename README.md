# gameclock

A clock and message board for a 64×32 RGB LED matrix, driven by a Raspberry Pi
Zero W through an Adafruit RGB Matrix HAT. Written in Rust, tuned for a single
ARMv6 core.

> **Status: early scaffold.** The architecture, tooling, and decision records
> are in place. Features are being built one at a time through
> [OpenSpec](https://github.com/Fission-AI/OpenSpec). See the
> [roadmap](docs/design/roadmap.md).

## Why

This replaces a Python + Node clock. On a single-core Pi Zero, every cycle the
app uses is taken from the LED driver's refresh thread, and that shows up as
flicker. The rewrite aims to stay out of the way:

- **Native code**, with no interpreter or GC.
- **Render on change.** The daemon sleeps until something visible changes
  (usually once a minute).
- **A proven driver.** Henner Zeller's
  [`rpi-rgb-led-matrix`](https://github.com/hzeller/rpi-rgb-led-matrix), pinned
  to current upstream, called through a small FFI layer.
- **Testable display logic.** Scenes render into an in-memory frame with
  [`embedded-graphics`](https://github.com/embedded-graphics/embedded-graphics),
  so they're unit-tested on a laptop with no Pi.

## Layout

```
crates/
  gameclock-core/   pure logic: Frame, Panel trait, scenes (no hardware, no I/O)
  gameclockd/       the daemon: time, input, event loop, panel driver
docs/
  adr/              architecture decision records (why)
  design/           how it works now, roadmap, legacy reference (what)
  hardware/         Pi Zero W + HAT facts and tuning notes
```

## Develop

Requires [rustup](https://rustup.rs). The toolchain is pinned in
`rust-toolchain.toml` and installs automatically.

```bash
cargo test                                   # runs anywhere, no hardware needed
cargo clippy --all-targets -- -D warnings
cargo deny check                             # license and advisory policy
cargo check --target arm-unknown-linux-gnueabihf   # Pi Zero W (ARMv6)
```

Cross-compiling and deploying to the Pi arrive with the `panel-bring-up` change
([ADR-0007](docs/adr/0007-target-and-cross-compilation.md)).

## Contributing

See [`CONTRIBUTING.md`](CONTRIBUTING.md). [`CLAUDE.md`](CLAUDE.md) is the full
working agreement: ADRs, living design docs, the OpenSpec flow, and performance
discipline.

## License

[GPL-2.0-or-later](LICENSE). The license is set by the GPL-2.0-only LED driver
this project links; see
[ADR-0003](docs/adr/0003-license-gpl-2.0-or-later.md). Dependencies must be
GPL-2.0-compatible, and `cargo deny` enforces this.
