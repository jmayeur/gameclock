## ADDED Requirements

### Requirement: The service makes the socket directory
The service unit SHALL make `/run/gameclockd` before it starts the daemon,
with access for root only. systemd SHALL remove the directory when the
service stops.

#### Scenario: Unit has a private runtime directory
- **WHEN** the unit test reads `deploy/gameclockd.service`
- **THEN** it has the lines `RuntimeDirectory=gameclockd` and `RuntimeDirectoryMode=0700`

#### Scenario: Socket on the Pi (on-device)
- **WHEN** the `gameclockd` service runs on the Pi
- **THEN** `/run/gameclockd/control.sock` is a socket, and `/run/gameclockd` has mode 0700 and owner root

#### Scenario: Directory goes away at stop (on-device)
- **WHEN** the `gameclockd` service stops
- **THEN** `/run/gameclockd` does not exist
