## MODIFIED Requirements

### Requirement: An alert is a definition, not code
Each alert SHALL be a definition with a name, a background effect, a text, a
text style, a duration, a priority, and a clear rule. One alert scene SHALL
render each definition. The built-in definitions are `panic`, `bedtime`, and
`bed-flash`. They apply when the config has no `[[alert]]` entries. Alerts in
the config replace them.

#### Scenario: Built-in alerts
- **WHEN** a client sends `alert panic`, `alert bedtime`, or `alert bed-flash` while the clock shows
- **THEN** each reply is `ok`, and the named alert shows

#### Scenario: Two definitions with the same parts give the same frames
- **WHEN** a test definition has the same effect, text, style, and duration as `bed-flash`, but a different name
- **THEN** its frames are the same as the frames of `bed-flash` at the same times

### Requirement: The alert command
The command `alert <name>` SHALL start the named alert. The command `alert
stop` SHALL remove the alert on the panel and reply `ok`. A missing or
unknown name SHALL get an error reply. The reply SHALL list the names of the
alert table that applies now, in the order of the table.

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

### Requirement: Timed triggers start the bedtime warning
If the config has no `[[trigger]]` entries, the built-in trigger rules
SHALL apply. On bedtime nights (Sunday to Thursday, local time), they SHALL start
`bed-flash` each 10 s from 23:00:00 to before 23:05:00. They SHALL start
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
