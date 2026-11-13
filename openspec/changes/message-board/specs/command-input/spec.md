## MODIFIED Requirements

### Requirement: One connection carries one command
A client SHALL send one command as one line of UTF-8 text, 256 bytes or
fewer. The line ends with a line feed or the end of the stream. The daemon SHALL
remove spaces at the start and the end of the line. It SHALL answer with one
line and then close the connection. A reply starts with `ok` or with
`error: `.

#### Scenario: Unknown command
- **WHEN** a client sends `frobnicate`
- **THEN** the reply is `error: unknown command 'frobnicate'. The commands are clock, test-pattern, status, message, and clear.`

#### Scenario: Line too long
- **WHEN** a client sends 300 bytes with no line feed
- **THEN** the reply is `error: the command is longer than 256 bytes.`

#### Scenario: Text that is not UTF-8
- **WHEN** a client sends the bytes 0xFF 0xFE and a line feed
- **THEN** the reply is `error: the command is not UTF-8 text.`

#### Scenario: Spaces around the command
- **WHEN** a client sends ` status ` and a line feed
- **THEN** the reply starts with `ok`

### Requirement: Status gives the state of the daemon
The command `status` SHALL get the reply `ok mode=<mode> zone=<zone>
brightness=<percent> messages=<count>`. The mode is `clock` or
`test-pattern`. The zone is the IANA name of the time zone, or `unnamed` for a
zone with no name. The brightness is the brightness of the last present. The
count is the number of messages on the panel and in the queue.

#### Scenario: Status of the clock
- **WHEN** the daemon shows the legacy clock in UTC at 1970-01-01T00:00:00.250Z, and a `status` command arrives
- **THEN** the reply is `ok mode=clock zone=UTC brightness=25 messages=0`

#### Scenario: Status of the test pattern
- **WHEN** the daemon shows the heartbeat in UTC, and a `status` command arrives
- **THEN** the reply is `ok mode=test-pattern zone=UTC brightness=100 messages=0`

#### Scenario: Status during a message
- **WHEN** one message scrolls, two messages wait, and a `status` command arrives
- **THEN** the reply ends with `messages=3`

### Requirement: The ctl client sends one command
`gameclockd ctl <command>` SHALL send the command to the control socket and
write the reply to standard output. It SHALL exit with status 0 for an `ok`
reply and 1 for an `error: ` reply. It SHALL exit with status 1 and write the
path and the error to standard error if it cannot connect. A `ctl` with no
command SHALL exit with status 2. The `--control-socket=<path>` argument
SHALL change the path.

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

## ADDED Requirements

### Requirement: The ctl client joins the command words
The `ctl` client SHALL join all arguments after the first command word with
one space, and send them as one command line. A `--control-socket=<path>`
argument SHALL count as an option only before the first command word.

#### Scenario: Message with no quotes
- **WHEN** the arguments are `ctl`, `message`, `hello`, and `world`
- **THEN** the client sends the command line `message hello world`

#### Scenario: Socket option after the command word
- **WHEN** the arguments are `ctl`, `message`, and `--control-socket=/tmp/a.sock`
- **THEN** the client sends the command line `message --control-socket=/tmp/a.sock` to the default socket path

#### Scenario: Message through the client
- **WHEN** the host build of `gameclockd` runs with a socket at a temporary path, and `gameclockd ctl --control-socket=<that path> message hello world` runs
- **THEN** the client writes `ok` and exits with status 0
