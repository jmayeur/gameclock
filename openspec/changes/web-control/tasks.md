# Tasks

## 1. ADR

- [x] 1.1 Write ADR-0020 (the daemon serves the web page with its own small HTTP server). Give the `tiny_http` findings (no header limit, no read timeout, one thread for each connection) and the `tiny_http_dh` findings. Link ADR-0010 and ADR-0016, and add ADR-0020 to the "Related" line of ADR-0016 as a small note. Add it to the index in `docs/adr/README.md`. Verify with `scripts/lint-docs.sh`.

## 2. The HTTP parser and responses (`web-control`)

- [x] 2.1 Red: write `http::parse` unit tests: a complete `GET`, a `POST` with a body, a `Partial` head, a `Partial` body, and LF-only line ends. Write the size refusals: a head over 8 KiB (431), with a 9000-byte header and no line end, and a body over 1 KiB (413). Write the form refusals: `Transfer-Encoding` with no `Content-Length` (411), a bad or doubled `Content-Length` (400), and `HTTP/2.0` (400). See them fail because the module does not exist.
- [x] 2.2 Green: add `http.rs` with `parse`, `Request`, `Status`, and the header lookup with no case (design D1, D2). Verify that the 2.1 tests pass.
- [x] 2.3 Red: write a random-bytes test with a fixed seed and a small generator in the test (no new dependency). Feed 10,000 random inputs and random cuts of good requests to `parse`. Verify that it never panics and never reports a head over 8 KiB. See it fail by a short change that removes the head limit, then restore it.
- [x] 2.4 Red: write `Response` tests: the status line, `Content-Length`, `Connection: close`, `Cache-Control: no-store`, `X-Content-Type-Options: nosniff`, and no `Access-Control-` header. See them fail.
- [x] 2.5 Green: add `Response` and its bytes. Verify that the 2.4 tests pass.

## 3. Routes, forms, and the page (`web-control`)

- [x] 3.1 Red: write form decoder tests: `+`, `%XX`, a bad `%G1`, `%FF` that is not UTF-8, a doubled field (the last wins), and a `Content-Type` with `; charset=UTF-8`. See them fail.
- [x] 3.2 Green: add the form decoder in `web.rs` (D5). Verify that the 3.1 tests pass.
- [ ] 3.3 Red: write `route` tests with a fake `ask` that records the commands. Write "Send a message", "Message with the defaults", "Message with white space", "Message that is too long", and "Empty message". Write "Bad passes or color" (also a color with `#`). Write "Start an alert", "Unknown alert", "Stop and clear", and "The loop does not answer". Write "Legacy panic button" and "Legacy bedtime button". Write "Post that is not a form", "Bad percent code", and "Unknown path and wrong method". See them fail.
- [ ] 3.4 Green: add `route`, the mapping to `Command`, and the error text from `command.rs` (D3, D5). Verify that the 3.3 tests pass.
- [ ] 3.5 Red: write page tests: "Page with the built-in alerts", "Page shows the status", and "Text from data is escaped". Test the `Content-Security-Policy` header. See them fail.
- [ ] 3.6 Green: add the page with its style in the HTML, the HTML escape, and the parse of the `status` reply (D4). Verify that the 3.5 tests pass. Ask the user to look at the page in a browser on the dev machine with the `simulator` build.
- [ ] 3.7 Red: write "Health with a loop" and "Health with no answer". See them fail.
- [ ] 3.8 Green: add `GET /health`. Verify that the 3.7 tests pass.
- [ ] 3.9 Red: write "Post from the page", "Post from another site", an `Origin` of `null`, an `Origin` with a different port, and "No CORS headers". See them fail.
- [ ] 3.10 Green: add the `Origin` test of each post (D6). Verify that the 3.9 tests pass.

## 4. The loop (`web-control`)

- [ ] 4.1 Red: write a `run` test for `Command::AlertNames` with the built-in tables (`ok panic bedtime bed-flash`) and for "Page after a reload" (`ok dinner`). Write a `parse_command` test that the text `alert-names` and similar words are not a command. See them fail.
- [ ] 4.2 Green: add `Command::AlertNames` and its reply in the loop. The socket parser cannot make it. Verify that the 4.1 tests and all earlier tests pass.

## 5. Arguments, the listener, and start (`daemon-lifecycle`, `web-control`)

