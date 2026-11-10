# ADR-0001: Record decisions as ADRs; keep design docs current

- **Status:** Accepted
- **Date:** 2026-11-09

## Context

The legacy clock grew as one loop in which Clock, Message Board, and Panic were
intertwined, and nothing recorded why things were the way they were. This
project is meant to be run formally and to evolve feature by feature, so the
reasoning behind choices has to survive past the commit that made them.

## Decision

- Significant decisions are recorded as numbered, append-only ADRs in
  `docs/adr/`, using `template.md`.
- `docs/design/` describes the current system and is updated in the same change
  that alters it.
- `CLAUDE.md` makes both rules part of the working agreement, so they apply to
  AI-assisted changes too.

## Consequences

- Future contributors can see both what was chosen and what was rejected.
- Reversals are explicit (a superseding ADR) rather than silent drift.
- Each significant change carries some writing overhead. We accept that.

## Alternatives considered

- **Git history only.** Diffs show what changed, not why, and rejected options
  leave no trace.
- **One big ARCHITECTURE.md.** Good for the current state (that's
  `docs/design/`), but it gets rewritten and loses history.
