## Purpose

Defines the config file of the daemon. It covers the path and the sections
for the schedule, the alerts, and the triggers. It also covers the checks,
the error messages, the start behavior, and the `reload` command.

## ADDED Requirements

### Requirement: The config file and its path
The daemon SHALL read its config from a TOML file. The default path SHALL be
`/etc/gameclockd/config.toml`. The argument `--config=<path>` SHALL give a
different path.

#### Scenario: Default config path
- **WHEN** the arguments have no `--config`
- **THEN** the config path is `/etc/gameclockd/config.toml`

#### Scenario: Config path from the argument
- **WHEN** the arguments are `--config=/tmp/gc.toml` and `--led-pwm-bits=8`
- **THEN** the config path is `/tmp/gc.toml`
- **AND** the driver flags are `--led-pwm-bits=8` only

#### Scenario: Empty config path is a usage error
- **WHEN** `gameclockd` is started with `--config=`
- **THEN** it exits with status 2, and standard error names `--config`

### Requirement: A missing file or section uses the built-in tables
A missing file at the default path SHALL give the built-in legacy tables,
and the daemon SHALL write a line that says so. Each section that the file
does not have SHALL use its built-in table. A section in the file SHALL
replace the whole built-in table of that section.

#### Scenario: No file at the default path
- **WHEN** no file is at the default path, and the daemon starts
- **THEN** standard error has the line `gameclockd: no config at /etc/gameclockd/config.toml. The built-in tables apply.`
- **AND** the clock, the alerts, and the triggers are the same as in the legacy tables

#### Scenario: Empty file
- **WHEN** the config file is empty
- **THEN** the schedule, the alerts, and the triggers are the built-in tables

#### Scenario: Only a schedule section
- **WHEN** the file has only a `[schedule]` section with the default style blue `0000ff` at 60 and no rules
- **THEN** the clock is blue at brightness 60 at each time
- **AND** the alerts and the triggers are the built-in tables

#### Scenario: Missing file at a given path
- **WHEN** `gameclockd` is started with `--config=/tmp/none.toml`, and that file does not exist
- **THEN** it exits with status 2 before it touches the panel, and standard error names `/tmp/none.toml`

#### Scenario: The example file gives the legacy tables
- **WHEN** the daemon reads `deploy/config.toml`
- **THEN** the schedule, the alerts, and the triggers are equal to the built-in legacy tables

### Requirement: The schedule section
The `[schedule]` section SHALL have a `default` style and zero or more
`[[schedule.rule]]` entries. A style has a `color` of six hexadecimal digits
and a `brightness` from 0 to 100. A rule has `from` and `to` times as `HH:MM`
from `00:00` to `24:00`, a style, and optional `days`. The first rule that
matches gives the style.

#### Scenario: Rule on bedtime nights
- **WHEN** the schedule has the default red `ff0000` at 25 and one rule with `days = ["sun", "mon", "tue", "wed", "thu"]`, `from = "23:00"`, `to = "23:15"`, color `ff8c00`, and brightness 100
- **THEN** the clock at 23:05 on a Monday is `(255,140,0)` at brightness 100
- **AND** the clock at 23:05 on a Friday is `(255,0,0)` at brightness 25

#### Scenario: Rule with no days
- **WHEN** a rule has no `days` key
- **THEN** it applies on each day

### Requirement: The alert section
Each `[[alert]]` entry SHALL have a `name`, a `text`, a `background`
(`black` or `color-wheel`), a `font` (`small` or `large`), a `baseline`, a
`color` (`contrast` or six hexadecimal digits), a duration in `seconds`, a
`priority`, and a `clear` rule (`removable` or `kept`). It SHALL have `x` for
a still text, or `passes` for a scrolling text, but not both.

#### Scenario: A new alert from the file
- **WHEN** the file has one alert `dinner` with text `DINNER`, background `black`, font `small`, baseline 20, color `ffff00`, `x = 11`, 5 seconds, priority 25, and clear `removable`
- **THEN** `alert dinner` gets the reply `ok`
- **AND** the panel shows `DINNER` in yellow at x = 11, baseline y = 20, on black, for 5 s

#### Scenario: Alerts in the file replace the built-in alerts
- **WHEN** the file has one alert `dinner`, and a client sends `alert panic`
- **THEN** the reply is `error: unknown alert 'panic'. The alerts are dinner.`

### Requirement: The trigger section
Each `[[trigger]]` entry SHALL have an `alert` name, `from` and `to` times,
`every` as a number of seconds, and optional `days`. The alert SHALL be in
the alert table that applies. That is the alerts of the file, or the built-in
alerts if the file has no alert section.

