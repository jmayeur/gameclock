# Tasks

## 1. ADRs

- [x] 1.1 Write ADR-0014 (scene brightness and the time-of-day schedule as data). Add it to the index in `docs/adr/README.md`. Link it from ADR-0012's related line as a mechanical note only. Verify with `scripts/lint-docs.sh` and `git diff docs/adr/0012-scenes-and-compositor.md`.

## 2. Scene brightness (`scene-model`)

- [x] 2.1 Red: write compositor tests for "Scenes give a brightness" (default full, top scene's brightness) and "Changed brightness gives a frame". Write the heartbeat test "Full brightness at each time". Change the existing `update` tests to the new result type. Run `cargo test -p gameclock-core` and see them fail because `Scene::brightness` and the new result do not exist.
- [x] 2.2 Green: add `Scene::brightness` with the default `Brightness::MAX`. Make `Compositor::update` give the frame and the brightness, and compare both (design D1). Verify that the 2.1 tests and `tests/no_alloc.rs` pass. `Heartbeat` does not change: the default method gives its brightness.

## 3. Schedule (`clock-scene`)

- [x] 3.1 Red: write `schedule` tests for "Rule times are whole minutes" and "A rule must not be empty". Also write tests for "First matching rule wins", "End time is not part of the rule", "Weekday set limits a rule", and "No rule matches". See them fail because `schedule` does not exist.
- [x] 3.2 Green: add `TimeOfDay`, `Weekdays`, `Style`, `Rule`, and `Schedule::style_at` (D2). Verify that the 3.1 tests pass.
- [x] 3.3 Red: write tests for "Each period of the legacy schedule", "Bedtime nights are Sunday to Thursday", and "Weekday of the local date". See them fail because `Schedule::legacy` does not exist.
- [x] 3.4 Green: add `Schedule::legacy()`. Verify that the 3.3 tests pass.

## 4. Clock scene (`clock-scene`)

- [x] 4.1 Red: write a `clock` test for each scenario of "The clock shows the local time in the legacy format". Also write "Text color and brightness from the style". The expected frame has the text in `FONT_7X13_BOLD` on a new frame. The text is at (8, 20) with `Baseline::Alphabetic`. See them fail because `Clock` does not exist.
- [x] 4.2 Green: add `clock::Clock` with `render` and `brightness` (D3). Verify that the 4.1 tests pass.
- [x] 4.3 Red: write tests for "Next change inside a minute" and "Next change on a whole minute". Also write tests for "Next change across the daylight saving time gap" and "Never finished". See them fail for the expected reason with a stub `next_change`.
- [x] 4.4 Green: implement `next_change` and `is_finished`. Verify that the 4.3 tests pass.
- [x] 4.5 Red: add "One thousand minutes allocate nothing" to `tests/no_alloc.rs`. Make it fail first with a `String` in `render` for one run. Then remove it and see the test pass.

## 5. Daemon (`daemon-lifecycle`)

- [x] 5.1 Red: write `run` tests for "Brightness of the base scene" and "Brightness change presents the same image". Change "One present at startup" to the legacy clock at brightness 25. See them fail because `run` presents at full brightness.
- [x] 5.2 Green: make `run` present at the compositor's brightness. Verify that the 5.1 tests and all other `gameclockd` tests pass.
- [x] 5.3 Red: write `parse_args` tests for "Test-pattern mode is not a driver flag" and the existing usage errors with the new result type. Add `tests/cli.rs` tests for "Host build names the clock" and "Host build names the test pattern". See them fail.
- [x] 5.4 Green: change `parse_args` to give the mode and the driver flags (D4). Make `main.rs` build the clock or the heartbeat and write the base scene line. Verify that the 5.3 tests pass and that `cargo sim` shows the clock.

## 6. Service unit (`pi-deployment`)

- [x] 6.1 Red: change `scripts/test-unit.sh` to expect `ExecStart=/usr/local/bin/gameclockd --led-pwm-bits=8 $GAMECLOCKD_ARGS`. Run it in the `gameclock-linux-test` image and see it fail.
- [x] 6.2 Green: change `deploy/gameclockd.service` (D5). Change the comment example in the unit so it does not suggest a second PWM-bits value. Verify that `scripts/test-unit.sh` passes.

- [ ] 6.3 Red: change the `rgbmatrix-sys` test "Built-in defaults describe this Pi" to expect `adafruit-hat-pwm`. Run `cargo test -p rgbmatrix-sys` and see it fail on the mapping.
- [ ] 6.4 Green: change `DEFAULTS.hardware_mapping` to `adafruit-hat-pwm` (design D6). Verify that the 6.3 test passes and that `scripts/clippy-pi.sh` passes.

## 7. On-device verification and measurements

- [x] 7.1 Build with `scripts/build-pi.sh` and run `scripts/check-pi-binary.sh`. Record the binary size against main in `openspec/changes/clock-scene/measurements.md`.
- [ ] 7.2 Deploy to the Pi with `scripts/deploy.sh`. Verify "Service replaces the legacy clock": the legacy clock stops, and the panel shows the time in the color of the current schedule period.
- [x] 7.3 Run "Clock daemon wakes only each minute": wait 10 s, then run `scripts/measure.sh` for 60 s with `--led-pwm-bits=8`. Record per-thread CPU %, voluntary context switches, RSS, and the `--led-show-refresh` rate.
- [x] 7.4 Repeat 7.3 with `--led-pwm-bits=11` in `/etc/default/gameclockd`. Record the same values. Then remove the override.
- [ ] 7.5 Run "Clock colors at 8 PWM bits": add a temporary systemd drop-in with a `TZ` value. Select the value so that the local time is in a red 25 period. Then do the same for a mellow 25 period. Ask the user to look at each. Remove the drop-in.
- [x] 7.6 Run "Heartbeat daemon wakes only each second" with `--test-pattern` for 60 s, to show the diagnostic mode still works. Record the values.
- [ ] 7.7 Write ADR-0015 (driver settings for the clock: the `adafruit-hat-pwm` mapping and 8 PWM bits). Use the 7.3 to 7.5 results and the mapping comparison. Add it to the index. Add a status note to ADR-0009 that ADR-0015 changes its default mapping. If the results do not support 8 bits, change the `pi-deployment` delta, the unit, and `scripts/test-unit.sh` back before you continue. Verify with `scripts/lint-docs.sh`.
- [ ] 7.9 Deploy the build with the 6.4 default. Run "The PWM mapping has no moving lines" with no flag in `/etc/default/gameclockd`. Measure the refresh rate and CPU again, and record them.
- [ ] 7.8 Start the legacy clock again on the Pi, and verify that `gameclockd` stops.

## 8. Docs and final verification

- [ ] 8.1 Update `docs/design/architecture.md` (clock, schedule, brightness flow, base scene selection). Update `glossary.md` (schedule, rule, style, bedtime night, test-pattern mode), `roadmap.md` (mark `clock-scene` done). Update `docs/hardware/pi-zero-w.md` (the mapping, the GPIO4↔18 link, the glitch finding, and PWM bits), the README status line, and `CHANGELOG.md`. Verify with `scripts/lint-docs.sh`.
- [ ] 8.2 Run `cargo fmt --all --check`, `cargo clippy --all-targets -- -D warnings`, the clippy and test runs with `--features simulator`, `cargo test`, `cargo deny check`, `scripts/clippy-pi.sh`, `openspec validate --all --strict`, and `scripts/lint-docs.sh`. All must pass.
- [ ] 8.3 Push the branch, open the PR, and verify that all CI jobs pass.

## Workflow follow-up

- After review, archive the change as the last commit on the PR (`/opsx:archive`), before the merge.
- After the sync, change the Purpose of `openspec/specs/heartbeat-scene/spec.md`: the heartbeat is a diagnostic mode, not a temporary base scene. The sync does not change a Purpose.
