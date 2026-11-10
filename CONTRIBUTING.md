# Contributing

The short version of [`CLAUDE.md`](CLAUDE.md), which is the full working
agreement.

1. **Features start as an OpenSpec change** in `openspec/changes/<name>/`. Agree
   on the behavior and scenarios before writing code. Install the CLI once with
   `npm i -g @fission-ai/openspec@1.17.0` (not the unscoped `openspec`
   package). In Claude Code, run `/opsx:propose`, then `/opsx:apply`, then
   `/opsx:archive`.
2. **Decisions get an ADR.** That covers dependencies, boundaries, hardware or
   deployment assumptions, licensing, and reversals. See
   [`docs/adr/`](docs/adr/).
3. **Design docs stay true.** If you change how something works, update
   [`docs/design/`](docs/design/) in the same commit.
4. **Tests and checks pass:**
   ```bash
   cargo fmt --all -- --check
   cargo clippy --all-targets -- -D warnings
   cargo test
   cargo deny check
   openspec validate --all --strict
   ```
5. **Performance claims come with numbers**, measured on the Pi Zero W for
   anything runtime-related.
6. **New dependencies must be GPL-2.0-compatible.** Apache-2.0-only and
   GPL-3.0/AGPL crates are rejected by `deny.toml`.

Keep commits small, with an imperative subject line. Reference the ADR or
OpenSpec change where relevant. By contributing, you agree your contribution is
licensed GPL-2.0-or-later.
