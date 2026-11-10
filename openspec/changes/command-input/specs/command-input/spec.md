## Purpose

Defines how a command gets to the running daemon and how the daemon answers.
It covers the control socket, the line protocol, the commands and their
replies, the limits for each connection, and the `ctl` client.

## ADDED Requirements

### Requirement: The daemon listens on a control socket
At startup, before it creates the panel, the daemon SHALL listen on a Unix
stream socket. The default path is `/run/gameclockd/control.sock`. The
`--control-socket=<path>` argument SHALL change the path. Only the owner of
the socket file SHALL be able to connect.

#### Scenario: Socket at the given path
- **WHEN** the host build of `gameclockd` starts with `--control-socket=<a path in a new temporary directory>`
- **THEN** a socket file is at that path
- **AND** its permissions give read and write access to the owner only

#### Scenario: A stale socket file is replaced
- **WHEN** a socket file that no process listens on is at the path, and the daemon starts with that path
- **THEN** the daemon listens on the path, and a `status` command gets a reply

#### Scenario: The socket file goes away at shutdown
- **WHEN** the daemon with a socket at a temporary path receives SIGTERM
- **THEN** it exits with status 0, and the socket file is not at the path

### Requirement: The clock runs without command input
If the daemon cannot listen on the socket, it SHALL write a warning that names
the path and the error. It SHALL then run with no command input.

#### Scenario: Socket directory does not exist
- **WHEN** the host build of `gameclockd` starts with `--control-socket=<a path in a directory that does not exist>`
- **THEN** standard error has a warning that names the path and says that the daemon runs with no command input
- **AND** the daemon presents a frame and stops with status 0 on SIGTERM

### Requirement: One connection carries one command
A client SHALL send one command as one line of UTF-8 text, 256 bytes or
fewer. The line ends with a line feed or the end of the stream. The daemon SHALL
remove spaces at the start and the end of the line. It SHALL answer with one
line and then close the connection. A reply starts with `ok` or with
`error: `.

#### Scenario: Unknown command
- **WHEN** a client sends `frobnicate`
- **THEN** the reply is `error: unknown command 'frobnicate'. The commands are clock, test-pattern, and status.`

#### Scenario: Line too long
- **WHEN** a client sends 300 bytes with no line feed
- **THEN** the reply is `error: the command is longer than 256 bytes.`

#### Scenario: Text that is not UTF-8
- **WHEN** a client sends the bytes 0xFF 0xFE and a line feed
- **THEN** the reply is `error: the command is not UTF-8 text.`

#### Scenario: Spaces around the command
- **WHEN** a client sends ` status ` and a line feed
- **THEN** the reply starts with `ok`

### Requirement: A slow client does not stop the panel
The daemon SHALL close a connection that sends no complete line in 1 s. The
reply is then `error: no command in 1 s.` A connection SHALL NOT delay a render
or a present.

#### Scenario: Silent client is closed
- **WHEN** a client connects and sends nothing
- **THEN** it gets the reply `error: no command in 1 s.` 1 s to 2 s after it connected

#### Scenario: The heartbeat continues during a silent connection
- **WHEN** the host build of `gameclockd --test-pattern` runs, and a client connects and sends nothing for 1 s
- **THEN** the daemon presents in that time, as it does with no connection

### Requirement: A busy daemon gives an error reply
If the loop does not answer a command in 2 s, the client SHALL get the reply
`error: the daemon did not answer in 2 s.`

#### Scenario: Loop does not answer
- **WHEN** the socket code sends a command to a loop that never answers
- **THEN** the client gets that reply 2 s to 3 s after it sent the command

### Requirement: Mode commands replace the base scene
The command `clock` SHALL make the clock with the legacy schedule the base
scene. The command `test-pattern` SHALL make the heartbeat the base scene. The
daemon SHALL present the new base scene before it replies `ok`. If the mode is
already the one that the command names, the daemon SHALL reply `ok` and SHALL
NOT replace the scene.

#### Scenario: Switch to the test pattern
- **WHEN** the daemon shows the clock, and a `test-pattern` command arrives at 1970-01-01T00:00:00.500Z
- **THEN** the next present is the heartbeat scene at that time
- **AND** the reply is `ok`, after that present

#### Scenario: Switch back to the clock
- **WHEN** the daemon shows the heartbeat, and a `clock` command arrives
- **THEN** the next present is the legacy clock at that time
- **AND** the reply is `ok`

#### Scenario: Same mode does not present
- **WHEN** the daemon shows the clock, and a `clock` command arrives
- **THEN** the reply is `ok`
- **AND** the daemon does not present

### Requirement: Status gives the state of the daemon
The command `status` SHALL get the reply `ok mode=<mode> zone=<zone>
brightness=<percent>`. The mode is `clock` or `test-pattern`. The zone is the
IANA name of the time zone, or `unnamed` for a zone with no name. The
brightness is the brightness of the last present.

#### Scenario: Status of the clock
- **WHEN** the daemon shows the legacy clock in UTC at 1970-01-01T00:00:00.250Z, and a `status` command arrives
- **THEN** the reply is `ok mode=clock zone=UTC brightness=25`

#### Scenario: Status of the test pattern
- **WHEN** the daemon shows the heartbeat in UTC, and a `status` command arrives
- **THEN** the reply is `ok mode=test-pattern zone=UTC brightness=100`

### Requirement: The ctl client sends one command
`gameclockd ctl <command>` SHALL send the command to the control socket and
write the reply to standard output. It SHALL exit with status 0 for an `ok`
reply and 1 for an `error: ` reply. It SHALL exit with status 1 and write the
path and the error to standard error if it cannot connect. A `ctl` with no
command or more than one command SHALL exit with status 2. The
`--control-socket=<path>` argument SHALL change the path.

#### Scenario: Status through the client
- **WHEN** the host build of `gameclockd` runs with a socket at a temporary path, and `gameclockd ctl --control-socket=<that path> status` runs
- **THEN** the client writes a line that starts with `ok mode=clock` and exits with status 0

#### Scenario: Error reply through the client
- **WHEN** `gameclockd ctl --control-socket=<that path> frobnicate` runs
- **THEN** the client writes the unknown-command reply and exits with status 1

#### Scenario: No daemon
- **WHEN** `gameclockd ctl --control-socket=<a path with no socket> status` runs
- **THEN** it exits with status 1, and standard error names the path

#### Scenario: Missing command
- **WHEN** `gameclockd ctl` runs with no command
- **THEN** it exits with status 2, and standard error gives the usage

#### Scenario: Commands from root only (on-device)
- **WHEN** the `gameclockd` service runs, and user `pi` runs `gameclockd ctl status` without `sudo`
- **THEN** the client exits with status 1 and a permission error
- **AND** `sudo gameclockd ctl status` gets an `ok` reply
