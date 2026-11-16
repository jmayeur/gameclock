## MODIFIED Requirements

### Requirement: The legacy schedule is the default
With no `[schedule]` section in the config, the clock SHALL give the styles of
the legacy schedule for the local time. Mellow is (255,140,0). Bedtime nights
are Sunday to Thursday.

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

#### Scenario: A schedule section replaces the legacy schedule
- **WHEN** the config has a `[schedule]` section with the default style blue `0000ff` at 60 and no rules
- **THEN** the clock at 23:05 on a Monday is `(0,0,255)` at brightness 60
