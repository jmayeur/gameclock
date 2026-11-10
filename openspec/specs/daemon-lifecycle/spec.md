# daemon-lifecycle Specification

## Purpose

Defines how the `gameclockd` daemon starts, selects its panel, shows content,
idles without CPU use, and stops. The daemon can then run unattended as a
service on the Pi.

## Requirements

### Requirement: Startup presents the test pattern once
At startup, the daemon SHALL render the test pattern into a frame of the
panel's size. It SHALL present that frame exactly once, at full brightness.

#### Scenario: One present at startup
- **WHEN** the daemon runs with a 64×32 recording test panel and then gets a shutdown request
- **THEN** the panel recorded exactly one present
- **AND** that frame is identical to the test pattern rendered into a new 64×32 frame
- **AND** the brightness was 100

#### Scenario: Frame follows the panel's size
- **WHEN** the daemon runs with a 32×16 recording test panel
- **THEN** the presented frame is 32×16

### Requirement: Idle means blocked, not polling
After the present, while the image does not change, the daemon SHALL block
until a shutdown signal arrives. It SHALL NOT wake at intervals, render again,
or present again.

#### Scenario: No presents while waiting
- **WHEN** the daemon presents and the shutdown request comes 500 ms later
- **THEN** the panel shows exactly one recorded present when the daemon returns

#### Scenario: Idle daemon does not wake (on-device)
- **WHEN** `gameclockd` shows the test pattern for at least 10 s, and we then measure it for 60 s
- **THEN** the main thread's count of voluntary context switches does not increase in the 60 s
- **AND** its CPU time does not increase by more than 10 ms

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

### Requirement: Command-line arguments are limited to driver flags
The daemon SHALL accept only hzeller `--led-*` flags as arguments. Any other
argument SHALL cause a usage error and exit status 2, before the daemon
touches the panel. A build without the hzeller driver SHALL accept and ignore
`--led-*` flags, with a notice.

#### Scenario: Unknown argument is a usage error
- **WHEN** `gameclockd` is started with `--frobnicate`
- **THEN** it exits with status 2, and standard error names `--frobnicate`

#### Scenario: Driver flags are ignored without the driver
- **WHEN** `gameclockd` built without the hzeller driver is started with `--led-pwm-bits=7`
- **THEN** it starts normally, and standard error says that `--led-*` flags are ignored
