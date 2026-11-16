# Tasks

## 1. ADR and dependencies

- [x] 1.1 Write ADR-0019 (the config is a TOML file that serde reads in the daemon). Link ADR-0014 and ADR-0018, and add ADR-0019 to their "Related" lines as a small note. Add it to the index in `docs/adr/README.md`. Verify with `scripts/lint-docs.sh`.
- [x] 1.2 Add `serde` (with `derive`) and `toml` (no default features: `std`, `serde`, `parse`) to the workspace and to `gameclockd`. Run `cargo deny check`. Add a license to `deny.toml` only if a new crate needs one, with a comment. Verify that `cargo build` and `cargo deny check` pass.

## 2. Owned alerts and limits in the core (`alerts`)

- [x] 2.1 Red: write `Alert::validate` and `Alerts::new` tests. Test a good alert, a name that is empty, too long, `stop`, or has an upper-case letter, and two alerts with one name. Also test a text that is empty or has 65 characters, `passes = 0` and 11, and `seconds` 0 and 3601. The core type `Motion` cannot hold `x` and `passes` together, so the config parse tests that case (3.3). Test `Alerts::find` and `names()` (`panic, bedtime, and bed-flash`, `a and b`, and `dinner`). See them fail.
- [x] 2.2 Green: make `Alert` owned (`String` name and text), add `AlertError`, `Alert::validate`, and `Alerts` with `new`, `legacy`, `find`, and `names` (design D3). Remove `BUILT_IN` and `find`. Change `AlertScene`, `Board`, and `Trigger` to `Rc<Alert>`. Verify that the 2.1 tests and all earlier core tests pass, with only the type changes in them.
- [x] 2.3 Verify that `tests/no_alloc.rs` passes for each alert of `Alerts::legacy()`. To see it fail, clone the alert text in `render` for a short time. Then remove the clone.
- [x] 2.4 Red: write `Trigger::new` tests: `from` at or after `to`, and `every` 0 s and 3601 s are errors. See them fail.
- [x] 2.5 Green: add `Trigger::new` with its limits, and use it in `Trigger::legacy()`. Verify that the 2.4 tests pass.
- [x] 2.6 Red: write a test that `Tables::legacy()` has the legacy schedule, the three built-in alerts, and the two bedtime triggers. Add a `PartialEq` test for two `Tables::legacy()` values. See it fail.
- [x] 2.7 Green: add `gameclock_core::tables::Tables` with `legacy()`, and `PartialEq` on `Schedule`, `Alerts`, and `Trigger`. Verify that the 2.6 test passes.

## 3. The config parse (`daemon-config`, `clock-scene`, `alerts`)

- [x] 3.1 Red: write `config::parse` tests for "Empty file", "Only a schedule section", "Rule on bedtime nights", and "Rule with no days". Also write "A new alert from the file" and "Alerts in the file replace the built-in alerts" (the table has only `dinner`). Also write "Trigger from the file" and "Trigger for an unknown alert". See them fail because the `config` module does not exist.
- [x] 3.2 Green: add the `config` module with the serde types, `parse`, and `ConfigError` (D1, D2). Verify that the 3.1 tests pass.
- [x] 3.3 Red: write tests for "Brightness out of range" (the exact error text with line 3, column 14) and "Unknown key". Also write "TOML syntax error" and each value of "Range values". Test the line and the column of each error. The spans of 3.2 made them pass at once, so a wrong column in `position` for a short time made 7 of them fail.
- [x] 3.4 Green: add the spans and the line and column from the byte offset. The 3.1 test "Trigger for an unknown alert" needs them, so they came in 3.2. Verify that the 3.3 tests pass.
- [x] 3.5 Red: write "The example file gives the legacy tables": parse `deploy/config.toml` and compare it with `Tables::legacy()`. See it fail because the file does not exist.
- [x] 3.6 Green: write `deploy/config.toml` with the legacy tables and STE comments. Verify that the 3.5 test passes.
- [x] 3.7 Red: write `config::load` tests with files in a temporary directory. Write "No file at the default path" (the built-in tables and the note text). Write "Missing file at a given path" (an error that names the path), and a file that is not UTF-8. See them fail.
- [x] 3.8 Green: add `config::load` with the origin (D1). Verify that the 3.7 tests pass.

## 4. Arguments and start (`daemon-config`, `daemon-lifecycle`)

- [x] 4.1 Red: write `parse_args` tests for "Default config path", "Config path from the argument", and "Empty config path is a usage error". See them fail.
- [x] 4.2 Green: add `--config=<path>` and the default path to `Args`. Verify that the 4.1 tests pass.
- [x] 4.3 Red: write `tests/cli.rs` tests. Write "Bad file at start": exit status 2, the error on standard error, and no `LogPanel` present line. Also write "Missing file at a given path", and a start with a correct file. See them fail.
- [x] 4.4 Green: load the config in `main` before the signal thread, the socket, and the panel (D6). Write the note for a missing default file. Verify that the 4.3 tests and all CLI tests pass in the default and the `simulator` builds.

