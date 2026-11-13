# gameclock

A clock and message board for a 64×32 RGB LED matrix. A Raspberry Pi Zero W
drives the matrix through an Adafruit RGB Matrix HAT. The code is Rust, tuned
for one ARMv6 core.

> **Status:** the daemon drives the real panel. It shows the clock, with the
> legacy colors and brightness for each time of day. `sudo gameclockd ctl
> status` talks to the running daemon. `sudo gameclockd ctl message hello`
> scrolls a message. Alerts come next, one at a time, through
> [OpenSpec](https://github.com/Fission-AI/OpenSpec). See the
> [roadmap](docs/design/roadmap.md).

## Why

This project replaces a Python and Node clock. On a single-core Pi Zero, CPU
that the app uses is CPU that the LED driver's refresh thread loses. The
result is flicker. So the design keeps the app out of the way:

- **Native code** with no interpreter and no garbage collector.
- **Render on change.** The daemon sleeps until the image changes, usually
  once a minute.
- **A proven driver.** Henner Zeller's
  [`rpi-rgb-led-matrix`](https://github.com/hzeller/rpi-rgb-led-matrix),
  pinned to a recent upstream commit, called through a small FFI layer.
- **Testable rendering.** Scenes draw into an in-memory frame with
  [`embedded-graphics`](https://github.com/embedded-graphics/embedded-graphics).
  Their tests run on a laptop with no Pi.

## Layout

```
crates/
  gameclock-core/   pure logic: Frame, Panel, scenes, compositor (no hardware, no I/O)
  gameclockd/       the daemon: arguments, signals, panel choice, lifecycle
  rgbmatrix-sys/    hzeller FFI and HzellerPanel (the only crate with unsafe)
vendor/
  rpi-rgb-led-matrix/  hzeller's driver, a pinned git submodule
deploy/             systemd unit
scripts/            Pi build, deploy, measurement, and doc lint, with tests
docs/
  adr/              architecture decision records (why)
  design/           how the system works now, roadmap, legacy reference (what)
  hardware/         Pi Zero W and HAT facts and tuning notes
openspec/           specs and changes
```

## Develop

You need [rustup](https://rustup.rs). `rust-toolchain.toml` pins the
toolchain, and rustup installs it automatically.

```bash
cargo test                                          # no hardware needed
cargo clippy --all-targets -- -D warnings
cargo deny check                                    # license and advisory policy
cargo check --target arm-unknown-linux-gnueabihf    # Pi Zero W (ARMv6)
scripts/lint-docs.sh                                # Simplified Technical English
cargo sim                                           # see the frames in the terminal
```

[`CONTRIBUTING.md`](CONTRIBUTING.md) tells how to build for the Pi, deploy, and
measure.

## Contributing

See [`CONTRIBUTING.md`](CONTRIBUTING.md). [`CLAUDE.md`](CLAUDE.md) has all the
rules: ADRs, current design docs, the OpenSpec flow, test-first work,
Simplified Technical English, and performance numbers.

## License

[GPL-2.0-or-later](LICENSE). The project links the GPL-2.0-only LED driver,
which sets the license
([ADR-0003](docs/adr/0003-license-gpl-2.0-or-later.md)). Each dependency must
be compatible with GPL-2.0. `cargo deny` enforces this.
