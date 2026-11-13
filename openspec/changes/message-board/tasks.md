# Tasks

## 1. ADR

- [x] 1.1 Write ADR-0017 (messages scroll from the injected time, and the daemon owns the queue). Add it to the index in `docs/adr/README.md`. Verify with `scripts/lint-docs.sh`.

## 2. Compositor (`scene-model`)

- [x] 2.1 Red: write compositor tests for "Next scene with no base frame between", "Unfinished scene stays", and "Base scene after the removal". Run `cargo test -p gameclock-core` and see them fail because `remove_finished` and `clear_foreground` do not exist.
- [x] 2.2 Green: add `Compositor::remove_finished` and `Compositor::clear_foreground`, and make `update` call `remove_finished` (design D4). Verify that the 2.1 tests and all earlier compositor tests pass.

## 3. Message, scroll, and queue (`message-board`)

- [x] 3.1 Red: write `Message::new` tests: the defaults, the 1 to 64 character limit (counted in characters), and the 1 to 10 pass limit. Also write "Spaces in the text stay". See them fail because the `message` module does not exist.
- [x] 3.2 Green: add `Message`, `MessageError`, and the default constants (D2). Verify that the 3.1 tests pass.
- [x] 3.3 Red: write `Scroll` tests with a reference drawing for "Text enters at the right edge" and "Position comes from the time". Also write "The next pass starts at the right edge". Also write "Character that the font does not have", "Finish after the passes", and "Next change is the next step". Also write "Message at night" and "Message in the morning". See them fail.
- [x] 3.4 Green: add `Scroll` with `STEP = 40 ms` and the position from the time (D1). Verify that the 3.3 tests pass.
- [x] 3.5 Red: add a test to `tests/no_alloc.rs`: a compositor with a `Scroll` on the stack is updated once. Then it is updated 1000 more times, 40 ms apart, with zero heap allocations. `Scroll` was public after 3.4, so the test passed at once. To see it fail, add a `String` clone to `render` for a short time: the test gives 1000 allocations. Then remove the clone.
- [x] 3.6 Green: make the test pass with no allocation in `render`, `next_change`, `is_finished`, or `brightness`. Verify with `cargo test -p gameclock-core --test no_alloc`.
- [x] 3.7 Red: write `Queue` tests: first in, first out. `push` gives `QueueFull` with 8 messages. `clear` empties it. `len` counts the waiting messages. See them fail.
- [x] 3.8 Green: add `Queue` on a `VecDeque` (D2). Verify that the 3.7 tests pass.

## 4. Commands (`message-board`, `command-input`)

- [x] 4.1 Red: write `parse_command` tests for "Message with the defaults" and "Message with options". Also write "No text", "Text too long", "Pass count out of range", "Color that is not correct", and "Unknown option". Test that `clear` parses and that `clear now` is an unknown command. Change the unknown-command test to the new reply. See them fail.
- [x] 4.2 Green: add `Command::Message` and `Command::Clear`, the option parser, and the error replies (D6). Verify that the 4.1 tests and all earlier command tests pass.
- [x] 4.3 Red: change the `status_reply` tests to the `messages=` field, and add a test for a count of 3. See them fail.
- [x] 4.4 Green: add the count to `status_reply` (D7). Verify that the 4.3 tests pass.

## 5. Loop (`message-board`, `command-input`)

- [x] 5.1 Red: write `run` tests with the scripted wait for "First message starts at once" and "Second message waits". Also write "Next message starts with no clock between", and "Clock returns after the last message". Also write "Queue is full", "Clear during a message", and "Clear with no message". Also write "Status during a message" and "Message over the test pattern". Change "Status of the clock" and "Status of the test pattern" to the new reply. See them fail.
- [x] 5.2 Green: add the queue to `run` with the loop steps of D3, and the `message` and `clear` handling. Verify that the 5.1 tests and all earlier `run` tests pass.

## 6. Client (`command-input`)

- [x] 6.1 Red: write `parse_args` tests for "Message with no quotes" and "Socket option after the command word". Change `ctl_needs_exactly_one_command` so that only a missing command is a usage error. Write the `tests/cli.rs` test "Message through the client". See them fail.
- [x] 6.2 Green: change `parse_ctl` to join the command words (D8), and change the usage text if needed. Verify that the 6.1 tests and all `tests/cli.rs` tests pass in the default and the `simulator` builds.

## 7. On-device verification and measurements

- [x] 7.1 Build with `scripts/build-pi.sh` and run `scripts/check-pi-binary.sh`. Record the binary size against main in `openspec/changes/message-board/measurements.md`.
- [x] 7.2 Deploy to the Pi with `scripts/deploy.sh` and start the service. Send `sudo gameclockd ctl message hello world`. Ask the user to look at the panel: the text scrolls 4 times, smoothly and with no flicker, and then the clock returns. Ask if the speed is good.
- [x] 7.3 Run "Long message on the Pi": send a message of 64 characters with `--passes=10`, wait 5 s, then run `scripts/measure.sh` for 60 s. Record per-thread CPU %, voluntary context switches, and RSS. The main thread must use 15% CPU or less.
- [x] 7.4 Stop the service. Run `gameclockd --led-show-refresh` by hand, and send the same long message from a second shell. Record the median and the 5th percentile of the refresh rate while it scrolls. The median must be 170 Hz or more.
- [x] 7.5 Send three messages and then `clear`. Ask the user to verify that the second message follows the first with no clock between, and that `clear` shows the clock at once. Verify the `messages=` count of `status`.
- [x] 7.6 Start the legacy clock again on the Pi, and verify that `gameclockd` stops.

## 8. Docs and final verification

- [x] 8.1 Update `docs/design/architecture.md` (the message module, the queue in the loop, the new commands). Update `glossary.md` (message, pass, scroll step, message queue), `roadmap.md` (mark `message-board` done), the README, and `CHANGELOG.md`. Verify with `scripts/lint-docs.sh`.
- [x] 8.2 Run `cargo fmt --all --check`, `cargo clippy --all-targets -- -D warnings`, the clippy and test runs with `--features simulator`, `cargo test`, `cargo deny check`, `scripts/clippy-pi.sh`, `openspec validate --all --strict`, and `scripts/lint-docs.sh`. All must pass.
- [ ] 8.3 Push the branch, open the PR, and verify that all CI jobs pass.

## Workflow follow-up

- After review, archive the change as the last commit on the PR (`/opsx:archive`), before the merge.
