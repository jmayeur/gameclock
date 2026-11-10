# Tasks

## 1. Decisions

- [x] 1.1 Make sure that no ADR is necessary: the change adds no dependency, crate, trait, process, or protocol (proposal, "New ADRs required"). If the work shows a new decision, stop and write the ADR first. Verify by a review of the final diff for new dependencies (`git diff main -- '*Cargo.toml' Cargo.lock` shows no new crates).

## 2. Terminal panel (gameclockd `sim` module)

- [x] 2.1 Add the `simulator = []` feature to `crates/gameclockd/Cargo.toml` and an empty `sim` module behind it, with a `TerminalPanel<W: Write>` stub whose `present` writes nothing. Verify that `cargo check -p gameclockd --features simulator` passes.
- [x] 2.2 Red: write unit tests in `sim.rs` for each `terminal-simulator` scenario, except the manual one. The tests read the output from a `Vec<u8>` or from a writer that always fails. Verify that `cargo test -p gameclockd --features simulator` fails on them for the expected reasons. The scenarios:
  - "Cell colors come from a pixel pair"
  - "A 64×32 frame uses 16 rows of 64 cells"
  - "An odd last row has a black lower half"
  - "Half brightness halves each channel"
  - "Zero brightness is black"
  - "Setup happens once"
  - "Drop restores the terminal"
  - "A closed output fails the present"
- [x] 2.3 Green: implement `TerminalPanel` (design D2–D5, D7). Verify that the 2.2 tests pass and that `cargo clippy -p gameclockd --features simulator --all-targets -- -D warnings` is clean.

## 3. Panel choice in the daemon

- [x] 3.1 Red: add the CLI test "Simulator build draws in the terminal" to `tests/cli.rs`. It runs only with `--features simulator`. Mark the existing CLI tests to run only without features. Verify that the new test fails, because the simulator build still uses `LogPanel`.
- [x] 3.2 Red: add `scripts/test-feature-exclusive.sh` for "Driver and simulator together do not build". It runs `cargo-zigbuild check` for the Pi target with `--features hzeller,simulator`, expects a failure, and expects the message (design D8: on the host, the C++ build fails first). Verify that it fails now, because the build passes.
- [x] 3.3 Green: in `main.rs`, add the `simulator` panel module with the stderr notice and the `--led-*` notice (design D6), and add the `compile_error!` for both features. Verify that 3.1 and 3.2 pass, and that `cargo test` without features still passes.
- [x] 3.4 Add `.cargo/config.toml` with the `sim` alias. Verify that `cargo sim` starts the simulator.
- [x] 3.5 Manual: run `cargo sim` in a 24-bit color terminal of at least 64 columns and 17 lines. Verify "Test pattern in a terminal": the corners, border, bars, and ramp are correct, and Ctrl-C returns the terminal to normal. Record the result in the PR.

## 4. CI

- [x] 4.1 In the `check` job, add `cargo clippy --all-targets --features simulator -- -D warnings` and `cargo test --features simulator`. Add `scripts/test-feature-exclusive.sh` to the `pi-build` job, which has zig. Verify that CI passes on the PR, and that the Pi build job and `scripts/check-pi-binary.sh` still pass (the Pi binary does not change).

## 5. Docs

- [x] 5.1 Update `docs/design/architecture.md` (the simulator `Panel` exists, and the panel choice has three builds) and `docs/design/glossary.md` (TerminalPanel, simulator). Verify that both agree with the code and pass `scripts/lint-docs.sh`.
- [x] 5.2 Update `README.md` and `CONTRIBUTING.md` with `cargo sim`, the terminal size, and `reset` after a SIGKILL. Verify that the commands work as written.
- [x] 5.3 Update `docs/design/roadmap.md` (`dev-simulator` done) and `CHANGELOG.md`. Verify with `scripts/lint-docs.sh`.

## 6. Wrap-up

- [x] 6.1 Run `cargo fmt --all`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, the same two with `--features simulator`, `cargo deny check`, `cargo check --target arm-unknown-linux-gnueabihf`, `scripts/build-pi.sh`, `scripts/check-pi-binary.sh`, `scripts/lint-docs.sh`, and `openspec validate --all --strict`. Verify that all pass.

## Workflow follow-up

- Open one PR from `change/dev-simulator`.
- After review and before merge, run `/opsx:archive` on the PR.
