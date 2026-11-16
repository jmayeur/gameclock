# alerts Specification

## Purpose

Defines how the daemon shows alerts. Each alert is a definition of one
generic type, with a background effect, a text and its style, a duration, a
priority, and interrupt rules. It covers the built-in alerts, the timed
triggers, and the `alert` command.

## Requirements

### Requirement: An alert is a definition, not code
Each alert SHALL be a definition with a name, a background effect, a text, a
text style, a duration, a priority, and a clear rule. One alert scene SHALL
render each definition. The built-in definitions are `panic`, `bedtime`, and
`bed-flash`.

#### Scenario: Built-in alerts
- **WHEN** a client sends `alert panic`, `alert bedtime`, or `alert bed-flash` while the clock shows
- **THEN** each reply is `ok`, and the named alert shows

#### Scenario: Two definitions with the same parts give the same frames
- **WHEN** a test definition has the same effect, text, style, and duration as `bed-flash`, but a different name
- **THEN** its frames are the same as the frames of `bed-flash` at the same times

### Requirement: The panic alert fills the panel with a color wheel
The `panic` alert SHALL fill the panel with one color of a color wheel. The
wheel position SHALL be 3 × (the number of 50 ms steps since the start),
modulo 765. Positions 0–255 go from blue to red, 256–511 from red to green,
and 512–764 from green to blue. The alert SHALL finish 20 s after its start.

#### Scenario: Wheel at the start
- **WHEN** `panic` starts at time T
- **THEN** each pixel of the frame at T that is not text is `(0,0,255)`

#### Scenario: Wheel position comes from the time
- **WHEN** `panic` starts at time T
- **THEN** each pixel of the frame at T + 5 s that is not text is `(211,44,0)`
- **AND** each pixel of the frame at T + 10 s that is not text is `(0,167,88)`

#### Scenario: Next change is the next wheel step
- **WHEN** `panic` starts at time T
- **THEN** its next change at T + 1.01 s is T + 1.05 s

#### Scenario: Panic finishes after 20 s
- **WHEN** `panic` starts at time T
- **THEN** it is not finished at T + 19.999 s
- **AND** it is finished at T + 20 s

### Requirement: The panic text contrasts with the wheel
The `panic` alert SHALL draw `PANIC!!` in the 7×13 bold font at x = 9,
baseline y = 20. The text SHALL be black if 299 × R + 587 × G + 114 × B of
the wheel color is more than 145,000. Otherwise the text SHALL be white.

#### Scenario: White text on a dark color
- **WHEN** `panic` starts at time T
- **THEN** the frame at T has `PANIC!!` in white `(255,255,255)` at x = 9, baseline y = 20, on `(0,0,255)`

#### Scenario: Black text on a light color
- **WHEN** `panic` starts at time T
- **THEN** the frame at T + 8.35 s has `PANIC!!` in black on `(10,245,0)`

### Requirement: The bedtime alert scrolls a large text
The `bedtime` alert SHALL scroll `GAME OVER - TURN IT OFF!` in lime
`(50,205,50)` in the 10×20 font at baseline y = 25, on black. It SHALL
scroll as a message scrolls, 1 pixel each 40 ms. It SHALL finish after 2
passes or 20 s, whichever is earlier.

#### Scenario: Bedtime text enters at the right edge
- **WHEN** `bedtime` starts at time T
- **THEN** the frame at T is black
- **AND** the frame at T + 1 s is the text drawn at x = 39, baseline y = 25

#### Scenario: Second pass starts at the right edge
- **WHEN** `bedtime` starts at time T
- **THEN** the frame at T + 12.16 s is black, and the alert is not finished
- **AND** the frame at T + 12.2 s is the text drawn at x = 63, baseline y = 25

#### Scenario: Duration ends the bedtime alert before its passes
- **WHEN** `bedtime` starts at time T
- **THEN** it is not finished at T + 19.999 s
- **AND** it is finished at T + 20 s

### Requirement: The bed-flash alert shows a still text for 1 s
The `bed-flash` alert SHALL draw `BED!!!!!` in red `(255,0,0)` in the 7×13
bold font at x = 8, baseline y = 20, on black. The image SHALL NOT change
until the alert finishes, 1 s after its start.

#### Scenario: Bed-flash frame
- **WHEN** `bed-flash` starts at time T
- **THEN** the frames at T and T + 0.5 s are `BED!!!!!` in red at x = 8, baseline y = 20

#### Scenario: Bed-flash next change is its finish
- **WHEN** `bed-flash` starts at time T
- **THEN** its next change at T is T + 1 s
- **AND** it is finished at T + 1 s

### Requirement: An alert uses the brightness of the clock schedule
An alert SHALL show at the brightness that the legacy clock schedule gives
for the current time.

#### Scenario: Panic at night
- **WHEN** `panic` shows at 01:00 local time
- **THEN** the compositor gives the brightness 25

#### Scenario: Bedtime on a bedtime night
- **WHEN** `bedtime` shows at 23:05 local time on a Monday
- **THEN** the compositor gives the brightness 100

### Requirement: An alert starts only above a lower priority
The priorities, from low to high, SHALL be: the base scene, `bed-flash`, a
message, `bedtime`, and `panic`. An alert SHALL start only if its priority is
higher than the priority of the item on the panel. The daemon SHALL present
the first frame of a started alert before it replies `ok`.

#### Scenario: Alert over the clock
- **WHEN** the clock shows, and `alert panic` arrives at time T
- **THEN** the daemon presents the `panic` frame for T, and then replies `ok`

