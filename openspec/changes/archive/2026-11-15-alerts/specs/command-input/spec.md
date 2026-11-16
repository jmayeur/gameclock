## MODIFIED Requirements

### Requirement: One connection carries one command
A client SHALL send one command as one line of UTF-8 text, 256 bytes or
fewer. The line ends with a line feed or the end of the stream. The daemon SHALL
remove spaces at the start and the end of the line. It SHALL answer with one
line and then close the connection. A reply starts with `ok` or with
`error: `.

#### Scenario: Unknown command
- **WHEN** a client sends `frobnicate`
- **THEN** the reply is `error: unknown command 'frobnicate'. The commands are clock, test-pattern, status, message, clear, and alert.`

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
brightness=<percent> messages=<count> alert=<alert>`. The mode is `clock` or
`test-pattern`. The zone is the IANA name of the time zone, or `unnamed` for a
zone with no name. The brightness is the brightness of the last present. The
count is the number of messages on the panel and in the queue. The alert is
the name of the alert on the panel, or `none`.

#### Scenario: Status of the clock
- **WHEN** the daemon shows the legacy clock in UTC at 1970-01-01T00:00:00.250Z, and a `status` command arrives
- **THEN** the reply is `ok mode=clock zone=UTC brightness=25 messages=0 alert=none`

#### Scenario: Status of the test pattern
- **WHEN** the daemon shows the heartbeat in UTC, and a `status` command arrives
- **THEN** the reply is `ok mode=test-pattern zone=UTC brightness=100 messages=0 alert=none`

#### Scenario: Status during a message
- **WHEN** one message scrolls, two messages wait, and a `status` command arrives
- **THEN** the reply ends with `messages=3 alert=none`

#### Scenario: Status during an alert
- **WHEN** `panic` interrupts a message, one other message waits, and a `status` command arrives
- **THEN** the reply ends with `messages=2 alert=panic`
