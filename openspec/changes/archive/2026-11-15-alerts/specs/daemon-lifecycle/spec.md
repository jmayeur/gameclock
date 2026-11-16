## MODIFIED Requirements

### Requirement: The daemon blocks between deadlines
Between renders, the daemon SHALL block until the earlier of the compositor's
deadline and the next trigger time, a command, or a shutdown signal. It SHALL
NOT wake at other times. With no deadline and no trigger time, it SHALL block
with no timeout until a command or a shutdown signal arrives.

#### Scenario: Wait ends at the scene deadline
- **WHEN** the daemon starts with the heartbeat scene at 1970-01-01T00:00:00.250Z
- **THEN** its first wait has the deadline 1970-01-01T00:00:01.000Z

#### Scenario: Static scene waits with no timeout
- **WHEN** the daemon runs with a base scene that has no next change, at 1970-01-01T00:00:00.250Z (a Thursday, in UTC)
- **THEN** its first wait has no scene deadline, only the first trigger time 1970-01-01T23:00:00Z
- **AND** the panel recorded exactly one present when the daemon returns

#### Scenario: No presents while waiting
- **WHEN** the daemon runs with a static base scene, and the shutdown request comes 500 ms after the first present
- **THEN** the panel recorded exactly one present when the daemon returns

#### Scenario: A command ends the wait
- **WHEN** the daemon runs with a static base scene and waits until the next trigger time, and a `status` command arrives
- **THEN** the daemon replies to the command and waits again until the same trigger time

#### Scenario: Heartbeat daemon wakes only each second (on-device)
- **WHEN** `gameclockd --test-pattern` shows the heartbeat for at least 10 s, and we then measure it for 60 s
- **THEN** the main thread's count of voluntary context switches increases by at least 55 and at most 180
- **AND** the main thread uses less than 1% of one core

#### Scenario: Clock daemon wakes only each minute (on-device)
- **WHEN** `gameclockd` shows the clock for at least 10 s, and we then measure it for 60 s
- **THEN** the main thread's count of voluntary context switches increases by at most 10
- **AND** the main thread uses less than 1% of one core
- **AND** each other thread of `gameclockd`, except the driver's refresh thread, uses less than 1% of one core
