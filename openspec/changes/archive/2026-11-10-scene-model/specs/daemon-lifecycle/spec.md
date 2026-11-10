# Spec Delta

## RENAMED Requirements

- FROM: `### Requirement: Startup presents the test pattern once`
- TO: `### Requirement: Startup presents the base scene at once`

## MODIFIED Requirements

### Requirement: Startup presents the base scene at once
At startup, the daemon SHALL render its base scene into a frame of the
panel's size, for the injected time. It SHALL present that frame at full
brightness before it waits for the first time. The base scene is the
heartbeat scene.

#### Scenario: One present at startup
- **WHEN** the daemon runs with a 64×32 recording test panel at 1970-01-01T00:00:00.250Z and gets a shutdown request at its first wait
- **THEN** the panel recorded exactly one present
- **AND** that frame is identical to the heartbeat scene rendered into a new 64×32 frame at the same time
- **AND** the brightness was 100

#### Scenario: Frame follows the panel's size
- **WHEN** the daemon runs with a 32×16 recording test panel
- **THEN** the presented frame is 32×16

## ADDED Requirements

### Requirement: The daemon blocks between deadlines
Between renders, the daemon SHALL block until the compositor's deadline or a
shutdown signal. It SHALL NOT wake at other times. With no deadline, it SHALL
block with no timeout until a shutdown signal arrives.

#### Scenario: Wait ends at the scene deadline
- **WHEN** the daemon starts with the heartbeat scene at 1970-01-01T00:00:00.250Z
- **THEN** its first wait has the deadline 1970-01-01T00:00:01.000Z

#### Scenario: Static scene waits with no timeout
- **WHEN** the daemon runs with a base scene that has no next change
- **THEN** its first wait has no deadline
- **AND** the panel recorded exactly one present when the daemon returns

#### Scenario: No presents while waiting
- **WHEN** the daemon runs with a static base scene, and the shutdown request comes 500 ms after the first present
- **THEN** the panel recorded exactly one present when the daemon returns

#### Scenario: Heartbeat daemon wakes only each second (on-device)
- **WHEN** `gameclockd` shows the heartbeat for at least 10 s, and we then measure it for 60 s
- **THEN** the main thread's count of voluntary context switches increases by at least 55 and at most 180
- **AND** the main thread uses less than 1% of one core

### Requirement: The loop renders at each deadline
When a wait ends at its deadline, the daemon SHALL update the compositor with
the current time. It SHALL present the frame if the update gives one. Then it
SHALL wait for the next deadline.

#### Scenario: Heartbeat presents each second
- **WHEN** the daemon starts with a 64×32 recording test panel at 1970-01-01T00:00:00.250Z, and its first three waits each end at their deadline
- **THEN** the panel recorded four presents
- **AND** the marker of each present is at column 3, 4, 5, and 6 in that order

#### Scenario: Early wake does not present
- **WHEN** a wait with the deadline 1970-01-01T00:00:01.000Z ends at 1970-01-01T00:00:00.900Z
- **THEN** the daemon does not present
- **AND** its next wait has the deadline 1970-01-01T00:00:01.000Z

### Requirement: The daemon reads the time zone once at startup
At startup, the daemon SHALL get the system time zone. It SHALL use that time
zone for each injected time, and it SHALL write the zone name to standard
error. If the system time zone is not available, it SHALL use UTC and write a
warning.

#### Scenario: Time zone from the environment
- **WHEN** the host build of `gameclockd` starts with `TZ=America/Los_Angeles`
- **THEN** standard error has a line that says the time zone is `America/Los_Angeles`

#### Scenario: Missing time zone falls back to UTC
- **WHEN** the daemon cannot get the system time zone
- **THEN** it uses UTC for each injected time
- **AND** standard error has a line that says the system time zone is not available and the daemon uses UTC

## REMOVED Requirements

### Requirement: Idle means blocked, not polling
**Reason**: With scenes that change, the daemon must also wake at scene deadlines. The daemon no longer blocks only on the shutdown signal.
**Migration**: "The daemon blocks between deadlines" replaces it. A scene with no next change still gives a wait with no timeout.
