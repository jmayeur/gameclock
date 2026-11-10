# CLAUDE.md

Rules for each person or AI agent that changes this repo. If a rule is not
true for the code, fix this file in the same change.

## Project

`gameclock` drives a 64×32 HUB75 LED matrix from a **Raspberry Pi Zero W**
(one ARMv6 core, 512 MB) through an Adafruit RGB Matrix HAT. It replaces a
Python and Node clock (`~/dev/gameclock`, not in this repo).

- **Performance on the single core is the primary goal.** The driver's refresh
  thread shares the core. CPU that the app uses causes flicker.
- Features arrive one capability at a time through OpenSpec.
- This is **not** a port. Each change specifies legacy behavior again.

## Rule 1: record decisions as ADRs

`docs/adr/` holds the Architecture Decision Records. `docs/adr/README.md` has
the process and the template.

- Write an ADR before or with a change that:
  - adds, removes, or replaces a dependency
  - adds or moves a boundary (crate, trait, process, or protocol)
  - assumes something about hardware or deployment
  - touches licensing
  - reverses an earlier decision
- ADRs are append-only history. To change a decision, write a new ADR. Mark
  the old ADR `Superseded by ADR-NNNN`.
- Do not rewrite an accepted ADR. Small clarifications and mechanical STE
  fixes are permitted (ADR-0011).
- An OpenSpec change links its ADRs. Each ADR links back.

## Rule 2: design docs describe the system as it is now

`docs/design/` holds `architecture.md`, `glossary.md`, `roadmap.md`, and
`legacy-behavior.md`.

- A change to structure, data flow, or responsibilities updates
  `architecture.md` in the same commit. New domain terms go in `glossary.md`.
- A doc that does not agree with the code is a bug, as urgent as a failing
  test.
- ADRs give reasons. Design docs give what is true now. Link the ADR instead
  of repeating its reasons.

## Rule 3: features go through OpenSpec

Do not write feature code without an OpenSpec change in
`openspec/changes/<name>/`.

1. **`/opsx:propose`**: write the proposal, spec deltas, design, and tasks.
   Agree on the behavior before code. For an unclear idea, use
   `/opsx:explore` first.
2. Write each ADR that the change needs, even if `design.md` also describes
   the decision.
3. **`/opsx:apply`**: implement red → green (Rule 4). Each scenario maps to a
   test. Update the design docs.
4. **`/opsx:archive`**: make the archive the last commit on the change's PR,
   after review and before merge. The code and the updated `openspec/specs/`
   then go into `main` together. Make all review changes before the archive.

- `openspec/config.yaml` repeats these rules for OpenSpec. Keep it in sync.
- Use a branch or worktree, and one PR, for each change.
- Refactors, dependency updates, and doc fixes need no change proposal.

## Rule 4: build test-first, red → green → refactor

This applies to all code: features, bug fixes, scripts, and build tools.

1. **Red.** Write the test. Run it. It must fail for the expected reason (an
   assertion or a missing item, not a typo).
2. **Green.** Write the least code that makes it pass. Run it.
3. **Refactor** while the tests pass. Then run all checks.

- Each spec scenario maps to a test that failed first. In a task list, the
  failing-test task comes before its implementation task.
- A bug fix starts with a test that shows the bug.
- FFI and hardware code also start red: a build-time assertion, a script
  test, or an on-device scenario.
- Commit each test with the code that makes it pass. Each commit passes all
  checks.

## Rule 5: write in Simplified Technical English

All text follows ASD-STE100 through the `asd-ste100` skill (ADR-0011).

- **Strict mode:** error messages, log lines, CLI output, code comments, and
  agent instructions (this file, `openspec/config.yaml`).
- **STE-flavored mode:** docs, ADRs, specs, OpenSpec artifacts, PR
  descriptions, commit messages, and agent replies.
- Sentences: 20 words or fewer for instructions, 25 for descriptions. Active
  voice. No semicolons or phrasal verbs. One name for one thing. Keep hedges.
- `scripts/lint-docs.sh` must pass. CI fails on a hard finding.

## Architecture invariants

An ADR is necessary to change one of these.

- **`gameclock-core` is pure.** No hardware, filesystem, network, threads, or
  `SystemTime::now()`. The daemon injects time and input. The core renders
  into a `Frame` (an `embedded-graphics` `DrawTarget`) and is testable on a
  laptop.
- **Panels only blit.** A `Panel` receives a finished `Frame` and a
  `Brightness`, and does no rendering. The hzeller driver, a simulator, and
  test doubles are all `Panel`s.
- **Render on change, not on tick.** The daemon sleeps until the image
  changes. Each animation has an explicit frame budget. No heap allocation for
  each frame on the hot path.
- **Unsafe code stays in the driver's FFI crate.** Each `unsafe` block has a
  `// SAFETY:` comment. All other crates deny `unsafe_code`.

## Performance

A performance claim needs a number: a benchmark or an on-device measurement
(CPU %, refresh rate, RSS), recorded in the OpenSpec change or the ADR. Device
numbers come from the Pi Zero W, not the dev machine.

## Licensing

The project is **GPL-2.0-or-later**, so it can link hzeller's GPL-2.0-only
driver (ADR-0003).

- Each dependency must be compatible with GPL-2.0. `deny.toml` enforces this.
- **Apache-2.0-only** and **GPL-3.0 or AGPL** crates are not compatible.
- `MIT OR Apache-2.0` is compatible. We use it under MIT.

## Commands

```bash
cargo fmt --all
cargo clippy --all-targets -- -D warnings          # pedantic lints are on
cargo test
cargo deny check                                    # licenses, advisories, sources
cargo check --target arm-unknown-linux-gnueabihf    # Pi Zero W target
openspec validate --all --strict                    # specs and changes
scripts/lint-docs.sh                                # STE (ADR-0011)
scripts/build-pi.sh                                 # Pi release build (ADR-0009)
```

`rust-toolchain.toml` pins the toolchain. `CONTRIBUTING.md` lists the Pi and
script tools.

## Conventions

- Use Rust 2024. Propagate errors. Do not use `unwrap` or `expect` on runtime
  paths outside tests.
- Make small commits with an imperative subject. Name the ADR or the OpenSpec
  change. Commit `Cargo.lock`.
- Work on branches. `main` must always pass all checks.
- Do not put secrets, personal data, or device addresses in the repo. The repo
  is public.
