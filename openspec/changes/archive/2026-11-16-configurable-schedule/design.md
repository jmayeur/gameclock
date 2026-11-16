# Design

## Context

- `Schedule` is data: `Rule`s of `Weekdays`, `TimeOfDay`, and a `Style`, and a
  default style. `Schedule::legacy()` builds the table. `Rule::new` and
  `TimeOfDay::hm` check their values and give `ScheduleError` (ADR-0014).
- `Alert` is `'static` data: `name` and `text` are `&'static str`.
  `alert::BUILT_IN` is a `const` slice, and `alert::find` looks up a name.
  `AlertScene`, `Board`, `Trigger`, and `Command::Alert` all hold
  `&'static Alert` (ADR-0018).
- `Trigger::legacy()` builds the two bedtime rules. `Triggers` keeps the next
  fire time of each rule.
- `run` makes its board with `Schedule::legacy()` and its triggers with
  `Trigger::legacy()`. `Mode::base_scene` makes the clock with
  `Schedule::legacy()`.
- `parse_command` runs in the control thread. A parse error gets its reply
  there, with no access to the loop state (ADR-0016). Today the unknown-alert
  reply comes from the parser and lists `BUILT_IN`.
- On the Pi, hzeller drops root privileges to the `daemon` user after the
  GPIO set-up. A `reload` then reads the file as that user.
- The service unit has `Restart=on-failure` and `RestartSec=2`.

## Goals / Non-Goals

**Goals:**

- One config file for the schedule, the alerts, and the triggers. Each
  section is optional, and the built-in tables are the defaults.
- Error messages with the path, the line, and the column.
- The core stays pure and free of serde. The core types do the range checks.
- `reload` with no restart, and no change to the tables when the file has an
  error.
- No change to the render loop or its allocations.

**Non-Goals:**

- A partial merge of a section with its built-in table.
- A file watcher, or a config from the command socket.
- Config for the driver, the socket, or the panel size.

## Decisions

### 1. TOML, read with serde in the daemon (ADR-0019)

`gameclockd` gets a `config` module. It has serde types that mirror the file,
with `#[serde(deny_unknown_fields)]`. Values that need a check after the
parse are `toml::Spanned<T>`, so that each error has a byte range. `toml` uses
only the features `std`, `serde`, and `parse`.

`config::parse(path: &str, text: &str) -> Result<Tables, ConfigError>`:

1. `toml::from_str` into the serde types. A syntax error, an unknown key, or
   a wrong type gives a `toml::de::Error` with a span.
2. Convert each section into core types. A core check that fails gives its
   message, at the span of the value.

`ConfigError { path, line, column, message }`. Its `Display` is
`<path>:<line>:<column>: <message>`. The line and the column come from the
byte offset. The line is 1 more than the count of line feeds before it. The
column is 1 more than the count of characters after the last line feed.

`config::load(path, origin) -> Result<Tables, ConfigError>` reads the file
and calls `parse`. `origin` is `Default` or `Argument`. A missing file at the
default path gives `Tables::legacy()` and a note for the log. A missing file
from `--config` is an error. Other read errors are errors.

Alternative: serde in the core. The core then has a parse dependency, and its
types have serde attributes for a file format that only the daemon reads.

### 2. The file format

```toml
[schedule]
default = { color = "ff0000", brightness = 25 }

[[schedule.rule]]
days = ["sun", "mon", "tue", "wed", "thu"]   # optional: each day
from = "23:00"
to = "23:15"
color = "ff8c00"
brightness = 100

[[alert]]
name = "panic"
text = "PANIC!!"
background = "color-wheel"   # or "black"
font = "small"               # or "large"
baseline = 20
color = "contrast"           # or six hexadecimal digits
x = 9                        # a still text. A scrolling text has passes.
seconds = 20
priority = 40
clear = "removable"          # or "kept"

[[trigger]]
alert = "bed-flash"
days = ["sun", "mon", "tue", "wed", "thu"]
from = "23:00"
to = "23:05"
every = 10                   # seconds
```

The limits:

| Value | Limit |
|---|---|
| `from`, `to` | `HH:MM`, from `00:00` to `24:00`, and `from` before `to` |
| `days` | `sun` to `sat`, at least one. No key means each day. |
| `color` | six hexadecimal digits |
| `brightness` | 0 to 100 |
| alert `name` | 1 to 32 characters of `a`–`z`, `0`–`9`, and `-`. Not `stop`. Unique. |
| `text` | 1 to 64 characters, not only spaces |
| `baseline` | 0 to 63 |
| `x` | -64 to 64 |
| `passes` | 1 to 10 |
| `seconds` | 1 to 3600 |
| `priority` | 0 to 255. A message has the priority 20. |
| `every` | 1 to 3600 |