#### Scenario: Panic interrupts bedtime
- **WHEN** `bedtime` shows, and `alert panic` arrives
- **THEN** the reply is `ok`, and `panic` shows
- **AND** the clock shows after `panic` finishes, not `bedtime`

#### Scenario: Lower priority is refused
- **WHEN** `panic` shows, and `alert bedtime` arrives
- **THEN** the reply is `error: the alert 'bedtime' did not start. The alert 'panic' on the panel has the same or a higher priority.`
- **AND** `panic` continues with no change

#### Scenario: Bed-flash does not interrupt a message
- **WHEN** a message scrolls, and `alert bed-flash` arrives
- **THEN** the reply is `error: the alert 'bed-flash' did not start. A message on the panel has the same or a higher priority.`

#### Scenario: Same alert again
- **WHEN** `bedtime` started at time T, and `alert bedtime` arrives at T + 5 s
- **THEN** the reply is `ok`
- **AND** the frame at T + 6 s is the same as when no second command arrived

### Requirement: The alert command
The command `alert <name>` SHALL start the named alert. The command `alert
stop` SHALL remove the alert on the panel and reply `ok`. A missing or
unknown name SHALL get an error reply.

#### Scenario: No name
- **WHEN** a client sends `alert`
- **THEN** the reply is `error: the alert command has no name. The alerts are panic, bedtime, and bed-flash.`

#### Scenario: Unknown name
- **WHEN** a client sends `alert fire`
- **THEN** the reply is `error: unknown alert 'fire'. The alerts are panic, bedtime, and bed-flash.`

#### Scenario: Stop an alert that clear keeps
- **WHEN** `bedtime` shows, no message waits, and `alert stop` arrives
- **THEN** the daemon presents the clock, and then replies `ok`

#### Scenario: Stop with no alert
- **WHEN** no alert shows, and `alert stop` arrives
- **THEN** the reply is `ok`, and the daemon does not present

### Requirement: Each alert has a clear rule
The command `clear` SHALL remove an alert only if the alert's clear rule
permits it. `clear` SHALL remove `panic`. It SHALL NOT remove `bedtime` or
`bed-flash`.

#### Scenario: Clear removes panic
- **WHEN** `panic` shows, two messages wait, and `clear` arrives
- **THEN** the daemon presents the clock, and then replies `ok`
- **AND** no message shows after that

#### Scenario: Clear keeps bedtime
- **WHEN** `bedtime` shows, two messages wait, and `clear` arrives
- **THEN** the reply is `ok`, and `bedtime` continues with no change
- **AND** the clock shows after `bedtime` finishes

### Requirement: Timed triggers start the bedtime warning
On bedtime nights (Sunday to Thursday, local time), the daemon SHALL start
`bed-flash` each 10 s from 23:00:00 to before 23:05:00. It SHALL start
`bedtime` each 10 s from 23:05:00 to before 23:16:00. A triggered alert SHALL
follow the priority rules. A refused trigger SHALL change nothing.

#### Scenario: Flash on a bedtime night
- **WHEN** the clock shows on a Monday, and the time goes from 22:59:59 to 23:00:00
- **THEN** `bed-flash` starts at 23:00:00, and the clock shows again at 23:00:01
- **AND** `bed-flash` starts again at 23:00:10

#### Scenario: Bedtime alert repeats with no gap
- **WHEN** the clock shows on a Monday, and the time goes from 23:04:59 to 23:05:30
- **THEN** `bedtime` starts at 23:05:00, and the trigger at 23:05:10 changes nothing
- **AND** a new `bedtime` starts at 23:05:20, when the first one finishes

#### Scenario: No trigger on a Friday
- **WHEN** the clock shows on a Friday from 22:59 to 23:20
- **THEN** no alert starts

#### Scenario: Last trigger
- **WHEN** the clock shows on a Monday
- **THEN** the last `bedtime` trigger is at 23:15:50
- **AND** no trigger starts an alert at 23:16:00

#### Scenario: Refused trigger during a message
- **WHEN** a message scrolls on a Monday at 23:00:00
- **THEN** `bed-flash` does not start, and the message continues with no change

### Requirement: The daemon wakes for the next trigger
The daemon's wait deadline SHALL be the earlier of the top scene's next
change and the next trigger time. A late wake SHALL start at most one
alert for each trigger rule.

#### Scenario: Wake before the next minute
- **WHEN** the clock shows on a Monday at 23:00:05, with no foreground scene
- **THEN** the daemon's wait deadline is 23:00:10

#### Scenario: No early wake outside a window
- **WHEN** the clock shows on a Monday at 12:00:05
- **THEN** the daemon's wait deadline is 12:01:00

#### Scenario: Late wake
- **WHEN** the daemon wakes on a Monday at 23:00:35 after it waited from 23:00:05
- **THEN** it starts `bed-flash` one time
- **AND** the next trigger time is 23:00:40

### Requirement: Alerts stay in their frame budget (on-device)
While `panic` or `bedtime` shows on the Pi Zero W, the main thread SHALL use
15% CPU or less. The median refresh rate SHALL stay at 170 Hz or more.

#### Scenario: Panic on the Pi
- **WHEN** the `gameclockd` service shows `panic` again each 20 s, and `scripts/measure.sh` runs for 60 s
- **THEN** the main thread CPU is 15% or less
- **AND** the median refresh rate with `--led-show-refresh` during `panic` is 170 Hz or more

#### Scenario: Bedtime on the Pi
- **WHEN** the `gameclockd` service shows `bedtime` again each 20 s, and `scripts/measure.sh` runs for 60 s
- **THEN** the main thread CPU is 15% or less
- **AND** the median refresh rate with `--led-show-refresh` during `bedtime` is 170 Hz or more
