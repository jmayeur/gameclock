# Design

## Context

See `proposal.md` for the reasons. `gameclockd` selects its panel at build
time in `main.rs` (ADR-0009). With `hzeller` it uses `HzellerPanel`.
Otherwise it uses `LogPanel` from `lib.rs`. The daemon presents once, then
blocks on a signal. stdout has no output now. stderr has the notices and the
`LogPanel` lines. The CLI tests in `tests/cli.rs` read stderr.

## Goals / Non-Goals

**Goals:**

- See frames on a dev machine with no new dependency.
- Unit tests for each scenario, with the output in memory.
- A clean terminal after Ctrl-C or SIGTERM.

**Non-Goals:**

- Fast output. The simulator is for development only. It can allocate in
  `present`, but it reuses one buffer.
- Support for terminals without 24-bit color.

## Decisions

### D1. `TerminalPanel<W: Write>` in a `sim` module of `gameclockd`

The panel is generic over its output. `main` gives it a locked stdout. Tests
give it a `Vec<u8>` or a writer that always fails. A new crate is not
necessary: the panel has no dependencies, and only the daemon uses it.

Alternative: a `gameclock-sim` crate. Rejected: one more crate for about 100
lines, and a new boundary that needs an ADR.

### D2. Upper half block with 24-bit colors

Each cell is `ESC[38;2;R;G;Bm` (upper pixel), `ESC[48;2;R;G;Bm` (lower
pixel), and `▀`. Each row ends with `ESC[0m` and a newline, so a narrow
terminal does not paint color past the row. A 64×32 frame uses 64 columns
and 16 lines. The cells are about square, like the LEDs.

Alternative: two space characters for each pixel, with a background color.
Rejected: 128 columns and 32 lines are too large for many terminals.

### D3. Escape codes

| Code | Use |
|---|---|
| `ESC[2J` | Clear the screen, once, on the first present |
| `ESC[?25l` | Hide the cursor, once, on the first present |
| `ESC[H` | Cursor home, at the start of each present |
| `ESC[0m` | Reset the colors, at the end of each row and on drop |
| `ESC[?25h` | Show the cursor, on drop |
| `ESC[{n};1H` | Move to line n (one below the frame), on drop |

The setup codes go out on the first present, not in `new`, because `new`
cannot return a write error. The scenario "Setup happens once" still holds:
the codes come before the first frame.

### D4. Brightness

Each channel becomes `channel * percent / 100`, in integer math, rounded down.
This copies the visible effect of `led_matrix_set_brightness`. It is not
hzeller's exact curve.

### D5. Errors

`TerminalPanel` has `type Error = std::io::Error`. A failed write returns the
`io::Error` unchanged. `run` then wraps it in `RunError::Present`, so the
daemon exits 1 with the message (spec `daemon-lifecycle`).

### D6. Feature and panel choice

- `gameclockd/Cargo.toml`: `simulator = []`. No dependencies.
- `main.rs` has three `panel` modules, one for each build: `hzeller`,
  `simulator`, and neither.
- `main.rs` has a `compile_error!` when both `hzeller` and `simulator` are on.
- The simulator build writes this notice to stderr: "gameclockd: no LED driver
  is built in. Frames are drawn in the terminal." Like the `LogPanel` build,
  it ignores `--led-*` flags with a notice.
- `.cargo/config.toml` adds `[alias] sim = "run -p gameclockd --features
  simulator"`.

### D7. Drop restores the terminal

`Drop` writes reset, show-cursor, and a move to the line below the frame,
then flushes. It ignores write errors, because `Drop` cannot return them.
`main` drops the panel before it exits (ADR-0010), so SIGTERM and Ctrl-C also
restore the terminal.

### D8. Tests

- Unit tests in `sim.rs` cover each `terminal-simulator` scenario. They parse
  the output in memory.
- A CLI test in `tests/cli.rs` covers "Simulator build draws in the terminal".
  It runs only with `--features simulator`. The existing CLI tests run only
  without features.
- A script test covers "Driver and simulator together do not build". It runs
  `cargo-zigbuild check` for the Pi target with both features. It expects a
  failure and the message. A host check cannot test this: the driver's build
  script compiles C++ first and fails on the host before the
  `compile_error!`. So the test needs zig and runs in the CI `pi-build` job.

## Risks / Trade-offs

- [A small terminal wraps the rows and the image breaks] → The manual
  scenario names the minimum size. A size check is out of scope.
- [A terminal without 24-bit color shows wrong colors] → Out of scope. Most
  current terminals support 24-bit color.
- [SIGKILL leaves the cursor hidden] → `reset` or `tput cnorm` restores it.
  CONTRIBUTING.md will say this.
- [The two CLI test sets need different features] → CI runs `cargo test`
  once without features and once with `--features simulator`.

## Migration Plan

None. The feature is new and off by default. The Pi build does not change.
