# Tasks

## 1. ADR

- [x] 1.1 Write ADR-0016 (command input over a Unix socket, and an event channel for the loop). Add it to the index in `docs/adr/README.md`. Add a status note to ADR-0013: "The deadline wait is changed by ADR-0016." Verify with `scripts/lint-docs.sh` and `git diff docs/adr/0013-time-and-deadline-wait.md`.

## 2. Compositor (`scene-model`)

- [x] 2.1 Red: write compositor tests for "New base scene shows at the next update" and "Foreground scenes stay on top". Also write "Same image still gives a frame". Also write "Last brightness after an update" and "Last brightness before an update". Run `cargo test -p gameclock-core` and see them fail because `set_base` and `last_brightness` do not exist.
- [x] 2.2 Green: add `Compositor::set_base` and `Compositor::last_brightness` (design D6). Verify that the 2.1 tests and `tests/no_alloc.rs` pass.

## 3. Commands and replies (`command-input`)

- [x] 3.1 Red: write `parse_command` tests for "Unknown command", "Line too long", "Text that is not UTF-8", and "Spaces around the command". Also test that `clock`, `test-pattern`, and `status` parse. Write tests for the exact text of each reply. See them fail because the items do not exist.
- [x] 3.2 Green: add `Command`, `CommandError`, `parse_command`, and the reply functions (D4). Verify that the 3.1 tests pass.

## 4. Event channel and loop (`daemon-lifecycle`, `command-input`)

- [x] 4.1 Red: change the `run` tests to a scripted wait that can give `Wake::Command`. Write tests for "A command ends the wait", "Switch to the test pattern", "Switch back to the clock", and "Same mode does not present". Also write "Status of the clock" and "Status of the test pattern". Verify that each reply comes after the present. See them fail because `Wake::Command` does not exist.
- [x] 4.2 Green: add `Event`, `Request`, `Wake::Command`, and the command handling in `run` (D1, D5). Verify that the 4.1 tests and all earlier `run` tests pass.
- [x] 4.3 Red: write `ChannelWait` tests. A past deadline gives `Deadline` at once. A 50 ms deadline gives `Deadline` after 50 ms or more. A `Shutdown` event gives `Shutdown`. A request gives `Command`. A closed channel gives `Shutdown`. See them fail because `ChannelWait` does not exist.
- [x] 4.4 Green: add `ChannelWait`. Remove `SignalWait` and its tests. Verify that the 4.3 tests pass.

## 5. Socket and client (`command-input`)

- [x] 5.1 Red: write socket tests in a temporary directory. The file mode is 0600. A stale file is replaced. A live socket is not taken. Also write "Silent client is closed" and "Loop does not answer" with a fake loop. See them fail because the socket code does not exist.
- [x] 5.2 Green: add the listener setup and the socket thread with its 1 s read limit and 2 s reply limit (D2, D3). Verify that the 5.1 tests pass.
- [x] 5.3 Red: write `parse_args` tests for "Socket path is not a driver flag" and "Default socket path". Also write "Empty socket path is a usage error" and the `ctl` forms. Change the existing tests to the new result type. See them fail.
- [x] 5.4 Green: add `Invocation`, `CtlArgs`, and the socket path in `Args` (D7). Verify that the 5.3 tests pass.
- [x] 5.5 Red: write `tests/cli.rs` tests for "Socket at the given path" and "The socket file goes away at shutdown". Also write "Socket directory does not exist" and "The heartbeat continues during a silent connection". Also write "Status through the client", "Error reply through the client", "No daemon", and "Missing command". Run the SIGTERM and SIGINT tests too. See the new tests fail.
- [x] 5.6 Green: change `main.rs`: the signal thread, the socket before the panel, the `ctl` client, and the removal of the socket file at exit. Verify that all `tests/cli.rs` tests pass in the default and the `simulator` builds.

## 6. Service unit (`pi-deployment`)

- [x] 6.1 Red: add the `RuntimeDirectory=gameclockd` and `RuntimeDirectoryMode=0700` lines to `scripts/test-unit.sh`. Run it in the `gameclock-linux-test` image and see it fail.
- [x] 6.2 Green: add the lines to `deploy/gameclockd.service` (D8). Verify that `scripts/test-unit.sh` passes.

## 7. On-device verification and measurements

- [x] 7.1 Build with `scripts/build-pi.sh` and run `scripts/check-pi-binary.sh`. Record the binary size against main in `openspec/changes/command-input/measurements.md`.
- [x] 7.2 Deploy to the Pi with `scripts/deploy.sh`. Run "Socket on the Pi" and "Commands from root only".
- [x] 7.3 Run "Clock daemon wakes only each minute": wait 10 s, then run `scripts/measure.sh` for 60 s. Record per-thread CPU %, voluntary context switches, RSS, and the refresh rate. Compare them with the `clock-scene` numbers.
- [ ] 7.4 Time 20 `sudo gameclockd ctl status` commands. Record the median and the slowest. Each must be less than 100 ms, not counting `sudo`.
- [x] 7.5 Send `test-pattern`, then `clock`. Ask the user to look at the panel and verify that it changes each time. Then stop the service and run "Directory goes away at stop".
- [x] 7.6 Start the legacy clock again on the Pi, and verify that `gameclockd` stops.

## 8. Docs and final verification

- [x] 8.1 Update `docs/design/architecture.md` (the event channel, the threads, the socket, the commands, and the startup order). Update `glossary.md` (control socket, command, reply, `ctl`), `roadmap.md` (mark `command-input` done), the README, and `CHANGELOG.md`. Verify with `scripts/lint-docs.sh`.
- [ ] 8.2 Run `cargo fmt --all --check`, `cargo clippy --all-targets -- -D warnings`, the clippy and test runs with `--features simulator`, `cargo test`, `cargo deny check`, `scripts/clippy-pi.sh`, `openspec validate --all --strict`, and `scripts/lint-docs.sh`. All must pass.
- [ ] 8.3 Push the branch, open the PR, and verify that all CI jobs pass.

## Workflow follow-up

- After review, archive the change as the last commit on the PR (`/opsx:archive`), before the merge.
