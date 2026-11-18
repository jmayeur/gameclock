# Tasks

## 1. ADR

- [x] 1.1 Write ADR-0021: page data comes from the system `curl` in a `fetch` thread, and `serde_json` parses it. Record the process boundary and the deployment assumption (`curl` and the CA certificates on the Pi). Record the measured cost of each fetch. Record the license reason against a Rust TLS client (`ring` is `Apache-2.0 AND ISC`). Add ADR-0021 to the "Related" line of ADR-0016 and ADR-0020 as a small note, and to the index in `docs/adr/README.md`. Verify with `scripts/lint-docs.sh`.

## 2. AQI and values in the core (`sensor-pages`)

- [x] 2.1 Red: write `page` module tests for "AQI values" and "Category colors", and for `PageName::parse` and `name`. See them fail because the module does not exist.
- [x] 2.2 Green: add `gameclock-core/src/page.rs` with `PageName`, `aqi`, `aqi_color`, and the EPA tables as `const` arrays (design D5). Verify that the 2.1 tests pass.
- [x] 2.3 Red: write tests for the lines of a page: "Outdoor page", "Indoor page", "Below zero", and "Old reading" (20 min old gives dashes). Write "Weather page", "No chance of rain", "Skip an old period", "No current period", and "Text that is not ASCII". Use injected times. See them fail.
- [x] 2.4 Green: add `Reading`, `Forecast`, `Period`, `PageData`, the 15 min age rule, and the choice of the current period (D4). Verify that the 2.3 tests pass.

## 3. The page scene (`sensor-pages`)

- [x] 3.1 Red: write `PageScene` tests: "Page ends after 30 s", "Short text is still", and "Long text scrolls" (10 px in 400 ms). Write "Page at night" (brightness 25, red lines), and the AQI color in the frame. Write a test that `update` allocates no heap memory, with `allocation-counter`. See them fail.
- [x] 3.2 Green: add `PageScene` with the two layouts and `next_change` (D5). Verify that the 3.1 tests pass.

## 4. The board (`sensor-pages`, `alerts`)

- [x] 4.1 Red: write `Board` tests: "A page replaces a page" and "Page over a message" (the message restarts after). Write "Message during a page" and "Clear removes a page". Write "Bed-flash does not interrupt a page" and "Bedtime interrupts a page" (the clock shows after, not the page). Test the `page_blocker` result during `panic`. See them fail.
- [x] 4.2 Green: add `Showing::Page`, `Priority::PAGE`, `Blocker::Page`, `Board::page`, `Board::page_blocker`, and `Board::page_name` (D6). Verify that the 4.1 tests and all earlier board tests pass.

## 5. Config (`daemon-config`)

- [x] 5.1 Red: write config tests: "Pages from the file", "No pages section", "URL with a different scheme". Write "URL that starts with a dash", "URL with a space", a URL of 257 characters, and "Unknown page key". Use `example` hosts only. See them fail.
- [x] 5.2 Green: add `PagesSection`, `PageUrls`, and the URL rules with the line and column (D7). Run the 5.1 tests, and see them pass.
- [x] 5.3 Add a commented `[pages]` example with placeholder hosts to `deploy/config.toml`, and a `[pages]` section to `docs/config.md`. Verify that "The example file gives the legacy tables" passes and that the file has no real address.

## 6. Commands and the loop (`sensor-pages`, `command-input`)

- [x] 6.1 Red: write `parse_command` tests for `page outdoor`, `page`, and `page garage`, and the new "Unknown command" reply. Write `status_reply` tests for all the status scenarios with `page=`. See them fail.
- [x] 6.2 Green: add `Command::Page`, `Command::PageNames`, the errors, and `page=` in `status_reply` (D7). Verify that the 6.1 tests pass.
- [ ] 6.3 Red: write `run` tests with a `RecordingFetch` and a `ScriptedWait` step that returns `Wake::Fetched` (D2). Write "Request a page", "Page with no URL", "No request, no fetch", and "The loop does not wait". Write "The newest request wins", "Request during a higher alert", and "Data arrives during a higher alert". Write "Fetch fails" (the stderr line), "Body is not JSON", "Status during a page", and "Reload changes the URLs". See them fail.
- [ ] 6.4 Green: add the `Fetch` trait, `Wake::Fetched`, `Event::Fetched`, the `seq` and `pending` state, and the parse of both JSON forms in a daemon `pages` module with `serde_json` (D1–D4). Add `serde_json` to the workspace. Verify that the 6.3 tests pass and that `cargo deny check` passes.

