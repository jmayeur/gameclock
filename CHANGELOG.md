# Changelog

Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/). Versioning:
[SemVer](https://semver.org/).

## [Unreleased]

### Added

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
