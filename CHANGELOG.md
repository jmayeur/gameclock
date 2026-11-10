# Changelog

Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/). Versioning:
[SemVer](https://semver.org/).

## [Unreleased]

### Added

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