`deploy/config.toml` has the legacy tables in this format, with comments. A
test parses it and compares it with `Tables::legacy()`.

### 3. Owned alerts and a table type in the core

- `Alert` gets `name: String` and `text: String`. The other fields do not
  change. `Alert::validate` gives an `AlertError` for a bad name, text, pass
  count, duration, or x and passes together.
- `alert::Alerts` holds `Vec<Rc<Alert>>`. `Alerts::new(Vec<Alert>)` checks
  each alert and that each name is unique. `Alerts::legacy()` replaces
  `BUILT_IN`. `find(name) -> Option<Rc<Alert>>` and `names()` (a `Display`
  value: `a, b, and c`) replace `find` and the list in `command.rs`.
- `AlertScene`, `Board`, and `Trigger` hold `Rc<Alert>`. A new scene clones
  the `Rc`, which does not allocate. The no-allocation tests stay.
- `Trigger::new` checks `from < to` and `every` from 1 s to 1 h.
- `Tables { schedule, alerts, triggers }` is a core type in a new module
  `gameclock_core::tables`, with `Tables::legacy()`. `Schedule`,
  `Alerts`, and `Trigger` get `PartialEq` for the example-file test.

`Rc` is correct because the core has no threads, and the loop owns all
tables. The control thread gets only the command text.

### 4. The alert name is looked up in the loop

`parse_command` gives `Command::Alert(Option<String>)` and
`Command::StopAlert`. `alert` with no name is `Alert(None)`. The loop looks up
the name in the current `Alerts`. It builds the replies "the alert command
has no name" and "unknown alert" with `alerts.names()`. The text of these
replies does not change for the built-in table.

### 5. `run` gets the tables and a reload function

`run(panel, mode, tables, reload, clock, wait)`:

- `tables: Tables` is the config at start.
- `reload: impl FnMut() -> Result<Tables, ConfigError>` reads the file again.
  `main` gives a closure over the path and its origin. Tests give a closure
  that returns scripted results.
- `Mode::base_scene(&Schedule)` makes the clock with the current schedule.

On `reload`:

1. Call `reload()`. An error gives the reply `error: <error>`, and nothing
   changes.
2. Keep the new `Tables`. `board.set_schedule` sets the schedule for new
   message and alert scenes. The scene on the panel keeps its schedule and
   its alert to its end.
3. If the mode is `clock`, `compositor.set_base` with a new clock. The next
   present shows it before the reply.
4. `Triggers::new(rules, now)`. The next fire time is at or after now.

### 6. Start order in `main`

1. Parse the arguments. A usage error exits 2.
2. `config::load`. An error writes `gameclockd: <error>` and exits 2. A
   missing default file writes `gameclockd: no config at <path>. The built-in
   tables apply.`
3. Then the signal thread, the time zone, the socket, the panel, and `run`,
   as now.

So a bad file stops the start before the socket and the panel exist.

### 7. Deployment

- `deploy/gameclockd.service` gets `RestartPreventExitStatus=2`. A usage or
  config error then stops the service, and `systemctl status` shows the error.
- `scripts/deploy.sh` copies `deploy/config.toml` to the temporary directory
  on the Pi. The remote command runs `[ -e /etc/gameclockd/config.toml ] ||
  sudo install -D -m 644 … /etc/gameclockd/config.toml`. `install -D` makes
  `/etc/gameclockd` with mode 0755.
- The file and its directory are readable by all users, so `reload` works
  after hzeller drops root privileges.

## Risks / Trade-offs

- [The Pi binary gets bigger] → We measure it (task group 9). The parse runs
  only at start and on `reload`.
- [A person edits the file with an error, and the Pi reboots] → The service
  stops with the error and does not restart. The panel is dark until the file
  is correct. To prevent this, a person sends `reload` after each edit: an
  error comes back in the reply, and the running tables do not change. The
  README describes this step.
- [`reload` as the `daemon` user cannot read a file with mode 0600] → The
  reply gives the read error, and the tables do not change. The deploy uses
  mode 0644.
- [A section replaces its whole built-in table] → A person who adds one
  alert must also copy the built-in alerts that they want to keep. The
  example file has all of them, so a person edits that file.
- [`Rc` in `Alert` users makes the core types `!Send`] → The loop is
  single-threaded. The control thread does not get core types.

## Migration Plan

- The first deploy installs `deploy/config.toml`. It gives the legacy tables,
  so the panel does not change.
- Rollback: deploy the earlier binary. It does not read the file, and
  `--config` is not in the unit, so the old binary starts.
