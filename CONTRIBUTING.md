# Contributing

The short form of [`CLAUDE.md`](CLAUDE.md), which has all the rules.

1. **Features start as an OpenSpec change** in `openspec/changes/<name>/`.
   Agree on the behavior and scenarios before you write code. Install the CLI
   with `npm i -g @fission-ai/openspec@1.17.0` (not the unscoped `openspec`
   package). In Claude Code, run `/opsx:propose`, then `/opsx:apply`. Run
   `/opsx:archive` as the last commit on the PR, after review and before
   merge.
2. **Decisions get an ADR:** dependencies, boundaries, hardware or deployment
   assumptions, licensing, and reversals. See [`docs/adr/`](docs/adr/).
3. **Design docs stay true.** If you change how something works, update
   [`docs/design/`](docs/design/) in the same commit.
4. **Build test-first (red → green → refactor).** Write a test and see it fail
   for the right reason. Make it pass. Then improve the code. A bug fix starts
   with a test that shows the bug. Commit each test with the code that makes
   it pass.
5. **Write in Simplified Technical English** (ADR-0011): short sentences,
   active voice, no semicolons, one name for one thing. The `asd-ste100` skill
   in `.claude/skills/` has the rules.
6. **All checks pass:**
   ```bash
   cargo fmt --all -- --check
   cargo clippy --all-targets -- -D warnings
   cargo test
   cargo deny check
   openspec validate --all --strict
   scripts/lint-docs.sh
   ```
7. **Performance claims come with numbers** from the Pi Zero W for all
   runtime behavior.
8. **New dependencies must be compatible with GPL-2.0.** `deny.toml` rejects
   Apache-2.0-only, GPL-3.0, and AGPL crates.

## Seeing frames without the Pi

```bash
cargo sim
```

This runs the daemon with the terminal simulator (`--features simulator`).
You need a terminal with 24-bit color, at least 64 columns, and 17 lines.
Stop it with Ctrl-C. If you kill it with SIGKILL, the cursor stays hidden. Run
`reset` to restore the terminal.

## Building for the Pi

The Pi build (`--features hzeller`) compiles hzeller's C++ driver from a git
submodule with zig (ADR-0007, ADR-0009). Other `cargo` commands do not need
these tools.

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

`GAMECLOCK_HOST` is the Pi's ssh destination (`user@host` or an
`~/.ssh/config` alias). Keep it in your environment, not in the repo. The Pi
user needs passwordless `sudo`.

```bash
GAMECLOCK_HOST=pi@<host> scripts/deploy.sh             # build, install, restart (stops the legacy clock)
GAMECLOCK_HOST=pi@<host> scripts/deploy.sh --enable    # ...and make gameclockd the boot service
GAMECLOCK_HOST=pi@<host> scripts/deploy.sh --rollback  # back to the legacy clock, now and at boot
```

Put driver flags in `/etc/default/gameclockd` on the Pi, for example
`GAMECLOCKD_ARGS=--led-pwm-bits=8`. Then run
`sudo systemctl restart gameclockd`.

## Measuring on the Pi

Copy `scripts/measure.sh` to the Pi and run it on a process:

```bash
sh measure.sh "$(pidof gameclockd)" 60
```

It prints CPU as a percentage of one core (total and for each thread), RSS,
and voluntary context switches for each thread. For the refresh rate, add
`--led-show-refresh` to the driver flags. Read the output with
`journalctl -u gameclockd -f`.

## Script tests

```bash
scripts/test-check-pi-binary.sh      # needs zig and readelf
scripts/test-deploy.sh               # stub ssh/scp, runs anywhere
scripts/test-unit.sh                 # Linux: needs systemd-analyze
scripts/test-measure.sh              # Linux: needs /proc and a C compiler
scripts/test-lint-docs.sh            # needs python3
```

On macOS, run the Linux tests in a container. For example, use a Debian image
with `systemd gcc libc6-dev procps`:
`docker run --rm -v "$PWD":/w -w /w <image> scripts/test-measure.sh`.

Make small commits with an imperative subject line. Name the ADR or the
OpenSpec change. Your contribution is licensed GPL-2.0-or-later.