- [ ] 5.1 Red: write `parse_args` tests for "Default web address" and "Web address is not a driver flag". Also write "Web listener off" and "Bad web address is a usage error". See them fail.
- [ ] 5.2 Green: add `--http=<address>` and `--http=off` to `Args` (D7). Verify that the 5.1 tests pass.
- [ ] 5.3 Red: write `tests/cli.rs` tests with a real `TcpStream`. Write "Listen on a given address", "Off", "One response, then close", and "Address in use". Write "Slow client" (408 in less than 3 s), "Headers that do not end", "Body too large", and "Post with no length". Write "A slow client does not stop the clock": a held connection, and a `status` on the socket that gets its reply during the hold. Write a test that sends `POST /message` and then reads `status` on the socket with `messages=1`. Change the CLI tests that start the daemon to pass `--http=127.0.0.1:0` or `--http=off`. See them fail.
- [ ] 5.4 Green: add the listener thread with the total read limit and the write timeout (D2). Bind it in `main.rs` after the socket and before the panel, write the address line or the warning (D7). Verify that the 5.3 tests and all CLI tests pass in the default and the `simulator` builds.

## 6. Deployment (`pi-deployment`)

- [ ] 6.1 Red: add "Deploy stops the legacy web stack" and "Enable disables the legacy web stack" to `scripts/test-deploy.sh`. Add "Rollback starts the legacy web stack". Test the order of the commands. See them fail.
- [ ] 6.2 Green: change `scripts/deploy.sh` and its usage text (D8). Verify with `scripts/test-deploy.sh`.

## 7. Simulator verification

- [ ] 7.1 Run the `simulator` build with `--http=127.0.0.1:8080`. Use `curl` to post a message, the `panic` alert, `clear`, and `alert stop`. Verify the frames in the captured output and the status codes. Post a `msg` of 65 characters and a cross-site `Origin`, and verify 400 and 403. Record the results here.

## 8. On-device verification and measurements

- [ ] 8.1 Before the deploy, record the RSS of Node, PM2, and nginx, and `free -m`. Record the time of `GET /` and `POST /message` to the legacy page from a laptop, 5 runs each. Use a message that the legacy clock can show. Write the numbers in `openspec/changes/web-control/measurements.md`.
- [ ] 8.2 Build with `scripts/build-pi.sh`, and run `scripts/check-pi-binary.sh`. Record the binary size against `main`.
- [ ] 8.3 Deploy with `scripts/deploy.sh`. Run "Web page on port 80": `sudo ss -ltnp` and `GET /health` from the laptop. Record `free -m`.
- [ ] 8.4 Run "Page from a phone": ask the user to open the page on a phone, send a message, and start the panic alert. Ask the user to verify the panel.
- [ ] 8.5 Run "Idle clock with the listener": `scripts/measure.sh` for 60 s with the clock only. Record CPU and voluntary context switches for the main thread and the web thread, and RSS.
- [ ] 8.6 Run "Response time on the LAN": 5 runs each of `GET /` and `POST /message`. Record the medians next to the legacy numbers.
- [ ] 8.7 Run "Rollback gives back the legacy page": `scripts/deploy.sh --rollback`, then a message from the legacy page. Leave the legacy clock running unless the user asks for `--enable`.

## 9. Docs and final verification

- [ ] 9.1 Update `architecture.md` (the web thread, the event channel, and the start order) and `glossary.md` (web page, web listener, origin test). Update `roadmap.md` (mark `web-control` done) and `legacy-behavior.md` (nginx, PM2, and port 3000 on the Pi). Update `docs/hardware/pi-zero-w.md` (port 80), the README, and `CHANGELOG.md`. Verify with `scripts/lint-docs.sh`.
- [ ] 9.2 Run `cargo fmt --all --check`, `cargo clippy --all-targets -- -D warnings`, the clippy and test runs with `--features simulator`, `cargo test`, `cargo deny check`, `scripts/clippy-pi.sh`, `openspec validate --all --strict`, and `scripts/lint-docs.sh`. All must pass. Run `cargo fmt` before each commit.
- [ ] 9.3 Push the branch, open the PR, and verify that all CI jobs pass.

## Workflow follow-up

- After review, archive the change as the last commit on the PR (`/opsx:archive`), before the merge.
