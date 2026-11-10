# daemon-lifecycle Specification

## Purpose

Defines how the `gameclockd` daemon starts, chooses its panel, shows content,
idles without consuming CPU, and shuts down, so it can run unattended as a
service on the Pi.

## Requirements

### Requirement: Startup presents the test pattern once
On startup the daemon SHALL render the test pattern into a frame sized to the
panel's dimensions and present it exactly once, at full brightness.

#### Scenario: One present at startup
- **WHEN** the daemon runs against a recording test panel of 64×32 and is then asked to shut down
- **THEN** the panel recorded exactly one present
- **AND** that frame is identical to the test pattern rendered into a fresh 64×32 frame
- **AND** the brightness was 100

#### Scenario: Frame follows the panel's size
- **WHEN** the daemon runs against a recording test panel of 32×16
- **THEN** the presented frame is 32×16

### Requirement: Idle means blocked, not polling
After presenting, while nothing visible changes, the daemon SHALL block until a
shutdown signal arrives. It SHALL NOT wake periodically, re-render, or present
again.

#### Scenario: No presents while waiting
- **WHEN** the daemon has presented and the shutdown request is delayed by 500 ms
- **THEN** the panel still records exactly one present when the daemon returns

#### Scenario: Idle daemon does not wake (on-device)
- **WHEN** `gameclockd` has been showing the test pattern for at least 10 s and is observed for a further 60 s
- **THEN** the main thread's voluntary context-switch count does not increase during the 60 s window
- **AND** its CPU time does not increase by more than 10 ms

### Requirement: Signals shut the daemon down cleanly
On SIGTERM or SIGINT the daemon SHALL stop waiting, release the panel, and exit
with status 0.

#### Scenario: SIGTERM exits with success
- **WHEN** the host build of `gameclockd` is running and receives SIGTERM
- **THEN** it exits with status 0 within 1 second

#### Scenario: SIGINT exits with success
- **WHEN** the host build of `gameclockd` is running and receives SIGINT
- **THEN** it exits with status 0 within 1 second

### Requirement: Panel errors are fatal and reported
If creating the panel or presenting a frame fails, the daemon SHALL print the
error to standard error and exit with a non-zero status, so the service manager
can restart it.

#### Scenario: Present failure exits non-zero
- **WHEN** the daemon runs against a test panel whose present always fails
- **THEN** the daemon returns an error that contains the panel's error message
- **AND** the process exit status is non-zero

### Requirement: Panel is chosen at build time
A daemon built for the Pi with the hzeller driver SHALL drive the LED matrix. A
daemon built without it SHALL use a panel that only logs what it would present,
and SHALL say so at startup, so the daemon runs on any development machine.

#### Scenario: Host build announces the logging panel
- **WHEN** `gameclockd` built without the hzeller driver starts
- **THEN** standard error contains a line saying that no LED driver is built in and frames are only logged
- **AND** a line describing the presented 64×32 frame

### Requirement: Command-line arguments are limited to driver flags
The daemon SHALL accept only hzeller `--led-*` flags as arguments. Any other
argument SHALL cause a usage error and exit status 2 before the panel is
touched. A build without the hzeller driver SHALL accept and ignore `--led-*`
flags, with a notice.

#### Scenario: Unknown argument is a usage error
- **WHEN** `gameclockd` is started with `--frobnicate`
- **THEN** it exits with status 2 and standard error names `--frobnicate`

#### Scenario: Driver flags are ignored without the driver
- **WHEN** `gameclockd` built without the hzeller driver is started with `--led-pwm-bits=7`
- **THEN** it starts normally and standard error notes that `--led-*` flags are ignored
