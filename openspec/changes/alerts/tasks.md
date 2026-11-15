# Tasks

## 1. ADR

- [x] 1.1 Write ADR-0018 (alerts are data, with priorities and timed triggers). Link ADR-0017, and add a link back from ADR-0017 under "Related" as a small note. Add ADR-0018 to the index in `docs/adr/README.md`. Verify with `scripts/lint-docs.sh`.

## 2. Shared parts: scroll path and effects (`alerts`, `message-board`)

- [x] 2.1 Red: write `ScrollPath` tests for the 7×13 font, with the numbers of the `Scroll` tests. Also write them for the 10×20 font: x = 39 at T + 1 s, and the second pass at T + 12.2 s. See them fail because `ScrollPath` does not exist.
- [x] 2.2 Green: move the scroll math from `Scroll` into `message::ScrollPath`, with the glyph width as an argument (design D3). Verify that the 2.1 tests and all earlier `message` tests pass with no change to them.
- [x] 2.3 Red: write `effect` tests: `wheel` at positions 0, 255, 256, 300, 501, 600, and 764. `wheel_position` at steps 0, 100, 167, 200, and 255 (wrap). `contrast` for `(0,0,255)`, `(211,44,0)`, and `(10,245,0)`, and for a color at exactly 145,000. See them fail.
- [x] 2.4 Green: add `gameclock_core::effect` with `WHEEL_STEP`, `wheel_position`, `wheel`, and `contrast` (D3). Verify that the 2.3 tests pass.

## 3. Alert definition and scene (`alerts`)

- [x] 3.1 Red: write `AlertScene` tests with reference drawings for "Wheel at the start" and "Wheel position comes from the time". Also write "White text on a dark color" and "Black text on a light color". Also write "Next change is the next wheel step" and "Panic finishes after 20 s". See them fail because the `alert` module does not exist.
- [x] 3.2 Green: add `Alert`, `Background`, `TextStyle`, `TextColor`, `Motion`, `ClearRule`, `Priority`, the `panic` entry of `BUILT_IN`, and `AlertScene` (D1, D2). Verify that the 3.1 tests pass.
- [x] 3.3 Red: write tests for "Bedtime text enters at the right edge" and "Second pass starts at the right edge". Also write "Duration ends the bedtime alert before its passes". Also write "Bed-flash frame", "Bed-flash next change is its finish", and "Two definitions with the same parts give the same frames". Also write "Panic at night" and "Bedtime on a bedtime night". Test `find` for each name and for an unknown name. See them fail.
- [x] 3.4 Green: add the `bedtime` and `bed-flash` entries, the still and scroll motions, the brightness, and `find`. Verify that the 3.3 tests pass.
- [x] 3.5 Red: add a test to `tests/no_alloc.rs` for each built-in alert: update a compositor with the alert scene on the stack 1000 times, spread evenly before the alert finishes. It must make zero heap allocations. It passed at once, so a `String` clone in `render` for a short time made it fail (panic). Then the clone was removed.
- [x] 3.6 Green: make the 3.5 tests pass with no allocation in `render`, `next_change`, `is_finished`, or `brightness`. Verify with `cargo test -p gameclock-core --test no_alloc`.

## 4. Timed triggers (`alerts`)

- [x] 4.1 Red: write `Trigger` and `Triggers` tests in UTC for "Flash on a bedtime night" (fire times 23:00:00 and 23:00:10). Also write "No trigger on a Friday" and "Last trigger". Also write "Late wake" (one alert at 23:00:35, next time 23:00:40), and the first fire time at or after a start time inside a window. Test that `next()` at Monday 12:00:05 is Monday 23:00:00. See them fail.
- [x] 4.2 Green: add `Trigger`, `Trigger::legacy()`, and `Triggers` with `new`, `due`, and `next` (D5). Verify that the 4.1 tests pass.

## 5. Board (`alerts`, `message-board`)

- [x] 5.1 Red: write `Queue::put_back` tests: it puts a message first, also when 8 wait, and `push` then gives `QueueFull`. See them fail.
- [x] 5.2 Green: add `Queue::put_back`. Verify that the 5.1 tests pass.
- [x] 5.3 Red: write `Board` tests with a compositor and injected time for "Alert over the clock" and "Panic interrupts bedtime". Also write "Lower priority is refused", "Bed-flash does not interrupt a message", and "Same alert again". Also write "Message waits during an alert" and "Message restarts after panic". Also write "Interrupted message goes first" and "Interrupted message with a full queue". Also write "Clear removes panic", "Clear keeps bedtime", and "Clear during an alert that clear keeps". Also write "Stop an alert that clear keeps" and "Stop with no alert". Test `messages()` and `alert_name()`. See them fail because the `board` module does not exist.
- [x] 5.4 Green: add `Board` with `advance`, `message`, `alert`, `stop_alert`, `clear`, `messages`, `alert_name`, and the `Refused` error (D4). Verify that the 5.3 tests pass.

