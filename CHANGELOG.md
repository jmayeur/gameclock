# Changelog

Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/). Versioning:
[SemVer](https://semver.org/).

## [Unreleased]

### Added

- Data pages (OpenSpec `sensor-pages`, ADR-0021):
  - `gameclock-core`: the `page` module. `PageName`, `Reading`,
    `Forecast`, `PageData`, the US EPA AQI from PM2.5 and PM10 with its
    category colors, and `PageScene`, which shows for 30 s. The board shows
    a page above a message and below `bedtime`.
  - `gameclockd`: the `page <name>` command for `weather`, `indoor`, and
    `outdoor`. The `fetch` thread runs the system `curl` for each request.
    The `pages` module parses the JSON with `serde_json`. Only the newest
    request shows. Bad or old data shows dashes and a warning.
  - A `[pages]` config section with the URL of each page. There are no
    built-in URLs.
  - The web page has a button for each page with a URL, which posts to
    `/page`.
  - New dependency `serde_json`.

- Web page (OpenSpec `web-control`, ADR-0020):
  - `gameclockd`: the `http` module (a small HTTP/1.1 parser with limits)
    and the `web` module (the routes, the forms, the page, and the `web`
    thread). `GET /`, `POST /message`, `POST /alert`, `POST /alert/stop`,
    `POST /clear`, and `GET /health`. The legacy values `panic88` and
    `bedtime` start their alerts.
  - `--http=<address>` and `--http=off`. The default is `0.0.0.0:80`.
  - A post from a different site gets 403. No CORS headers.
  - `scripts/deploy.sh` stops nginx and PM2. `--enable` disables them at
    boot, and `--rollback` starts them again.

- Config file (OpenSpec `configurable-schedule`, ADR-0019):
  - `gameclockd`: the `config` module reads `/etc/gameclockd/config.toml`,
    or the path of `--config=<path>`, with `serde` and `toml`. The sections
    `[schedule]`, `[[alert]]`, and `[[trigger]]` are optional. A missing
    section uses its built-in table. Errors give the path, the line, and
    the column.
  - The `reload` command reads the file again. An error keeps the tables.
  - A file with an error at start exits with status 2.
  - `gameclock-core`: `Tables`, `Alerts` (owned alerts with checks),
    `Alert::validate`, `Trigger::new`, and `Board::set_schedule`.
  - `deploy/config.toml` with the legacy tables, and `docs/config.md`.

- Alerts (OpenSpec `alerts`, ADR-0018):
  - `gameclock-core`: the `effect` module (color wheel, contrast color). The
    `alert` module (the alert definition, the `BUILT_IN` table, the
    `AlertScene`, and the timed triggers). The `board` module (queue,
    priorities, restart of an interrupted message, and clear rules).
    `ScrollPath` is the shared scroll math. `Queue::put_back`.
  - The built-in alerts `panic`, `bedtime`, and `bed-flash`. On bedtime
    nights, the daemon starts `bed-flash` each 10 s from 23:00 to 23:05, and
    `bedtime` each 10 s from 23:05 to 23:16.
  - `gameclockd`: the commands `alert <name>` and `alert stop`. `status`
    gives `alert=<name>` or `alert=none`.
- Message board (OpenSpec `message-board`, ADR-0017):
  - `gameclock-core`: the `message` module with `Message`, the `Scroll`
    scene, and the `Queue`. A message scrolls 1 pixel each 40 ms, from the
    injected time, at the brightness of the clock schedule.
    `Compositor::remove_finished` and `clear_foreground`.
  - `gameclockd`: the commands `message [--passes=N] [--color=RRGGBB]
    <text>` and `clear`. At most 8 messages wait. `status` gives
    `messages=<n>`. `gameclockd ctl` joins its command words with one space.
- Command input (OpenSpec `command-input`, ADR-0016):
  - `gameclockd`: a root-only control socket (`/run/gameclockd/control.sock`,
    `--control-socket=<path>`) with one command line for each connection.
    The commands are `clock`, `test-pattern`, and `status`.
  - `gameclockd ctl <command>`: the client in the same binary.
  - `gameclock-core`: `Compositor::set_base` and `last_brightness`.
- Clock scene (OpenSpec `clock-scene`, ADR-0014, ADR-0015):
  - `gameclock-core`: the `Clock` scene (`09:05PM` in a 7×13 bold font) and
    the time-of-day `Schedule` as data, with the legacy table as the default.
    `Scene::brightness`, and the compositor gives the brightness with the
    frame.
  - `gameclockd`: shows the clock as the base scene, at the scene's
    brightness. `--test-pattern` shows the heartbeat.
- Scene model (OpenSpec `scene-model`, ADR-0012, ADR-0013):
  - `gameclock-core`: the `Scene` trait, the `Compositor`, and the
    `Heartbeat` scene. A test checks that updates allocate no heap memory.
  - `gameclockd`: the event-driven loop. It waits on the signal pipe with a
    timeout until the next scene change. It reads the system time zone once
    at startup, with a UTC fallback.
  - New dependency `jiff`. New dev-dependency `allocation-counter`.
- Terminal simulator (OpenSpec `dev-simulator`): `TerminalPanel` draws
  frames in a 24-bit color terminal. Run it with `cargo sim`. The `simulator`
  feature selects it. It cannot be used with `hzeller`.
- Simplified Technical English for all text (ADR-0011, CLAUDE.md Rule 5).
  The `asd-ste100` skill is committed in `.claude/skills/`.
  `scripts/lint-docs.sh` checks all tracked Markdown, and CI fails on a hard
  finding.
- Panel bring-up (OpenSpec `panel-bring-up`, ADR-0009, ADR-0010):
  - `gameclock-core`: the diagnostic `test_pattern`.
  - `rgbmatrix-sys`: hzeller's driver as a pinned submodule (`3da7a3b`),
    built with zig behind the `link` feature. Hand-written FFI with a
    build-time struct layout check, and `HzellerPanel`.
  - `gameclockd`: shows the test pattern once, then blocks until SIGTERM or
    SIGINT. It accepts only `--led-*` driver flags. The `hzeller` feature
    selects the real panel.
  - `deploy/gameclockd.service` and scripts to build, check, lint, deploy,
    roll back, and measure on the Pi, each with tests.
  - CI jobs for the Pi build and the scripts.
  - On-device baseline measurements of the legacy clock and `gameclockd`.
- CLAUDE.md Rule 4: test-first, red → green → refactor.

### Changed

- **The `status` reply ends with `page=<name>` or `page=none`.** The
  unknown-command reply names eight commands. Before a rollback to an older
  `gameclockd`, remove the `[pages]` section from the config. An older
  daemon refuses it and does not start (OpenSpec `sensor-pages`).
- Alerts are owned data in an `Alerts` table, shared with `Rc`. `BUILT_IN`
  and `alert::find` are gone. The loop looks up alert names, so the alert
  error replies list the current table. The unknown-command reply names
  seven commands (OpenSpec `configurable-schedule`).
- The service does not restart after exit status 2
  (`RestartPreventExitStatus=2`). The deploy installs `deploy/config.toml`
  only if no config exists.

- The `status` reply ends with `alert=<name>`. The unknown-command reply
  names six commands. `clear` keeps an alert whose clear rule protects it.
  The daemon also wakes at the next trigger time, so a static scene no
  longer waits with no timeout (OpenSpec `alerts`).
- The `status` reply ends with `messages=<n>`. The unknown-command reply
  names five commands. `gameclockd ctl` with more than one word is no
  longer a usage error (OpenSpec `message-board`).
- The loop waits on an event channel. A signal thread and a control thread
  send events to it. This replaces the signal-pipe read (ADR-0016).
- The service unit makes `/run/gameclockd` for the socket.
- `gameclockd` shows the clock, not the heartbeat. It presents once each
  minute (OpenSpec `clock-scene`).
- The built-in GPIO mapping is `adafruit-hat-pwm`, not `adafruit-hat`. The
  service unit passes `--led-pwm-bits=8`. This removes the moving white lines
  and raises the refresh rate from 147 Hz to 176 Hz (ADR-0015).
- `gameclockd` no longer presents the test pattern only once. It shows the
  heartbeat and presents once each second (OpenSpec `scene-model`).
- Docs, specs, code comments, and messages rewritten in STE. Accepted ADRs got
  mechanical fixes only. Error and notice text in `gameclockd` and
  `rgbmatrix-sys` changed (for example, "unknown argument" replaces
  "unrecognized argument").
- OpenSpec changes are archived on their PR, before merge (CLAUDE.md Rule 3).

- Cargo workspace (Rust 2024, toolchain pinned to 1.99.1, ARMv6 target).
- `gameclock-core`: `Frame` (an `embedded-graphics` `DrawTarget`), the `Panel`
  trait, and `Brightness`, with unit tests.
- `gameclockd` daemon stub.
- GPL-2.0-or-later license and a `cargo-deny` policy for GPL-2-compatible
  dependencies.
- ADRs 0001–0008, design docs (architecture, glossary, roadmap, legacy
  behavior), and Pi Zero W hardware notes.
- CI: fmt, clippy (pedantic), tests, cargo-deny, ARMv6 type-check. Dependabot
  for cargo and actions.
- OpenSpec 1.17.0 (`@fission-ai/openspec`), set up for Claude Code: `/opsx:*`
  commands and skills, plus a project `openspec/config.yaml` that encodes the
  ADR, design-doc, performance, and license rules. CI runs
  `openspec validate --all --strict`.
