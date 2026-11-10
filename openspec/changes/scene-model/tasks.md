# Tasks

## 1. ADRs and dependencies

- [x] 1.1 Write ADR-0012 (scenes and the compositor, `allocation-counter`) and ADR-0013 (jiff, injected time, the deadline wait). Add both to the index in `docs/adr/README.md`. Verify with `scripts/lint-docs.sh`.
- [x] 1.2 Add a status note to ADR-0010: "The signal wait is changed by ADR-0013." Verify that the rest of ADR-0010 does not change (`git diff`).
- [x] 1.3 Add `jiff` to the workspace. Give `gameclock-core` `default-features = false, features = ["std"]`. Give `gameclockd` `tz-system` and `tzdb-zoneinfo`. Add `allocation-counter` as a `gameclock-core` dev-dependency. Verify that `cargo deny check` passes and that `cargo tree -p gameclock-core -e normal` shows only `embedded-graphics` and `jiff`.

## 2. Scene contract and compositor (`scene-model`)

- [x] 2.1 Red: write compositor tests with test scenes: solid color, one pixel, timed finish, timed color change, and fixed next change. They cover "The top scene is visible" and "Each render starts from a black frame". They also cover "Finished foreground scenes go away" and "The base scene always stays". Run `cargo test -p gameclock-core` and see them fail because `Scene` and `Compositor` do not exist.
- [x] 2.2 Green: add the `Scene` trait and `Compositor::{new, push, update}` (design D1, D2). Verify that the 2.1 tests pass.
- [x] 2.3 Red: write tests for "Only a changed frame is presented" and "The deadline is the top scene's next change". Also write tests for "A scene's finish time is a change" and "Renders are at most 30 per second". See them fail for the expected reason.
- [x] 2.4 Green: add the comparison with `shown`, the first-update flag, and `Compositor::deadline` with the 33 ms cap (D2, D3). Verify that all compositor tests pass.

## 3. Heartbeat scene (`heartbeat-scene`)

- [ ] 3.1 Red: write a test for each `heartbeat-scene` scenario. The scenarios are: one changed pixel, the marker at 00:00:00, 00:00:57.999, and 00:00:58, the next change at .250 and .000, never finished, and tiny frames. See them fail because `Heartbeat` does not exist.
- [ ] 3.2 Green: add `heartbeat::Heartbeat` (D7). Verify that the 3.1 tests pass.
- [ ] 3.3 Red: write the integration test `crates/gameclock-core/tests/no_alloc.rs` for "Updates do not allocate heap memory" with `allocation_counter::measure`. Make it fail first: allocate a new frame in `update` for one run and see the count above zero. Then remove the allocation and see the test pass.

## 4. Event loop and time in the daemon (`daemon-lifecycle`)

- [ ] 4.1 Red: change the `gameclockd` unit tests to the new `run` with `FakeClock` and `ScriptedWait` (D6). Cover "One present at startup", "Frame follows the panel's size", "Wait ends at the scene deadline", and "Static scene waits with no timeout". Also cover "Heartbeat presents each second", "Early wake does not present", the present failure, and the exit status. See them fail because the new `run`, `Clock`, and `Wait` do not exist.
- [ ] 4.2 Green: add `Clock`, `SystemClock`, `Wait`, `Wake`, and the loop in `run`. Verify that the 4.1 tests pass.
- [ ] 4.3 Red: write a `SignalWait` unit test with a `UnixStream` pair. A past deadline gives `Deadline` at once. A 50 ms deadline gives `Deadline` after 50 ms or more. A byte in the pipe gives `Shutdown`. Add "No presents while waiting": a static scene and a byte written 500 ms later. See the tests fail because `SignalWait` does not exist.
- [ ] 4.4 Green: add `SignalWait` (D5). Change `main.rs` to register SIGTERM and SIGINT on the pipe and to call the new `run` with `Heartbeat`. Verify that the 4.3 tests and the existing SIGTERM and SIGINT tests in `tests/cli.rs` pass.
- [ ] 4.5 Red: write the test for "Missing time zone falls back to UTC". It calls the library function that turns the zone lookup into a `TimeZone` and a message (D8). Add a `tests/cli.rs` test for "Time zone from the environment" with `TZ=America/Los_Angeles`. See both fail.
- [ ] 4.6 Green: add the function and call it from `main.rs`. Verify that the 4.5 tests pass and that `cargo sim` shows the marker move once each second.
- [ ] 4.7 Update `docs/design/architecture.md` (scene model, data flow, daemon lifecycle, crates), `glossary.md` (scene, compositor, heartbeat, deadline), `roadmap.md` (mark `scene-model` done), and `CHANGELOG.md`. Verify with `scripts/lint-docs.sh`.

## 5. On-device verification and measurements

- [ ] 5.1 Build with `scripts/build-pi.sh` and run `scripts/check-pi-binary.sh`. Record the binary size before and after this change in `openspec/changes/scene-model/measurements.md`.
- [ ] 5.2 Deploy to the Pi with `scripts/deploy.sh`. Verify by eye that the marker moves once each second and that the test pattern did not change.
- [ ] 5.3 Run "Heartbeat daemon wakes only each second (on-device)": wait 10 s, then run `scripts/measure.sh` for 60 s. Record the per-thread CPU %, voluntary context switches, RSS, and the `--led-show-refresh` rate in `measurements.md`. Compare them with the bring-up baseline. Stop and report if the main thread uses 1% or more of one core, or if the refresh rate falls by more than 2%.
- [ ] 5.4 Start the legacy clock again on the Pi, and verify that `gameclockd` stops.

## 6. Final verification

- [ ] 6.1 Run `cargo fmt --all --check`, `cargo clippy --all-targets -- -D warnings`, the clippy and test runs with `--features simulator`, `cargo test`, `cargo deny check`, `scripts/clippy-pi.sh`, `openspec validate --all --strict`, and `scripts/lint-docs.sh`. All must pass.
- [ ] 6.2 Push the branch, open the PR, and verify that all CI jobs pass.

## Workflow follow-up

- After review, archive the change as the last commit on the PR (`/opsx:archive`), before the merge.
