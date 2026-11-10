## ADDED Requirements

### Requirement: The clock shows the local time in the legacy format
The clock scene SHALL show the local time of the injected time as two hour
digits, a colon, two minute digits, and `AM` or `PM`. The hour SHALL be 12
for the first hour after midnight and after noon. It SHALL draw the text in
the 7×13 bold font with its left edge at x=8 and its baseline at y=20. All
other pixels SHALL be black.

#### Scenario: Morning time
- **WHEN** the clock scene is rendered into a 64×32 frame at 2026-11-10T09:05:30 in UTC
- **THEN** the frame is identical to the text `09:05AM` in the 7×13 bold font at x=8, baseline y=20, on black
- **AND** the text has the color that the schedule gives

#### Scenario: Evening time
- **WHEN** the clock scene is rendered at 2026-11-10T21:05:00 in UTC
- **THEN** the frame shows the text `09:05PM`

#### Scenario: Midnight is 12 AM
- **WHEN** the clock scene is rendered at 2026-11-10T00:00:00 in UTC
- **THEN** the frame shows the text `12:00AM`

#### Scenario: Noon is 12 PM
- **WHEN** the clock scene is rendered at 2026-11-10T12:30:00 in UTC
- **THEN** the frame shows the text `12:30PM`

#### Scenario: Time zone of the injected time
- **WHEN** the clock scene is rendered at 2026-11-10T17:05:00Z in the time zone America/Los_Angeles
- **THEN** the frame shows the text `09:05AM`

#### Scenario: Daylight saving time starts
- **WHEN** the clock scene is rendered at 2026-03-08T10:00:00Z in the time zone America/Los_Angeles
- **THEN** the frame shows the text `03:00AM`

### Requirement: The clock changes only at each new minute
The clock scene's next change SHALL be the start of the next whole minute of
the injected time. The clock scene SHALL never be finished.

#### Scenario: Next change inside a minute
- **WHEN** the clock scene's next change is asked at 2026-11-10T09:05:30.250Z
- **THEN** it is 2026-11-10T09:06:00Z

#### Scenario: Next change on a whole minute
- **WHEN** the clock scene's next change is asked at 2026-11-10T09:05:00Z
- **THEN** it is 2026-11-10T09:06:00Z

#### Scenario: Next change across the daylight saving time gap
- **WHEN** the clock scene's next change is asked at 2026-03-08T09:59:30Z in the time zone America/Los_Angeles
- **THEN** it is 2026-03-08T10:00:00Z

#### Scenario: Never finished
- **WHEN** the clock scene is asked if it is finished at any time
- **THEN** the answer is no

### Requirement: The schedule sets the color and brightness
The clock scene SHALL get its text color and its brightness from a
time-of-day schedule. A schedule is a list of rules and one default style. A
rule has a start and an end time of day in whole minutes, a set of weekdays,
and a style. A style is a color and a brightness.

#### Scenario: Text color and brightness from the style
- **WHEN** the clock uses a schedule whose only rule is 00:00 to 24:00 in green (0,255,0) at 40, and it is rendered at any time
- **THEN** each lit pixel of the frame is green (0,255,0)
- **AND** the scene brightness is 40

### Requirement: The first matching rule gives the style
A rule SHALL match when the local time of day is at or after its start and
before its end. Its weekday set SHALL also be empty or have the local
weekday. The first rule that matches SHALL give the style. If no rule
matches, the default style SHALL apply.

#### Scenario: First matching rule wins
- **WHEN** a schedule has a rule for 10:00 to 12:00 in green at 40, then a rule for 00:00 to 24:00 in white at 80
- **AND** the clock is rendered at 11:00 UTC
- **THEN** the text is green and the scene brightness is 40

#### Scenario: End time is not part of the rule
- **WHEN** the same schedule is used at 12:00 UTC
- **THEN** the text is white and the scene brightness is 80

#### Scenario: Weekday set limits a rule
- **WHEN** a schedule has a rule for Monday only, 00:00 to 24:00, in green at 40, and the default style is white at 80
- **AND** the clock is rendered on Tuesday 2026-11-10 at 08:00 UTC
- **THEN** the text is white and the scene brightness is 80

#### Scenario: No rule matches
- **WHEN** a schedule has no rules and the default style is red at 25
- **THEN** the text is red and the scene brightness is 25 at each time

#### Scenario: Rule times are whole minutes
- **WHEN** a rule is made with an hour of 24 and a minute other than 0, an hour above 24, or a minute above 59
- **THEN** the rule is rejected

#### Scenario: A rule must not be empty
- **WHEN** a rule is made with a start time that is not before its end time
- **THEN** the rule is rejected

### Requirement: The legacy schedule is the default
The daemon's clock SHALL use the legacy schedule. It SHALL give these styles
for the local time. Mellow is the color (255,140,0). Bedtime nights are
Sunday to Thursday.

| Local time | Color | Brightness |
|---|---|---|
| 00:00 to 05:59 | red (255,0,0) | 25 |
| 06:00 to 09:59 | blue (0,0,255) | 100 |
| 10:00 to 20:59 | blue (0,0,255) | 60 |
| 21:00 to 22:59 | mellow | 60 |
| 23:00 to 23:14 on bedtime nights | mellow | 100 |
| 23:00 to 23:59 at other times | mellow | 25 |

#### Scenario: Each period of the legacy schedule
- **WHEN** the legacy clock is rendered on Tuesday 2026-11-10 in UTC at 05:59, 06:00, 09:59, 10:00, 20:59, 21:00, 22:59, 23:00, 23:14, 23:15, and 23:59
- **THEN** the styles in that order are red 25, blue 100, blue 100, blue 60, blue 60, and mellow 60
- **AND** after them, mellow 60, mellow 100, mellow 100, mellow 25, and mellow 25

#### Scenario: Bedtime nights are Sunday to Thursday
- **WHEN** the legacy clock is rendered at 23:05 UTC on each day from Sunday 2026-11-08 to Saturday 2026-11-14
- **THEN** the brightness is 100 from Sunday to Thursday and 25 on Friday and Saturday

#### Scenario: Weekday of the local date
- **WHEN** the legacy clock is rendered at 2026-11-14T07:05:00Z in the time zone America/Los_Angeles, which is Friday 23:05 local time
- **THEN** the brightness is 25

### Requirement: Clock updates do not allocate heap memory
After the first update, a compositor with the clock as its base scene SHALL
NOT allocate heap memory for an update.

#### Scenario: One thousand minutes allocate nothing
- **WHEN** a compositor with the legacy clock is updated once, and then 1000 more times with the time 1 minute later each time
- **THEN** the 1000 updates make zero heap allocations
