# CLAUDE.md

Working agreement for anyone (human or AI) changing this repo. If something here
stops matching reality, fix this file in the same change.

## Project in one paragraph

`gameclock` drives a 64×32 HUB75 RGB LED matrix from a **Raspberry Pi Zero W**
(single-core ARMv6, 512 MB) through an Adafruit RGB Matrix HAT. It is a Rust
successor to a Python + Node clock (legacy code: `~/dev/gameclock`, not in this
repo). **Performance on that single core is the primary goal**, because every
cycle the app burns is stolen from the driver's refresh thread and shows up as
flicker. Features are built incrementally through OpenSpec. This is **not** a
port: legacy behavior is reference material, re-specified one capability at a
time.

## Rule 1: decisions are recorded as ADRs

`docs/adr/` holds Architecture Decision Records. Process and template:
`docs/adr/README.md`.

- Write an ADR **before or with** any change that adds/removes/swaps a
  dependency, introduces or moves a boundary (crate, trait, process, protocol),
  assumes something about hardware or deployment, touches licensing, or
  reverses an earlier decision.
- ADRs are append-only history. Change a decision by writing a new ADR and
  marking the old one `Superseded by ADR-NNNN`; don't rewrite accepted ADRs
  beyond small clarifications.
- An OpenSpec change that makes an architectural choice links its ADR, and the
  ADR links back.

## Rule 2: design docs describe the system as it is now

`docs/design/` is the current map: `architecture.md`, `glossary.md`,
`roadmap.md`, and `legacy-behavior.md`.

- A change that alters structure, data flow, or responsibilities updates
  `architecture.md` in the same commit. New domain terms go in `glossary.md`.
- Docs that contradict the code are a bug, fixed with the same priority as
  failing tests.
- ADRs say *why*, at a point in time. Design docs say *what is true today*.
  Don't put rationale in design docs, link the ADR instead.

## Rule 3: features go through OpenSpec

No feature code without an OpenSpec change (`openspec/changes/<name>/`):

1. **`/opsx:propose`**: write the proposal, spec deltas, design, and tasks.
   Agree on behavior before any code. Use `/opsx:explore` first if the idea is
   still fuzzy.
2. Record any ADR the change needs. An ADR is required even if the decision is
   also described in the change's `design.md`.
3. **`/opsx:apply`**: implement test-first (every scenario maps to a test) and
   update the design docs.
4. **`/opsx:archive`** once merged, so `openspec/specs/` stays the source of
   truth.

`openspec/config.yaml` builds these rules into every proposal and task list.
Keep it in sync with this file. Do feature work on a branch or worktree, one
change per PR.

Refactors, dependency bumps, and doc fixes don't need a change proposal.

## Architecture invariants (changing these needs an ADR)

- **`gameclock-core` is pure.** No hardware, filesystem, network, threads, or
  `SystemTime::now()`. Time and input are injected. It renders into a `Frame`
  (an `embedded-graphics` `DrawTarget`) and is fully testable on a laptop.
- **Panels only blit.** A `Panel` takes a finished `Frame` + `Brightness`. It
  holds no display logic. The hzeller driver, a simulator, and test doubles are
  all `Panel`s.
- **Render on change, not on tick.** The daemon sleeps until something visible
  changes. Animations get an explicit frame budget. No per-frame heap
  allocation on the hot path.
- **Unsafe code is confined** to the FFI crate for the driver, documented with
  `// SAFETY:` comments. Every other crate denies `unsafe_code`.

## Performance discipline

Performance claims need numbers: a benchmark, or an on-device measurement (CPU
%, refresh rate, RSS) recorded in the relevant OpenSpec change or ADR. "Should
be faster" isn't evidence. Device numbers come from the Pi Zero W, not the dev
machine.

## Licensing

The project is **GPL-2.0-or-later** so it can link hzeller's GPL-2.0-only
driver (ADR-0003). Every dependency must be GPL-2-compatible; `deny.toml`
enforces this. Watch for **Apache-2.0-only** and **GPL-3.0/AGPL** crates: both
are incompatible. `MIT OR Apache-2.0` is fine (we use it under MIT).

## Commands

```bash
cargo fmt --all
cargo clippy --all-targets -- -D warnings     # pedantic lints are on
cargo test
cargo deny check                               # licenses, advisories, sources
cargo check --target arm-unknown-linux-gnueabihf   # Pi Zero W target
openspec validate --all --strict               # specs and changes
```

The toolchain is pinned in `rust-toolchain.toml` (rustup installs it
automatically).

## Conventions

- Rust 2024 edition. Errors are propagated; no `unwrap`/`expect` on runtime
  paths outside tests.
- Small, focused commits with an imperative subject. Reference the ADR or
  OpenSpec change. `Cargo.lock` is committed.
- Work on branches; `main` stays green.
- Don't put secrets, personal data, or device addresses in the repo. It is
  public.
