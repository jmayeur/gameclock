## MODIFIED Requirements

### Requirement: Runs as a supervised service
The daemon SHALL run as a systemd service that starts as root. The driver
drops privileges after it initializes GPIO. systemd SHALL restart the daemon
after a failure and stop it with SIGTERM. It SHALL NOT restart the daemon after
exit status 2, a usage or config error. Starting it SHALL stop the legacy
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

#### Scenario: A config error is not restarted (on-device)
- **WHEN** `/etc/gameclockd/config.toml` has an error, and the `gameclockd` service starts
- **THEN** the service stops with exit status 2, and systemd does not start it again
- **AND** `journalctl -u gameclockd` shows the error with the path, line, and column

## ADDED Requirements

### Requirement: Deploy installs the example config once
`scripts/deploy.sh` SHALL install `deploy/config.toml` as
`/etc/gameclockd/config.toml`, mode 0644, only if that file does not exist.
It SHALL NOT change a config file that exists.

#### Scenario: The install of the config has a guard
- **WHEN** the deploy script runs with stub `ssh` and `scp`
- **THEN** it copies `deploy/config.toml` to the host
- **AND** the remote command installs it at `/etc/gameclockd/config.toml` only if no file is at that path

#### Scenario: Deploy keeps an edited config (on-device)
- **WHEN** `/etc/gameclockd/config.toml` on the Pi has an edit, and the deploy script runs
- **THEN** the file on the Pi has the same content as before the deploy
