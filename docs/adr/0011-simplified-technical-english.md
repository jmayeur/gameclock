# ADR-0011: Write in Simplified Technical English

- **Status:** Accepted
- **Date:** 2026-11-09
- **Related:** ADR-0001, ADR-0008

## Context

People and AI agents both read and write this repo. The docs, specs, and agent
output were too long and sometimes easy to misread. An agent that misreads an
instruction or a spec makes the wrong change.

ASD-STE100 (Simplified Technical English) is a controlled language from the
aerospace industry. It removes words with more than one meaning and sentences
with more than one structure. The `asd-ste100` skill
(github.com/danyuchn/asd-ste100-skill, MIT) applies its writing rules. It
includes `ste-lint.py`, a linter for the rules that a script can check. The
skill does not include ASD's approved dictionary, because ASD does not permit
redistribution. So the word-choice rules are a direction, not a check.

## Decision

- All text in this repo follows STE. There are two modes:
  - **Strict:** error messages, log lines, CLI output, code comments, and agent
    instructions (`CLAUDE.md`, `openspec/config.yaml`).
  - **STE-flavored:** docs, ADRs, specs, OpenSpec artifacts, PR descriptions,
    and commit messages. The structural rules apply in full. The word-choice
    rules are advisory.
- The AI agent writes its own output (replies, status reports) in STE-flavored
  mode.
- We commit the skill in `.claude/skills/asd-ste100/` and pin it in
  `skills-lock.json`.
- `scripts/lint-docs.sh` runs the linter on all tracked Markdown, except
  vendored code, the skill, and archived OpenSpec changes. CI fails on any hard
  finding.
- Accepted ADRs get mechanical lint fixes only (for example, a semicolon
  becomes two sentences). Their decisions and wording stay as written
  (ADR-0001).

## Consequences

- Text is shorter, and one sentence has one reading.
- New docs, specs, and ADRs must pass the linter before merge.
- The linter checks structure only. Plain words and short paragraphs still need
  review.
- Some prose reads flatter. We accept this for docs. Creative text is not in
  scope.
- The skill is third-party code that runs with the agent's permissions. We
  review it when we change the pinned version.

## Alternatives considered

- **A general style guide with no linter.** It cannot be enforced, so the
  docs drift.
- **Vale or another prose linter.** It needs a custom STE rule set, which we
  would have to write and maintain.
- **A baseline that permits current findings.** It keeps old problems. The
  number of findings was small, so we fixed them.
