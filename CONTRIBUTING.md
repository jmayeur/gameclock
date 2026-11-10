# Contributing

The short version of [`CLAUDE.md`](CLAUDE.md), which is the full working
agreement.

1. **Features start as an OpenSpec change** in `openspec/changes/<name>/`. Agree
   on the behavior and scenarios before writing code. Install the CLI once with
   `npm i -g @fission-ai/openspec@1.17.0` (not the unscoped `openspec`
   package). In Claude Code, run `/opsx:propose`, then `/opsx:apply`, then
   `/opsx:archive` as the last commit on the PR, after review and before
   merge.
2. **Decisions get an ADR.** That covers dependencies, boundaries, hardware or
   deployment assumptions, licensing, and reversals. See
   [`docs/adr/`](docs/adr/).
3. **Design docs stay true.** If you change how something works, update
   [`docs/design/`](docs/design/) in the same commit.
4. **Build test-first (red → green → refactor).** Write a failing test, see it
   fail for the right reason, make it pass, then clean up. Bug fixes start with
   a test that reproduces the bug. Commit tests together with the code that
   makes them pass.
5. **Tests and checks pass:**
   ```bash
   cargo fmt --all -- --check
   cargo clippy --all-targets -- -D warnings
   cargo test
   cargo deny check
   openspec validate --all --strict
   ```
6. **Performance claims come with numbers**, measured on the Pi Zero W for
   anything runtime-related.
7. **New dependencies must be GPL-2.0-compatible.** Apache-2.0-only and
   GPL-3.0/AGPL crates are rejected by `deny.toml`.

## Building for the Pi

The Pi build (`--features hzeller`) compiles hzeller's C++ driver, vendored as
a git submodule, with zig (ADR-0007, ADR-0009). Other `cargo` commands don't
need any of this.

```bash
git submodule update --init          # vendor/rpi-rgb-led-matrix, pinned
brew install zig                     # 0.17.1 (CI pins the same version)
cargo install --locked cargo-zigbuild@0.24.1
brew install binutils                # readelf, for check-pi-binary.sh (macOS)
```

```bash
scripts/build-pi.sh                  # -> target/arm-unknown-linux-gnueabihf/release/gameclockd
scripts/check-pi-binary.sh target/arm-unknown-linux-gnueabihf/release/gameclockd
scripts/clippy-pi.sh                 # lints the driver code host clippy can't see
```

## Deploying to the Pi

`GAMECLOCK_HOST` is the Pi's ssh destination (`user@host` or an `~/.ssh/config`
alias). It stays in your environment, never in the repo. The Pi user needs
passwordless `sudo`.

```bash
GAMECLOCK_HOST=pi@<host> scripts/deploy.sh             # build, install, restart (stops the legacy clock)
GAMECLOCK_HOST=pi@<host> scripts/deploy.sh --enable    # ...and make gameclockd the boot service
GAMECLOCK_HOST=pi@<host> scripts/deploy.sh --rollback  # back to the legacy clock, now and at boot
```

Driver flags go in `/etc/default/gameclockd` on the Pi, for example
`GAMECLOCKD_ARGS=--led-pwm-bits=7 --led-show-refresh`, then
`sudo systemctl restart gameclockd`.

## Measuring on the Pi

Copy `scripts/measure.sh` to the Pi and run it against a process:

```bash
sh measure.sh "$(pidof gameclockd)" 60
```

It prints CPU % of one core (total and per thread), RSS, and voluntary context
switches per thread. For refresh rate, add `--led-show-refresh` to the driver
flags and read the driver's output with `journalctl -u gameclockd -f`.

## Script tests

```bash
scripts/test-check-pi-binary.sh      # needs zig and readelf
scripts/test-deploy.sh               # stub ssh/scp, runs anywhere
scripts/test-unit.sh                 # Linux: needs systemd-analyze
scripts/test-measure.sh              # Linux: needs /proc and a C compiler
```

On macOS, run the Linux ones in a container, for example a Debian image with
`systemd gcc libc6-dev procps` installed:
`docker run --rm -v "$PWD":/w -w /w <image> scripts/test-measure.sh`.

Keep commits small, with an imperative subject line. Reference the ADR or
OpenSpec change where relevant. By contributing, you agree your contribution is
licensed GPL-2.0-or-later.
