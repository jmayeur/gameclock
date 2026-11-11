# daemon-lifecycle Specification

## Purpose

Defines how the `gameclockd` daemon starts, selects its panel, shows content,
idles without CPU use, and stops. The daemon can then run unattended as a
service on the Pi.

## Requirements

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

### Requirement: Signals shut the daemon down cleanly
On SIGTERM or SIGINT, the daemon SHALL stop the wait, release the panel, and
exit with status 0.

#### Scenario: SIGTERM exits with success
- **WHEN** the host build of `gameclockd` runs and receives SIGTERM
- **THEN** it exits with status 0 in 1 second or less

#### Scenario: SIGINT exits with success
- **WHEN** the host build of `gameclockd` runs and receives SIGINT
- **THEN** it exits with status 0 in 1 second or less

### Requirement: Panel errors are fatal and reported
If the panel cannot be created or cannot present a frame, the daemon SHALL
write the error to standard error. It SHALL exit with a non-zero status, so
the service manager can restart it.

#### Scenario: Present failure exits non-zero
- **WHEN** the daemon runs with a test panel whose present always fails
- **THEN** the daemon returns an error that contains the panel's error message
- **AND** the process exit status is non-zero

### Requirement: Panel is chosen at build time
A daemon built for the Pi with the hzeller driver SHALL drive the LED matrix.
A daemon built with the simulator SHALL draw frames in the terminal. A daemon
built with neither SHALL use a panel that only logs its presents. It SHALL say
so at startup. The daemon then runs on any development machine. A build with
both the hzeller driver and the simulator SHALL fail.

#### Scenario: Host build announces the logging panel
- **WHEN** `gameclockd` built without the hzeller driver starts
- **THEN** standard error has a line that says no LED driver is built in and frames are only logged
- **AND** standard error has a line that describes the presented 64×32 frame

#### Scenario: Simulator build draws in the terminal
- **WHEN** `gameclockd` built with the simulator starts and then receives SIGTERM
- **THEN** standard output has 16 rows of 64 `▀` cells
- **AND** standard error has a line that says frames are drawn in the terminal
- **AND** the process exits with status 0

#### Scenario: Driver and simulator together do not build
- **WHEN** `gameclockd` is built with both the hzeller and simulator features
- **THEN** the build fails with a message that says the two features cannot be used together

### Requirement: Command-line arguments
The daemon SHALL accept hzeller `--led-*` flags, `--test-pattern`, and
`--control-socket=<path>` as arguments. It SHALL give the `--led-*` flags to
the driver in their order, without the other arguments. Any other argument
SHALL cause a usage error and exit status 2, before the daemon touches the
panel. A build without the hzeller driver SHALL accept and ignore `--led-*`
flags, with a notice.

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

### Requirement: The first argument ctl selects the client
If the first argument is `ctl`, the binary SHALL run the client of
`command-input` with the other arguments. It SHALL NOT start the daemon.

#### Scenario: ctl with a command
- **WHEN** the arguments are `ctl` and `status`
- **THEN** the binary runs the client with the command `status` and the default socket path
- **AND** it does not start the daemon
