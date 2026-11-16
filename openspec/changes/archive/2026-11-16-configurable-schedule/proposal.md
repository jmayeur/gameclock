# Proposal

## Why

The clock schedule, the alert definitions, and the bedtime trigger rules are
built-in tables in the code (ADR-0014, ADR-0018). To change a bedtime hour or
a color, a person must edit Rust, build, and deploy. The legacy clock had an
admin page for these settings, but it was never finished. This change puts
the same data in a config file on the Pi.

## What Changes

- Add a **config file** in TOML. The default path is
  `/etc/gameclockd/config.toml`. `--config=<path>` gives a different path.
  The file has three optional sections:
  - `[schedule]` and `[[schedule.rule]]`: the color and brightness rules, by
    weekday and time of day, and the default style
  - `[[alert]]`: the alert definitions
  - `[[trigger]]`: the timed trigger rules
- A **missing section** uses its built-in legacy table. A section that is in
  the file replaces the whole built-in table of that section.
- A **missing file** at the default path uses the built-in tables, and the
  daemon writes a line that says so. A missing file at a path that
  `--config` gives is an error.
- A **file with errors** stops the start. The daemon exits with status 2
  before it touches the panel. The message names the file, the line, the
  column, and the problem. The service unit does not restart the daemon
  after exit status 2.
- The daemon checks each value: the times, the weekdays, the colors, the
  brightness, the alert names, the text limits, and the durations. Each
  trigger must name an alert in the file or in the built-in table.
- Add the **`reload` command**. It reads the file again. If the file is
  correct, the new tables apply at once. The clock gets the new schedule, and
  new alerts and messages get the new brightness. The triggers start again
  from the current time. An alert on the panel continues to its end.
  If the file has errors, the reply is the error, and the daemon keeps its
  tables.
- The `alert` command uses the **current alert table**. The error replies
  list the names from that table.
- `deploy/config.toml` is an example file with the legacy tables.
  `scripts/deploy.sh` installs it only if `/etc/gameclockd/config.toml` does
  not exist, so it does not overwrite the edits of a person.
- **BREAKING** (protocol): the unknown-command reply names seven commands.

## Capabilities

### New Capabilities

- `daemon-config`: the config file, its path, its sections and values, the
  checks and error messages, the start behavior, and the `reload` command.

### Modified Capabilities

- `clock-scene`: the schedule comes from the config file. The legacy
  schedule is the default when the file has no `[schedule]` section.
- `alerts`: the definitions and the trigger rules come from the config file.
  The built-in tables are the defaults. The `alert` error replies list the
  names of the current table.
- `daemon-lifecycle`: the `--config=<path>` argument.
- `command-input`: the unknown-command reply lists `reload`.
- `pi-deployment`: the service does not restart after exit status 2, and the
  deploy installs the example config if no config exists.

## ADRs

- Uses ADR-0003 (license), ADR-0005 (pure core), ADR-0010 (systemd service),
  ADR-0014 (schedule as data), ADR-0016 (command socket), and ADR-0018
  (alerts as data).
- **New ADR-0019: The config is a TOML file that serde reads in the daemon.**
  It adds the dependencies `toml` and `serde`. It decides the file path, and
  that the core stays free of the file and of serde. It also decides the
  start and reload behavior, and that a bad file stops the start.

New dependencies, both `MIT OR Apache-2.0` (used under MIT, so compatible
with GPL-2.0-or-later):

- `serde` with `derive`
- `toml` with only `std`, `serde`, and `parse` (no `display`), and the crates
  that it needs (`toml_parser`, `toml_datetime`, `serde_spanned`, `winnow`)

`cargo deny check` must pass.

## Expected effect on the Pi

- The file is read at start and on `reload`, never in the render loop. The
  loop CPU, the wakes, and the refresh rate do not change.
- The Pi binary gets bigger. We expect about 100 to 300 KB for the TOML
  parser and the serde code. RSS can grow a little at start.
- **Measurement:**
  - the binary size against `main`
  - the RSS of the service with the example config, against the last
    measurement (2,850 KiB)
  - the time of a `reload` reply with the example config
  - `scripts/measure.sh` for 60 s with the clock only, to show that the idle
    loop does not change (main thread 0.0%, 2 switches)

## Out of scope

- An HTTP or web form to edit the config (`web-control`).
- A file watcher. The person sends `reload` after an edit.
- The panel size, the driver flags, and the socket path in the config file.
  They stay arguments.
- Named colors. A color is six hexadecimal digits.
- A `status` field for the config source.
- New fonts. An alert font is `small` or `large`, as now.

## Impact

- `gameclock-core`: alerts become owned data (`String` names and texts),
  not `&'static`. An alert table type replaces `BUILT_IN` and `find`.
  Checked constructors give errors for bad values. The board and the
  triggers hold shared alerts (`Rc`).
- `gameclockd`: a `config` module with the serde types, the parse, and the
  conversion to core types. `--config`, the start check, the `reload`
  command, and the alert lookup in the loop.
- `deploy/`: `config.toml`, and `RestartPreventExitStatus=2` in the unit.
  `scripts/deploy.sh` and its test.
- `Cargo.toml`, `Cargo.lock`, and `deny.toml` if a license must be added.
- Docs: ADR-0019, the ADR index, `architecture.md`, `glossary.md`,
  `roadmap.md`, `CHANGELOG.md`, the README, and a config section in the
  README or a new `docs/config.md`.
