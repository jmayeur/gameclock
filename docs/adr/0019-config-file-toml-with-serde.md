# ADR-0019: The config is a TOML file that serde reads in the daemon

- **Status:** Accepted
- **Date:** 2026-11-15
- **Related:** ADR-0003, ADR-0005, ADR-0010, ADR-0014, ADR-0016, ADR-0018.
  OpenSpec change `configurable-schedule`.

## Context

The clock schedule, the alert definitions, and the bedtime trigger rules are
tables in the code (ADR-0014, ADR-0018). A person who wants a different
bedtime hour must edit Rust, build, and deploy. The legacy admin page stored
similar settings as JSON, but its renderer side was never finished.

The core is pure (ADR-0005). It has no file I/O, and its only dependencies
are `embedded-graphics` and `jiff`. The daemon runs as a systemd service that
restarts after a failure (ADR-0010). On the Pi, the LED driver drops root
privileges after it sets up the GPIO.

## Decision

- **The config is one TOML file.** The default path is
  `/etc/gameclockd/config.toml`. `--config=<path>` gives a different path.
  It has three optional sections: the schedule, the alerts, and the
  triggers. A section in the file replaces the whole built-in table of that
  section. A missing section uses its built-in table.
- **serde and `toml` read it, in the daemon only.** `gameclockd` gets
  `serde` (with `derive`) and `toml` (with only `std`, `serde`, and
  `parse`). Both are `MIT OR Apache-2.0`, which we use under MIT
  (ADR-0003). The core gets no new dependency. Its types keep the range
  checks, and the daemon converts the file into them.
- **Errors give a place.** Each error gives the path, the line, the column,
  and the problem.
- **A bad file stops the start.** The daemon exits with status 2 before it
  touches the panel. The service does not restart after exit status 2. A
  missing file at the default path is not an error: the built-in tables
  apply.
- **`reload` reads the file again.** A correct file applies at once. A file
  with an error gives an error reply, and the tables do not change. The file
  must be readable by all users, so a reload works after the driver drops
  root privileges.

## Consequences

- A person changes the schedule, the alerts, or the bedtime warning with an
  edit and `reload`, with no build.
- The Pi binary gets bigger. The parse runs only at start and on `reload`,
  never in the render loop.
- Alerts become owned data, not `'static` data. The alert name is looked up
  in the loop, because the table can change.
- After a reboot, a file with an error leaves the panel dark until a person
  corrects it. A `reload` after each edit shows the error first.
- `web-control` can later write the same file and send `reload`.

## Alternatives considered

- **A small format of our own, with no dependency.** It needs its own parser,
  tests, and error positions, for a format that only this project uses.
- **JSON, as the legacy admin page used.** It has no comments, and a person
  edits this file by hand.
- **serde in the core.** The core then depends on a parse library for a file
  that only the daemon reads.
- **Warn and use the built-in tables when the file has an error.** The clock
  then always runs, but a typo can go unnoticed for a long time.
- **A file watcher instead of `reload`.** It needs a thread or inotify, and a
  half-written file can apply.