## 7. The fetch thread (`sensor-pages`)

- [ ] 7.1 Red: write `fetch` tests against a local test HTTP server on `127.0.0.1:0` in the test: a good body and "Slow server" (no response for 10 s). Write "Body too large" (300 KiB with no length), a 404, and "User-Agent". Test that the URL goes after `--`. See them fail.
- [ ] 7.2 Green: add the `fetch` thread and `ChannelFetch` with the fixed `curl` arguments and the 256 KiB read limit (D1). Start the thread in `main.rs`. Verify that the 7.1 tests pass in the default and the `simulator` builds.

## 8. The web page (`web-control`)

- [ ] 8.1 Red: write `route` tests: "Page buttons from the config", "No pages in the config", and "Request a page". Write "Unknown page from the form", and "Page shows the status" with `page=`. Change the fake loop of the web tests to answer `PageNames`. See them fail.
- [ ] 8.2 Green: add `POST /page`, the page buttons, and `page` in the status line (D7). Verify that the 8.1 tests and all web tests pass.

## 9. Simulator verification

- [ ] 9.1 Run the `simulator` build with a config in the scratchpad that has the real URLs (not in the repo) and `--http=127.0.0.1:8080`. Post each page and look at the frames. Verify that the layouts fit in 64×32 and that a long forecast text scrolls. Ask the user to look at the page buttons and the frames. Record the results here, with no addresses.

## 10. On-device verification and measurements

- [ ] 10.1 Build with `scripts/build-pi.sh`, and run `scripts/check-pi-binary.sh`. Record the binary size against `main` in `openspec/changes/sensor-pages/measurements.md`.
- [ ] 10.2 Deploy with `scripts/deploy.sh`. Ask the user before the deploy. With approval, add the `[pages]` section with the real URLs to `/etc/gameclockd/config.toml` on the Pi, and run `sudo gameclockd ctl reload`. Verify that `curl` runs as the `daemon` user and fetches each page.
- [ ] 10.3 Run "Refresh rate during a fetch": `scripts/measure.sh` with no requests, and then with a `page weather` request each 5 s. Record both refresh rates and the CPU. If the rate falls by more than 10%, add `nice` (design risk 1), and measure again.
- [ ] 10.4 Run "Memory after requests": record the RSS before and after 10 page requests.
- [ ] 10.5 Ask the user to press each page button on a phone and to verify the panel. Record the result.

## 11. Docs and final verification

- [ ] 11.1 Update `architecture.md` (the `fetch` thread, the `Fetched` event, the page in the board, and the priorities) and `glossary.md` (page, reading, AQI, fetch). Update `roadmap.md` (add `sensor-pages` as done, and the font, icon, and layout items from the exploration), the README, and `CHANGELOG.md` (with the rollback note about `[pages]`). Verify with `scripts/lint-docs.sh`.
- [ ] 11.2 Run `cargo fmt --all --check`, `cargo clippy --all-targets -- -D warnings`, the clippy and test runs with `--features simulator`, `cargo test`, `cargo deny check`, `scripts/clippy-pi.sh`, `openspec validate --all --strict`, and `scripts/lint-docs.sh`. All must pass. Run `cargo fmt` before each commit.
- [ ] 11.3 Push the branch, open the PR, and verify that all CI jobs pass.

## Workflow follow-up

- After review, archive the change as the last commit on the PR (`/opsx:archive`), before the merge.
