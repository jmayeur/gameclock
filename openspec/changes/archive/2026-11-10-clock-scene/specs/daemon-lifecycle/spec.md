## RENAMED Requirements

- FROM: `### Requirement: Command-line arguments are limited to driver flags`
- TO: `### Requirement: Command-line arguments are driver flags and the test-pattern mode`

## MODIFIED Requirements

### Requirement: Startup presents the base scene at once
At startup, the daemon SHALL render its base scene into a frame of the
panel's size, for the injected time. It SHALL present that frame at the
brightness that the compositor gives, before it waits for the first time.
The base scene is the clock with the legacy schedule. With `--test-pattern`,
the base scene is the heartbeat scene.

#### Scenario: One present at startup
- **WHEN** the daemon runs with the clock and a 64×32 recording test panel at 1970-01-01T00:00:00.250Z in UTC
- **AND** it gets a shutdown request at its first wait
- **THEN** the panel recorded exactly one present
- **AND** that frame is identical to the legacy clock rendered into a new 64×32 frame at the same time
- **AND** the brightness was 25

#### Scenario: Frame follows the panel's size
- **WHEN** the daemon runs with a 32×16 recording test panel
- **THEN** the presented frame is 32×16

#### Scenario: Host build names the clock
- **WHEN** the host build of `gameclockd` starts with no arguments
- **THEN** standard error has a line that says the base scene is the clock

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
- **WHEN** `gameclockd --test-pattern` shows the heartbeat for at least 10 s, and we then measure it for 60 s
- **THEN** the main thread's count of voluntary context switches increases by at least 55 and at most 180
- **AND** the main thread uses less than 1% of one core

#### Scenario: Clock daemon wakes only each minute (on-device)
- **WHEN** `gameclockd` shows the clock for at least 10 s, and we then measure it for 60 s
- **THEN** the main thread's count of voluntary context switches increases by at most 10
- **AND** the main thread uses less than 1% of one core

### Requirement: Command-line arguments are driver flags and the test-pattern mode
The daemon SHALL accept hzeller `--led-*` flags and `--test-pattern` as
arguments. It SHALL give the `--led-*` flags to the driver in their order,
without `--test-pattern`. Any other argument SHALL cause a usage error and
exit status 2, before the daemon touches the panel. A build without the
hzeller driver SHALL accept and ignore `--led-*` flags, with a notice.

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

## ADDED Requirements

### Requirement: The daemon presents at the scene's brightness
Each present SHALL use the brightness that the compositor gives with the
frame. A change of brightness alone SHALL cause a present.

#### Scenario: Brightness of the base scene
- **WHEN** the daemon runs with a static base scene at brightness 25
- **THEN** the panel recorded one present at brightness 25

#### Scenario: Brightness change presents the same image
- **WHEN** the daemon runs with a base scene that keeps one image, at brightness 25 before 1970-01-01T00:00:01Z and 60 after it
- **AND** its first wait ends at its deadline
- **THEN** the panel recorded two presents with the same frame
- **AND** their brightness values are 25 and 60 in that order
