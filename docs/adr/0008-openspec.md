# ADR-0008: Drive features with OpenSpec

- **Status:** Accepted
- **Date:** 2026-11-09

## Context

This isn't a port. Features from the legacy clock (and new ones) should be
agreed on, built, and verified one at a time, with AI assistance, without the
spec and the code drifting apart.

## Decision

Use [OpenSpec](https://github.com/Fission-AI/OpenSpec) (MIT, by Fission-AI):

- Each feature starts as a change in `openspec/changes/<name>/`: a proposal,
  tasks, and spec deltas with concrete scenarios.
- Scenarios become tests during implementation.
- A shipped change is archived, merging its deltas into `openspec/specs/`, which
  becomes the current behavioral spec.
- Architectural choices made inside a change are recorded as ADRs (ADR-0001).

OpenSpec is a development tool run with `npx`. It is not a runtime or build
dependency, so it has no effect on the license policy.

## Consequences

- Behavior is written down before code and stays current after.
- `docs/design/legacy-behavior.md` is the reference input for proposals, not a
  spec in its own right.
- There's some process overhead per feature. Trivial fixes skip it (see
  `CLAUDE.md`).

## Alternatives considered

- **GitHub Spec Kit.** Heavier, with phase gates and Python setup.
- **Issues plus ad-hoc docs.** Specs and code drift apart.
