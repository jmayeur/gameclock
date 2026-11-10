# Changelog

Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/). Versioning:
[SemVer](https://semver.org/).

## [Unreleased]

### Added

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
