# ADR-0008: Drive features with OpenSpec

- **Status:** Accepted
- **Date:** 2026-11-09

## Context

This isn't a port. Features from the legacy clock (and new ones) should be
agreed on, built, and verified one at a time, with AI assistance. The spec and
the code must not drift apart.

## Decision

Use [OpenSpec](https://github.com/Fission-AI/OpenSpec) (MIT, by Fission-AI):

- Each feature starts as a change in `openspec/changes/<name>/`: a proposal,
  tasks, and spec deltas with concrete scenarios.
- Scenarios become tests during implementation.
- A shipped change is archived, merging its deltas into `openspec/specs/`, which
  becomes the current behavioral spec.
- Architectural choices made inside a change are recorded as ADRs (ADR-0001).

OpenSpec is a development tool. It is not a runtime or build dependency, so it
has no effect on the license policy.

### Tooling details (clarified 2026-11-09)

- The package is **`@fission-ai/openspec`**, pinned to **1.17.0**. The unscoped
  `openspec` npm package is an unrelated placeholder. Don't use it.
- Install the CLI globally (`npm i -g @fission-ai/openspec@1.17.0`). The Claude
  Code skills call a bare `openspec` command. CI runs the same pinned version
  with `npx` to execute `openspec validate --all --strict`.
- `openspec init --tools claude` generated `.claude/skills/openspec-*` and
  `.claude/commands/opsx/*`. Both are committed, so everyone gets the same
  `/opsx:propose`, `/opsx:apply`, and `/opsx:archive` workflows.
- `openspec/config.yaml` carries this project's context and per-artifact rules:
  ADR triggers, design-doc and CHANGELOG tasks, the on-device measurement
  requirement, and the license constraints. Proposals and task lists produced
  by the tool therefore follow `CLAUDE.md` by default.
- Upgrading means bumping the version in `CONTRIBUTING.md`, CI, and this ADR,
  then running `openspec update` and reviewing the regenerated `.claude/` files
  in the same PR.

## Consequences

- Behavior is written down before code and stays current after.
- `docs/design/legacy-behavior.md` is the reference input for proposals, not a
  spec in its own right.
- There's some process overhead per feature. Trivial fixes skip it (see
  `CLAUDE.md`).

## Alternatives considered

- **GitHub Spec Kit.** Heavier, with phase gates and Python setup.
- **Issues plus ad-hoc docs.** Specs and code drift apart.
