# message-board Specification

## Purpose

Defines how the daemon shows text messages. It covers the message and its
limits, the scroll, the brightness, the queue, and the `message` and `clear`
commands.

## Requirements

### Requirement: A message has a text, a pass count, and a color
A message SHALL have a text of 1 to 64 characters, a pass count from 1 to 10,
and a color. The default pass count is 4. The default color is green
`(0,150,0)`.

#### Scenario: Message with the defaults
- **WHEN** a client sends `message hello`
- **THEN** the daemon shows `hello` in the color `(0,150,0)` for 4 passes

#### Scenario: Message with options
- **WHEN** a client sends `message --passes=2 --color=ff8c00 good night`
- **THEN** the daemon shows `good night` in the color `(255,140,0)` for 2 passes

#### Scenario: Spaces in the text stay
- **WHEN** a client sends `message a  b`
- **THEN** the text of the message is `a  b`, with two spaces

### Requirement: The message command checks its arguments
The `message` command SHALL take options only before the text. The daemon
SHALL reply with an error and SHALL NOT queue the message if the text or an
option is not correct.

#### Scenario: No text
- **WHEN** a client sends `message` or `message --passes=2`
- **THEN** the reply is `error: the message has no text.`

#### Scenario: Text too long
- **WHEN** a client sends `message` and a text of 65 characters
- **THEN** the reply is `error: the message is longer than 64 characters.`

#### Scenario: Pass count out of range
- **WHEN** a client sends `message --passes=0 hi` or `message --passes=11 hi` or `message --passes=x hi`
- **THEN** the reply is `error: --passes must be a number from 1 to 10.`

#### Scenario: Color that is not correct
- **WHEN** a client sends `message --color=green hi` or `message --color=12345 hi`
- **THEN** the reply is `error: --color must be six hexadecimal digits, for example 00ff00.`

#### Scenario: Unknown option
- **WHEN** a client sends `message --speed=2 hi`
- **THEN** the reply is `error: unknown message option '--speed=2'. The options are --passes and --color.`

### Requirement: The text scrolls from right to left
A message SHALL draw its text in the 7×13 bold font at baseline y = 20. The
text SHALL start just past the right edge. Each 40 ms, it SHALL move 1 pixel
to the left. A pass SHALL end when the last column of the text goes past the left
edge. The position SHALL come from the time since the message started.

#### Scenario: Text enters at the right edge
- **WHEN** the message `HI` starts at time T
- **THEN** the frame at T is black
- **AND** the frame at T + 40 ms is the text `HI` drawn at x = 63, baseline y = 20

#### Scenario: Position comes from the time
- **WHEN** the message `HI` starts at time T
- **THEN** the frame at T + 1 s is the text `HI` drawn at x = 39, baseline y = 20

#### Scenario: The next pass starts at the right edge
- **WHEN** the message `HI` (14 pixels wide) with 2 passes starts at time T
- **THEN** the frame at T + 3.12 s is black, and the message is not finished
- **AND** the frame at T + 3.16 s is the text `HI` drawn at x = 63, baseline y = 20

#### Scenario: Character that the font does not have
- **WHEN** the message `é` scrolls
- **THEN** each frame is the same as the frame of the message `?` at the same time

### Requirement: A message finishes after its passes
A message SHALL finish when its last pass ends. Its next change SHALL be the
time of its next 1-pixel step, or its finish time if that is earlier.

#### Scenario: Finish after the passes
- **WHEN** the message `HI` with 4 passes starts at time T
- **THEN** it is not finished at T + 12.479 s
- **AND** it is finished at T + 12.48 s

#### Scenario: Next change is the next step
- **WHEN** the message `HI` starts at time T
- **THEN** its next change at T + 1.01 s is T + 1.04 s

### Requirement: A message uses the brightness of the clock schedule
A message SHALL show at the brightness that the legacy clock schedule gives
for the current time. The base scene does not change this.

#### Scenario: Message at night
- **WHEN** a message scrolls at 01:00 local time
- **THEN** the compositor gives the brightness 25

#### Scenario: Message in the morning
- **WHEN** a message scrolls at 07:00 local time
- **THEN** the compositor gives the brightness 100

#### Scenario: Message over the test pattern
- **WHEN** the base scene is the heartbeat, and a message scrolls at 01:00 local time
- **THEN** the compositor gives the brightness 25

### Requirement: Messages wait in a queue
If no message is on the panel, a new message SHALL start at once. Otherwise
it SHALL wait in a queue. When a message finishes, the first message in the
queue SHALL start at the same update. At most 8 messages SHALL wait.

#### Scenario: First message starts at once
- **WHEN** the daemon shows the clock, and `message hello` arrives at time T
- **THEN** the daemon presents the first frame of `hello` before it replies `ok`

#### Scenario: Second message waits
- **WHEN** `message one` arrives, and `message two` arrives while `one` scrolls
- **THEN** the reply to `message two` is `ok`
- **AND** the frames show `one` until it finishes

#### Scenario: Next message starts with no clock between
- **WHEN** `two` waits in the queue, and `one` finishes at time F
- **THEN** the update at F gives the first frame of `two`, not the clock

#### Scenario: Clock returns after the last message
- **WHEN** the last message finishes at time F, and the queue is empty
- **THEN** the update at F gives the clock

#### Scenario: Queue is full
- **WHEN** one message scrolls, 8 messages wait, and one more `message` command arrives
- **THEN** the reply is `error: the message queue is full. 8 messages wait.`

### Requirement: The clear command removes all messages
The command `clear` SHALL empty the queue and remove the message on the
panel. The daemon SHALL present the base scene before it replies `ok`.

#### Scenario: Clear during a message
- **WHEN** one message scrolls, two messages wait, and `clear` arrives
- **THEN** the daemon presents the clock and then replies `ok`
- **AND** no message shows after that

#### Scenario: Clear with no message
- **WHEN** no message is on the panel, and `clear` arrives
- **THEN** the reply is `ok`, and the daemon does not present

### Requirement: A message scroll stays in its frame budget (on-device)
While a message scrolls on the Pi Zero W, the main thread SHALL use 15% CPU or
less. The median refresh rate SHALL stay at 170 Hz or more.

#### Scenario: Long message on the Pi
- **WHEN** the `gameclockd` service scrolls a message of 64 characters with 10 passes, and `scripts/measure.sh` runs for 60 s
- **THEN** the main thread CPU is 15% or less
- **AND** the median refresh rate with `--led-show-refresh` during a scroll is 170 Hz or more
