# The config file

`gameclockd` reads its schedule, its alerts, and its bedtime triggers from a
TOML file (ADR-0019). The file has no settings for the driver, the socket, or
the panel size. Those stay arguments.

## Where the file is

- The default path is `/etc/gameclockd/config.toml`.
- `--config=<path>` gives a different path.
- `scripts/deploy.sh` installs `deploy/config.toml` at the default path, but
  only if no file is there. A later deploy does not change your edits.
- The file must be readable by all users (mode 0644). On the Pi, the LED
  driver drops root privileges after it starts, so `reload` reads the file
  as the `daemon` user.

If no file is at the default path, the built-in tables apply, and the daemon
writes this line on standard error:

```
gameclockd: no config at /etc/gameclockd/config.toml. The built-in tables apply.
```

If no file is at a path that `--config` gives, the daemon does not start.

## How to change the config

1. Edit `/etc/gameclockd/config.toml` with `sudo`.
2. Send `sudo gameclockd ctl reload`.
3. Read the reply:
   - `ok`: the new tables apply now.
   - `error: <path>:<line>:<column>: <problem>`: the file has an error. The
     daemon keeps the tables that it has. Correct the line and send `reload`
     again.

Always send `reload` after an edit. A file with an error stops the next start
of the daemon, for example after a reboot. The service then stops, and the
panel stays dark until the file is correct. `sudo journalctl -u gameclockd`
shows the error.

After a `reload`:

- The clock uses the new schedule at once.
- New messages and alerts use the new schedule and the new alert table.
- The triggers start again from the current time.
- A message or an alert on the panel continues to its end with its old
  settings.

## The sections

The file has three sections. Each section is optional.

- A section in the file **replaces the whole built-in table** of that
  section. To add one alert, copy the built-in alerts that you want to keep.
  `deploy/config.toml` has all of them.
- A section that is not in the file uses its built-in table.
- Without a `[[trigger]]` section, the built-in bedtime triggers apply, but
  only for the alerts `bed-flash` and `bedtime` that the alert table has.

The daemon refuses a key that it does not know, and a value of the wrong
type.

### `[schedule]`: the clock color and the brightness

```toml
[schedule]
default = { color = "ff0000", brightness = 25 }

[[schedule.rule]]
days = ["sun", "mon", "tue", "wed", "thu"]
from = "23:00"
to = "23:15"
color = "ff8c00"
brightness = 100
```

The first rule that matches the local day and time gives the style. The
`default` style applies at the times that no rule matches. `default` is
necessary in a `[schedule]` section. Rules are optional.

| Key | Value |
|---|---|
| `default` | a `color` and a `brightness` |
| `days` | a list of `sun`, `mon`, `tue`, `wed`, `thu`, `fri`, `sat`, with one or more days. With no `days` key, the rule applies on each day. |
| `from`, `to` | `HH:MM`, from `00:00` to `24:00`. `from` must be before `to`. The rule ends before `to`. |
| `color` | six hexadecimal digits `RRGGBB`, for example `ff8c00` |
| `brightness` | 0 to 100 |

A period that crosses midnight is two rules.

### `[[alert]]`: the alerts

```toml
[[alert]]
name = "dinner"
text = "DINNER"
background = "black"
font = "small"
baseline = 20
color = "ffff00"
x = 11
seconds = 5
priority = 25
clear = "removable"
```

`sudo gameclockd ctl alert dinner` starts this alert.

| Key | Value |
|---|---|
| `name` | 1 to 32 characters of `a`–`z`, `0`–`9`, and `-`. It must be unique, and it cannot be `stop`. |
| `text` | 1 to 64 characters, not only spaces |
| `background` | `black`, or `color-wheel` (all pixels in one color that moves through red, green, and blue) |
| `font` | `small` (7×13 bold) or `large` (10×20) |
| `baseline` | the y position of the text baseline, 0 to 63 |
| `color` | `contrast` (black or white, whichever is easier to read on the background), or six hexadecimal digits |
| `x` | for a still text: the x position, -64 to 64 |
| `passes` | for a scrolling text: the number of passes, 1 to 10 |
| `seconds` | the longest time that the alert shows, 1 to 3600. A scrolling text also stops after its passes. |
| `priority` | 0 to 255 |
| `clear` | `removable` (the `clear` command removes the alert) or `kept` (only `alert stop` removes it) |

An alert has `x` or `passes`, but not both.

An alert starts only if its priority is higher than the priority of the item
on the panel. The base scene has the priority 0, and a message has the
priority 20. An alert with a priority of 20 or less does not interrupt a
message. A message that an alert interrupts starts again after the alert.

### `[[trigger]]`: alerts at set times

```toml
[[trigger]]
alert = "bed-flash"
days = ["sun", "mon", "tue", "wed", "thu"]
from = "23:00"
to = "23:05"
every = 10
```

The rule starts its alert at `from`, and then each `every` seconds, before
`to`, on its days, in local time.

| Key | Value |
|---|---|
| `alert` | the name of an alert in the alert table that applies: the `[[alert]]` entries of the file, or the built-in alerts |
| `days` | as in a schedule rule. With no `days` key, the rule applies on each day. |
| `from`, `to` | as in a schedule rule |
| `every` | 1 to 3600 seconds |

A trigger follows the priority rules. If the item on the panel has the same
or a higher priority, the trigger does nothing. If its alert shows already,
the trigger does nothing.

## The built-in tables

`deploy/config.toml` has the built-in tables in this format. A test checks
that the file and the built-in tables are equal.

- **Schedule:** red at 25 from 00:00, blue at 100 from 06:00, blue at 60 from
  10:00, and mellow orange at 60 from 21:00. From 23:00, mellow at 100 until
  23:15 on bedtime nights (Sunday to Thursday), and mellow at 25 at other
  times.
- **Alerts:**
  - `panic`: color wheel, 20 s, priority 40, removable
  - `bedtime`: lime large scrolling text, 2 passes or 20 s, priority 30, kept
  - `bed-flash`: red `BED!!!!!`, 1 s, priority 10, kept
- **Triggers:** on bedtime nights, `bed-flash` each 10 s from 23:00 to 23:05,
  and `bedtime` each 10 s from 23:05 to 23:16.