#### Scenario: Trigger from the file
- **WHEN** the file has one trigger for `panic` on each day, from `07:00` to `07:01`, each 30 seconds
- **THEN** `panic` starts at 07:00:00 and at 07:00:30
- **AND** no `bed-flash` or `bedtime` starts at 23:00 on a Monday

#### Scenario: Trigger for an unknown alert
- **WHEN** a trigger names the alert `fire`, and no alert has that name
- **THEN** the config has the error `the trigger names the alert 'fire', which is not in the alert table.`

### Requirement: The daemon checks each value
The daemon SHALL refuse a config with a value out of its range, an unknown
key, a wrong type, or a TOML syntax error. The error SHALL give the path, the
line, the column, and the problem in one line.

#### Scenario: Brightness out of range
- **WHEN** line 3 of `/tmp/gc.toml` is `brightness = 120`, from column 1
- **THEN** the error is `/tmp/gc.toml:3:14: the brightness must be from 0 to 100.`

#### Scenario: Unknown key
- **WHEN** a rule has the key `colour`
- **THEN** the error names the line and the column of `colour`, and says that the key is not known

#### Scenario: Range values
- **WHEN** the file has a time `25:00`, a rule with `from` at or after `to`, a color `green`, or a day `monday`
- **OR** the file has an alert name `stop`, two alerts with one name, or a text of 65 characters
- **OR** the file has `passes = 0`, `seconds = 0`, an alert with `x` and `passes`, or `every = 0`
- **THEN** each one is an error with its line and column

#### Scenario: TOML syntax error
- **WHEN** the file has the line `[schedule` with no closing bracket
- **THEN** the error gives the line and the column of the syntax error

### Requirement: A file with errors stops the start
If the config file has an error at start, the daemon SHALL write the error
on standard error and exit with status 2. It SHALL NOT touch the panel.

#### Scenario: Bad file at start
- **WHEN** the config file has `brightness = 120` on line 3, and the daemon starts
- **THEN** standard error has `gameclockd: ` and the error with the path, line, and column
- **AND** the daemon exits with status 2, and the panel gets no present

### Requirement: The reload command reads the file again
The command `reload` SHALL read the config file again. If the file is
correct, the new tables SHALL apply at once, and the reply SHALL be `ok`. If
the file has an error, the reply SHALL be `error: ` and the error, and the
daemon SHALL keep its tables.

#### Scenario: Reload with a new schedule
- **WHEN** the clock shows blue, the file changes the default style to green `00ff00` with no rules, and `reload` arrives
- **THEN** the daemon presents the clock in green, and then replies `ok`

#### Scenario: Reload with an error
- **WHEN** the file has `brightness = 120` on line 3, and `reload` arrives
- **THEN** the reply is `error: ` and the error with the path, line, and column
- **AND** the clock shows as before

#### Scenario: Reload during an alert
- **WHEN** `panic` shows, and `reload` arrives with a file that has a different `panic`
- **THEN** the reply is `ok`, and the `panic` on the panel continues to its end
- **AND** the next `alert panic` uses the new definition

#### Scenario: Reload restarts the triggers
- **WHEN** the time is 22:00 on a Monday, and `reload` arrives with a file that has a trigger from `22:00` to `23:00` each 60 seconds
- **THEN** the next trigger time is 22:01:00

#### Scenario: Reload in the test-pattern mode
- **WHEN** the base scene is the heartbeat, and `reload` arrives with a correct file
- **THEN** the reply is `ok`, and the heartbeat continues
- **AND** a later `clock` command shows the clock with the new schedule

#### Scenario: Reload after the file is removed
- **WHEN** the daemon started with a file at the default path, the file is removed, and `reload` arrives
- **THEN** the reply is `ok`, and the built-in tables apply

### Requirement: Reload does not change the render loop (on-device)
The config SHALL be read only at start and on `reload`. With the clock only,
the main thread SHALL keep its idle numbers.

#### Scenario: Idle clock with a config file
- **WHEN** the `gameclockd` service shows the clock with `deploy/config.toml` installed, and `scripts/measure.sh` runs for 60 s
- **THEN** the main thread uses less than 1% of one core
- **AND** its voluntary context switches increase by at most 10

#### Scenario: Reload time on the Pi
- **WHEN** `sudo gameclockd ctl reload` runs with `deploy/config.toml` installed
- **THEN** the reply is `ok` in less than 1 s
