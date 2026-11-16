## MODIFIED Requirements

### Requirement: Command-line arguments
The daemon SHALL accept hzeller `--led-*` flags, `--test-pattern`,
`--control-socket=<path>`, `--config=<path>`, and `--http=<address>` as
arguments. `--http=off` SHALL turn the web listener off. It SHALL give the
`--led-*` flags to the driver in their order, without the other arguments. Any other argument
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

#### Scenario: Default web address
- **WHEN** the arguments have no `--http`
- **THEN** the web address is `0.0.0.0:80`

#### Scenario: Web address is not a driver flag
- **WHEN** the arguments are `--http=127.0.0.1:8080` and `--led-pwm-bits=8`
- **THEN** the web address is `127.0.0.1:8080`
- **AND** the driver flags are `--led-pwm-bits=8` only

#### Scenario: Web listener off
- **WHEN** the arguments have `--http=off`
- **THEN** the daemon has no web address

#### Scenario: Bad web address is a usage error
- **WHEN** `gameclockd` is started with `--http=` or with `--http=clock:eighty`
- **THEN** it exits with status 2, and standard error names `--http` and gives an example address
