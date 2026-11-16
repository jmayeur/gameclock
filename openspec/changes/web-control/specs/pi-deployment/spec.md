## MODIFIED Requirements

### Requirement: Runs as a supervised service
The daemon SHALL run as a systemd service that starts as root. The driver
drops privileges after it initializes GPIO. systemd SHALL restart the daemon
after a failure and stop it with SIGTERM. It SHALL NOT restart the daemon after
exit status 2, a usage or config error. Starting it SHALL stop the legacy
clock service. Starting the legacy service SHALL stop it. The two never drive
the panel at the same time. The service SHALL listen for HTTP on port 80.

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

#### Scenario: Web page on port 80 (on-device)
- **WHEN** the `gameclockd` service runs, and nginx and PM2 are stopped
- **THEN** `sudo ss -ltnp` shows `gameclockd` on `0.0.0.0:80`
- **AND** `GET /health` from a laptop on the LAN gets status 200

### Requirement: Starting at boot is an explicit choice
Installing the service SHALL NOT enable it at boot. A separate, explicit deploy
option SHALL enable it at boot and disable the legacy clock, nginx, and PM2 at
boot. Exactly one of the two web stacks is then enabled.

#### Scenario: Plain deploy leaves boot unchanged (on-device)
- **WHEN** the deploy script runs without the enable option
- **THEN** after a reboot, the legacy clock, nginx, and PM2 run, and `gameclockd` does not

#### Scenario: Enable swaps the boot service (on-device)
- **WHEN** the deploy script runs with the enable option and the Pi reboots
- **THEN** `gameclockd` runs and serves the page on port 80
- **AND** the legacy clock service, `nginx.service`, and `pm2-pi.service` are disabled and do not run

## ADDED Requirements

### Requirement: The deploy moves port 80 between the two stacks
`scripts/deploy.sh` SHALL stop `nginx.service` and `pm2-pi.service` before it
restarts `gameclockd`, so that the daemon can bind port 80. With `--enable`, it
SHALL also disable them at boot. With `--rollback`, it SHALL stop
`gameclockd`, and then enable and start the legacy clock, nginx, and PM2.

#### Scenario: Deploy stops the legacy web stack
- **WHEN** the deploy script runs with stub `ssh` and `scp`
- **THEN** the remote command stops `nginx.service` and `pm2-pi.service` before it restarts `gameclockd`
- **AND** it does not disable them

#### Scenario: Enable disables the legacy web stack
- **WHEN** the deploy script runs with `--enable` and stub `ssh` and `scp`
- **THEN** the remote command disables `clock.svc.service`, `nginx.service`, and `pm2-pi.service`, and enables `gameclockd`

#### Scenario: Rollback starts the legacy web stack
- **WHEN** the deploy script runs with `--rollback` and a stub `ssh`
- **THEN** the remote command disables and stops `gameclockd` first
- **AND** it then enables and starts `clock.svc.service`, `nginx.service`, and `pm2-pi.service`

#### Scenario: Rollback gives back the legacy page (on-device)
- **WHEN** the deploy script runs with `--rollback`
- **THEN** the legacy page answers on port 80 from a laptop on the LAN, and a message from it shows on the legacy clock