## 6. Commands (`alerts`, `command-input`)

- [ ] 6.1 Red: write `parse_command` tests for `alert panic`, `alert bedtime`, `alert bed-flash`, and `alert stop`. Also write "No name", "Unknown name", and `alert fire now` as the unknown name `fire now`. Change the unknown-command test to the new reply. Write the refusal reply test for a message and for an alert. See them fail.
- [ ] 6.2 Green: add `Command::Alert`, `Command::StopAlert`, `CommandError::NoAlertName`, `CommandError::UnknownAlert`, and the refusal reply (D6). The list of names comes from `BUILT_IN`. Verify that the 6.1 tests and all earlier command tests pass.
- [ ] 6.3 Red: change the `status_reply` tests to the `alert=` field. Add a test for `alert=panic`. See them fail.
- [ ] 6.4 Green: add the alert name to `status_reply`. Verify that the 6.3 tests pass.

## 7. Loop (`alerts`, `message-board`, `command-input`)

- [ ] 7.1 Red: write `run` tests with the scripted wait. Write "Alert over the clock" (the present comes before the `ok` reply), and "Status during an alert". Also write "Wake before the next minute", "No early wake outside a window", and "Bedtime alert repeats with no gap". Also write "Refused trigger during a message", and a trigger in `test-pattern` mode. Change the status tests to the new reply. See them fail.
- [ ] 7.2 Green: replace the queue and `show` in `run` with `Board` and `Triggers` (D4, D5). Handle `alert`, `alert stop`, and the new `clear`. Use the earlier of the compositor deadline and `triggers.next()` as the wait deadline. Verify that the 7.1 tests and all earlier `run` tests pass.
- [ ] 7.3 Red: write the `tests/cli.rs` test "Alert through the client": `ctl alert panic` gives `ok`, and `ctl alert fire` exits with status 1. See it fail. Then make it pass in the default and the `simulator` builds.
- [ ] 7.4 Update `docs/design/architecture.md`: the `effect`, `alert`, and `board` modules, the board in the loop, the trigger wake, and the `alert` command. Verify with `scripts/lint-docs.sh`.

## 8. Simulator verification

- [ ] 8.1 Run the `simulator` build and send `alert panic`, `alert bedtime`, and `alert bed-flash`. Verify by eye that the terminal shows the wheel with the contrast text, the large scroll, and the red flash.

## 9. On-device verification and measurements

- [ ] 9.1 Build with `scripts/build-pi.sh` and run `scripts/check-pi-binary.sh`. Record the binary size against main in `openspec/changes/alerts/measurements.md`.
- [ ] 9.2 Deploy to the Pi with `scripts/deploy.sh` and start the service. Send `alert panic`, then `alert bedtime`, then `alert bed-flash`. Ask the user to look at the panel: the wheel moves smoothly, the text contrast changes, the large text fits, and the flash is red. Ask if the wheel speed is good.
- [ ] 9.3 Run "Panic on the Pi": start `panic` each 20 s from a shell loop, wait 5 s, then run `scripts/measure.sh` for 60 s. Record per-thread CPU %, voluntary context switches, and RSS. Do the same for "Bedtime on the Pi". The main thread must use 15% CPU or less in each run.
- [ ] 9.4 Stop the service. Run `gameclockd --led-show-refresh` by hand with a socket in `/tmp`, and start each alert from a second shell. Record the median and the 5th percentile of the refresh rate during each alert. The median must be 170 Hz or more.
- [ ] 9.5 Send a message, then `alert panic`, then `clear`. Then send `alert bedtime`, `clear`, and `alert stop`. Ask the user to verify the restart of the message after `panic`, and the clear rules. Verify the `alert=` field of `status`.
- [ ] 9.6 Start the legacy clock again on the Pi, and verify that `gameclockd` stops.

## 10. Docs and final verification

- [ ] 10.1 Update `glossary.md` (alert, alert definition, priority, clear rule, timed trigger, color wheel, board), `roadmap.md` (mark `alerts` done, and name the config-file follow-up under `configurable-schedule`), the README, and `CHANGELOG.md`. Verify with `scripts/lint-docs.sh`.
- [ ] 10.2 Run `cargo fmt --all --check`, `cargo clippy --all-targets -- -D warnings`, the clippy and test runs with `--features simulator`, `cargo test`, `cargo deny check`, `scripts/clippy-pi.sh`, `openspec validate --all --strict`, and `scripts/lint-docs.sh`. All must pass.
- [ ] 10.3 Push the branch, open the PR, and verify that all CI jobs pass.

## Workflow follow-up

- After review, archive the change as the last commit on the PR (`/opsx:archive`), before the merge.
