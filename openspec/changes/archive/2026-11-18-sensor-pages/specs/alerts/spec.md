## MODIFIED Requirements

### Requirement: An alert starts only above a lower priority
The priorities, from low to high, SHALL be: the base scene, `bed-flash`, a
message, a page, `bedtime`, and `panic`. An alert SHALL start only if its priority is
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

#### Scenario: Bed-flash does not interrupt a page
- **WHEN** the `outdoor` page shows, and `alert bed-flash` arrives
- **THEN** the reply is `error: the alert 'bed-flash' did not start. A page on the panel has the same or a higher priority.`

#### Scenario: Bedtime interrupts a page
- **WHEN** the `indoor` page shows, and `alert bedtime` arrives
- **THEN** the reply is `ok`, and `bedtime` shows
- **AND** the clock shows after `bedtime` finishes, not the page
