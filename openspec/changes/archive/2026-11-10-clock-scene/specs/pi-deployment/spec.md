## MODIFIED Requirements

### Requirement: Runs as a supervised service
The daemon SHALL run as a systemd service that starts as root. The driver
drops privileges after it initializes GPIO. systemd SHALL restart the daemon
after a failure and stop it with SIGTERM. Starting it SHALL stop the legacy
clock service. Starting the legacy service SHALL stop it. The two never drive
the panel at the same time.

#### Scenario: Service replaces the legacy clock (on-device)
- **WHEN** the legacy clock service runs and the `gameclockd` service is started
- **THEN** the legacy clock service stops and the panel shows the clock

#### Scenario: Legacy clock is a one-command rollback (on-device)
- **WHEN** the `gameclockd` service runs and the legacy clock service is started
- **THEN** the `gameclockd` service stops and the panel shows the legacy clock

#### Scenario: Crash is restarted (on-device)
- **WHEN** the `gameclockd` process is killed with SIGKILL
- **THEN** systemd starts it again in 10 seconds or less

## ADDED Requirements

### Requirement: The service sets the PWM bits
The service unit SHALL start the daemon with `--led-pwm-bits=8` (ADR-0015).
The flags from `/etc/default/gameclockd` SHALL come after it, so that a
device can set a different value.

#### Scenario: Unit passes 8 PWM bits first
- **WHEN** the unit test reads `deploy/gameclockd.service`
- **THEN** its start command is `/usr/local/bin/gameclockd --led-pwm-bits=8 $GAMECLOCKD_ARGS`

#### Scenario: Clock colors at 8 PWM bits (on-device)
- **WHEN** the service shows the clock at brightness 25 in red and in mellow
- **THEN** a person sees each color as steady and correct, with no visible color steps, flicker, or moving lines
