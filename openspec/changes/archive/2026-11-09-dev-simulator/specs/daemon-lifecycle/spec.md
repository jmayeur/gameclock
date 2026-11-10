# Spec Delta

## MODIFIED Requirements

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
