## MODIFIED Requirements

### Requirement: Driver flags pass through over Pi defaults
The panel SHALL accept hzeller's standard `--led-*` command-line flags. It
SHALL apply them over built-in defaults for this hardware: 64 columns, 32
rows, one chain, one parallel, and the `adafruit-hat-pwm` GPIO mapping. A
flag on the command line SHALL win over a built-in default.

#### Scenario: Built-in defaults describe this Pi
- **WHEN** the panel's default driver options are read, before any flags apply
- **THEN** they give 64 columns, 32 rows, chain length 1, parallel 1, and hardware mapping `adafruit-hat-pwm`

#### Scenario: Driver flags reach the driver (on-device)
- **WHEN** `gameclockd` is started with `--led-show-refresh`
- **THEN** the driver writes its refresh rate to standard error while the panel shows the test pattern

#### Scenario: A flag overrides a default (on-device)
- **WHEN** `gameclockd` is started with `--led-gpio-mapping=regular` on the Adafruit HAT
- **THEN** the panel no longer shows the test pattern correctly
- **AND** this shows that the flag replaced the `adafruit-hat-pwm` default

#### Scenario: Invalid flags fail at startup (on-device)
- **WHEN** `gameclockd` is started with `--led-rows=abc`
- **THEN** it exits with a non-zero status and an error message, and does not drive the panel

#### Scenario: The PWM mapping has no moving lines (on-device)
- **WHEN** `gameclockd` shows the clock in red at brightness 25 with the default mapping
- **THEN** a person sees no white lines that move down the panel