## 5. Commands and the loop (`daemon-config`, `alerts`, `command-input`)

- [x] 5.1 Red: change the `parse_command` tests: `alert` gives `Command::Alert(None)`, and `alert fire` gives `Command::Alert(Some("fire"))`. Add `reload`. Change the unknown-command reply to seven commands, also in `control.rs`. See them fail.
- [x] 5.2 Red: write `run` tests with a scripted reload function. Write "Reload with a new schedule" (the present comes before `ok`) and "Reload with an error". Also write "Reload during an alert", "Reload restarts the triggers", and "Reload in the test-pattern mode". Move the "No name" and "Unknown name" tests to `run`, and add one with a table that has only `dinner`. Change all `run` calls to the new arguments. See them fail.
- [x] 5.3 Green: add `Command::Reload` and `Command::Alert(Option<String>)`, the lookup in the loop, `Board::set_schedule`, `Mode::base_scene(&Schedule)`, and the new `run` arguments (D4, D5). Verify that the 5.1 and 5.2 tests and all earlier tests pass.
- [x] 5.4 Red: write the `tests/cli.rs` test "Reload through the client". Start with a correct file, and change it to a file with an error. Then `ctl reload` must exit 1 with the error. Then restore the file and see `ok`. The code of 5.3 made it pass at once, so a wrong command word in the parser for a short time made it fail. It passes in the default and the `simulator` builds.
- [x] 5.5 Update `docs/design/architecture.md`: the `config` module, `Tables`, the owned alerts, the start order, and `reload`. Verify with `scripts/lint-docs.sh`.

## 6. Deployment (`pi-deployment`)

- [x] 6.1 Red: add to `scripts/test-deploy.sh`: the script copies `deploy/config.toml`, and the remote command has the guard `[ -e /etc/gameclockd/config.toml ] ||` before the install. Add a test that the unit has `RestartPreventExitStatus=2`. See them fail.
- [x] 6.2 Green: change `scripts/deploy.sh` and `deploy/gameclockd.service` (D7). Verify with `scripts/test-deploy.sh` and the unit test in CI.

## 7. Config docs

- [x] 7.1 Write `docs/config.md`, and link it from the README. It gives the path, the sections, each key and its limit, and the built-in defaults. It also gives the steps to edit the file and to send `reload`. Verify with `scripts/lint-docs.sh`.

## 8. Simulator verification

- [x] 8.1 Run the `simulator` build with a temporary config that has a green default style and the alert `dinner`. Send `alert dinner`. Then change the file to a blue style and send `reload`. Verify the colors in the captured output, and the error reply for a file with an error. Result: green clock `(0,255,0)`, yellow `DINNER` `(255,255,0)`, blue clock `(0,0,255)` after `reload`, `alert panic` refused with "The alerts are dinner.", and the error reply at line 3, column 14.

## 9. On-device verification and measurements

- [x] 9.1 Build with `scripts/build-pi.sh` and run `scripts/check-pi-binary.sh`. Record the binary size against `main` in `openspec/changes/configurable-schedule/measurements.md`.
- [x] 9.2 Deploy with `scripts/deploy.sh`. Verify that `/etc/gameclockd/config.toml` exists with mode 0644, and that the panel shows the clock as before. Ask the user to look at the panel.
- [x] 9.3 Run "Idle clock with a config file": `scripts/measure.sh` for 60 s with the clock only. Record CPU, voluntary context switches, and RSS.
- [x] 9.4 Run "Reload time on the Pi": time `sudo gameclockd ctl reload` 5 times. Record the median. Then edit the file on the Pi to a different color, send `reload`, and ask the user to look at the panel. Restore the file, send `reload`, and run the deploy again to verify "Deploy keeps an edited config".
- [x] 9.5 Run "A config error is not restarted": put an error in the file, and restart the service. Verify the exit status 2, that systemd does not restart it, and the error in `journalctl`. Restore the file and start the service.
- [x] 9.6 Start the legacy clock again on the Pi, and verify that `gameclockd` stops.

## 10. Docs and final verification

- [x] 10.1 Update `glossary.md` (config file, section, built-in table, reload), `roadmap.md` (mark `configurable-schedule` done), the README status, and `CHANGELOG.md`. Verify with `scripts/lint-docs.sh`.
- [ ] 10.2 Run `cargo fmt --all --check`, `cargo clippy --all-targets -- -D warnings`, the clippy and test runs with `--features simulator`, `cargo test`, `cargo deny check`, `scripts/clippy-pi.sh`, `openspec validate --all --strict`, and `scripts/lint-docs.sh`. All must pass.
- [ ] 10.3 Push the branch, open the PR, and verify that all CI jobs pass.

## Workflow follow-up

- After review, archive the change as the last commit on the PR (`/opsx:archive`), before the merge.
