## RENAMED Requirements

- FROM: `### Requirement: Command-line arguments are driver flags and the test-pattern mode`
- TO: `### Requirement: Command-line arguments`

## MODIFIED Requirements

### Requirement: The daemon blocks between deadlines
Between renders, the daemon SHALL block until the compositor's deadline, a
command, or a shutdown signal. It SHALL NOT wake at other times. With no
deadline, it SHALL block with no timeout until a command or a shutdown signal
arrives.

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

#### Scenario: A command ends the wait
- **WHEN** the daemon runs with a static base scene and waits with no deadline, and a `status` command arrives
- **THEN** the daemon replies to the command and waits again with no deadline

#### Scenario: Heartbeat daemon wakes only each second (on-device)
- **WHEN** `gameclockd --test-pattern` shows the heartbeat for at least 10 s, and we then measure it for 60 s
- **THEN** the main thread's count of voluntary context switches increases by at least 55 and at most 180
- **AND** the main thread uses less than 1% of one core

#### Scenario: Clock daemon wakes only each minute (on-device)
- **WHEN** `gameclockd` shows the clock for at least 10 s, and we then measure it for 60 s
- **THEN** the main thread's count of voluntary context switches increases by at most 10
- **AND** the main thread uses less than 1% of one core
- **AND** each other thread of `gameclockd`, except the driver's refresh thread, uses less than 1% of one core

### Requirement: Command-line arguments
The daemon SHALL accept hzeller `--led-*` flags, `--test-pattern`, and
`--control-socket=<path>` as arguments. It SHALL give the `--led-*` flags to
the driver in their order, without the other arguments. If the first argument
is `ctl`, the binary SHALL run the client of `command-input` and SHALL NOT
start the daemon. Any other argument SHALL cause a usage error and exit
status 2, before the daemon touches the panel. A build without the hzeller
driver SHALL accept and ignore `--led-*` flags, with a notice.

#### Scenario: Unknown argument is a usage error
- **WHEN** `gameclockd` is started with `--frobnicate`
- **THEN** it exits with status 2, and standard error names `--frobnicate`

#### Scenario: Driver flags are ignored without the driver
- **WHEN** `gameclockd` built without the hzeller driver is started with `--led-pwm-bits=7`
- **THEN** it starts normally, and standard error says that `--led-*` flags are ignored

#### Scenario: Test-pattern mode is not a driver flag
- **WHEN** the arguments are `--led-pwm-bits=8`, `--test-pattern`, and `--led-slowdown-gpio=1`
- **THEN** the base scene is the heartbeat scene
- **AND** the driver flags are `--led-pwm-bits=8` and `--led-slowdown-gpio=1` in that order

#### Scenario: Host build names the test pattern
- **WHEN** the host build of `gameclockd` starts with `--test-pattern`
- **THEN** standard error has a line that says the base scene is the test pattern

#### Scenario: Socket path is not a driver flag
- **WHEN** the arguments are `--control-socket=/tmp/a.sock` and `--led-pwm-bits=8`
- **THEN** the socket path is `/tmp/a.sock`
- **AND** the driver flags are `--led-pwm-bits=8` only

#### Scenario: Default socket path
- **WHEN** the arguments have no `--control-socket`
- **THEN** the socket path is `/run/gameclockd/control.sock`

#### Scenario: Empty socket path is a usage error
- **WHEN** `gameclockd` is started with `--control-socket=`
- **THEN** it exits with status 2, and standard error names `--control-socket`
