# ADR-0017: Messages scroll from the injected time, and the daemon owns the queue

- **Status:** Accepted
- **Date:** 2026-11-12
- **Related:** ADR-0005, ADR-0006, ADR-0012, ADR-0014, ADR-0016. OpenSpec
  change `message-board`.

## Context

A message scrolls text across the panel for a number of passes. The legacy
clock moved the text 1 pixel for each loop of its render loop, so the speed
changed with the CPU load. It showed one message at a time. A new message
replaced the message on the panel. It used the brightness of its schedule.

The compositor holds scenes as `Box<dyn Scene>` (ADR-0012). After the daemon
puts a scene on the stack, it has no handle to it. The line protocol of
ADR-0016 has commands with no arguments.

## Decision

- **The position comes from the time.** A message holds its start time. Its
  position is the time since the start, divided by a fixed step of 40 ms for
  each pixel. A late wake does not slow the scroll.
- **The daemon owns the queue.** `run` holds a queue of at most 8 waiting
  messages, first in, first out. The compositor stack holds at most one
  message scene. Before each update, the loop removes finished scenes and
  starts the next message. The next message then starts at the same update,
  with no base frame between.
- **A message uses the brightness of the clock schedule.** Each message scene
  gets the schedule and gives its brightness for the current time.
- **Commands can have arguments.** The first word selects the command.
  `message` takes options that start with `--`, and then the text. The
  `ctl` client joins its command words with one space.

## Consequences

- Tests set the time and check the frame at that time. They do not count
  renders.
- The message parts (the message, the scroll, and the queue) are in the pure
  core, so `alerts` can use them again.
- While a message scrolls, the daemon renders 25 times each second. The CPU
  cost needs an on-device measurement.
- `clear` removes all foreground scenes. `alerts` must decide if `clear`
  also removes an alert.
- The message brightness follows the hard-coded schedule until
  `configurable-schedule`.

## Alternatives considered

- **1 pixel for each render, as in the legacy clock.** The speed then depends
  on the CPU load and on the 33 ms render cap.
- **A board scene on the stack that holds the queue.** The daemon then needs
  a shared handle, for example `Rc<RefCell<…>>`, to a scene that the
  compositor owns.
- **Replace the message on the panel, as in the legacy clock.** Two people
  who send a message at the same time then lose one of them.
- **Full brightness for messages.** A message at night is then much brighter
  than the clock.
