## MODIFIED Requirements

### Requirement: One connection carries one command
A client SHALL send one command as one line of UTF-8 text, 256 bytes or
fewer. The line ends with a line feed or the end of the stream. The daemon SHALL
remove spaces at the start and the end of the line. It SHALL answer with one
line and then close the connection. A reply starts with `ok` or with
`error: `.

#### Scenario: Unknown command
- **WHEN** a client sends `frobnicate`
- **THEN** the reply is `error: unknown command 'frobnicate'. The commands are clock, test-pattern, status, message, clear, alert, and reload.`

#### Scenario: Line too long
- **WHEN** a client sends 300 bytes with no line feed
- **THEN** the reply is `error: the command is longer than 256 bytes.`

#### Scenario: Text that is not UTF-8
- **WHEN** a client sends the bytes 0xFF 0xFE and a line feed
- **THEN** the reply is `error: the command is not UTF-8 text.`

#### Scenario: Spaces around the command
- **WHEN** a client sends ` status ` and a line feed
- **THEN** the reply starts with `ok`
