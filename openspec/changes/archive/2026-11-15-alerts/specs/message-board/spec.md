## MODIFIED Requirements

### Requirement: Messages wait in a queue
If no message and no alert is on the panel, a new message SHALL start at
once. Otherwise it SHALL wait in a queue. When a message or an alert
finishes, the first message in the queue SHALL start at the same update. At
most 8 messages SHALL wait.

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

#### Scenario: Message waits during an alert
- **WHEN** `panic` shows, and `message hello` arrives
- **THEN** the reply is `ok`, and `panic` continues with no change
- **AND** the update at the finish of `panic` gives the first frame of `hello`

### Requirement: The clear command removes all messages
The command `clear` SHALL empty the queue and remove the message on the
panel. It SHALL remove an alert only if the alert's clear rule permits it.
The daemon SHALL present the next image before it replies `ok`.

#### Scenario: Clear during a message
- **WHEN** one message scrolls, two messages wait, and `clear` arrives
- **THEN** the daemon presents the clock and then replies `ok`
- **AND** no message shows after that

#### Scenario: Clear with no message
- **WHEN** no message is on the panel, and `clear` arrives
- **THEN** the reply is `ok`, and the daemon does not present

#### Scenario: Clear during an alert that clear keeps
- **WHEN** `bedtime` shows, one message waits, and `clear` arrives
- **THEN** the reply is `ok`, and the daemon does not present
- **AND** no message shows after `bedtime` finishes

## ADDED Requirements

### Requirement: An interrupted message restarts
When an alert interrupts a message, the message SHALL go back to the front
of the queue. It SHALL start again from the right edge, with all its passes,
when no alert shows. This SHALL also apply when 8 messages wait.

#### Scenario: Message restarts after panic
- **WHEN** the message `HI` starts at time T, and `panic` starts at T + 1 s
- **THEN** the update at T + 21 s gives a black frame, and the message is not finished
- **AND** the frame at T + 21.04 s is the text `HI` drawn at x = 63, baseline y = 20

#### Scenario: Interrupted message goes first
- **WHEN** `one` scrolls, `two` waits, and `panic` interrupts `one`
- **THEN** `one` starts when `panic` finishes, and `two` starts after `one`

#### Scenario: Interrupted message with a full queue
- **WHEN** one message scrolls, 8 messages wait, and `panic` interrupts it
- **THEN** 9 messages wait, and the interrupted message is first
- **AND** a new `message` command gets the reply `error: the message queue is full. 8 messages wait.`
