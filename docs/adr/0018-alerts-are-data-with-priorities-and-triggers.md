# ADR-0018: Alerts are data, with priorities and timed triggers

- **Status:** Accepted
- **Date:** 2026-11-15
- **Related:** ADR-0005, ADR-0006, ADR-0012, ADR-0013, ADR-0014, ADR-0017,
  ADR-0019.
  OpenSpec change `alerts`.

## Context

The legacy clock has a panic alert, a bedtime alert, and a timed bedtime
warning. Its code has one special mode for each alert. The roadmap asks for
one alert capability with configured instances.

An alert must interrupt a message, and some alerts must stay when a person
sends `clear`. ADR-0017 puts the message queue in `run`, and the stack holds
at most one message scene. The rules for what shows are then in the daemon
loop. They have tests only through that loop.

The daemon sleeps until the next change of the top scene (ADR-0013). With
only the clock, it wakes once each minute. The bedtime warning needs a start
each 10 s.

## Decision

- **An alert is a static definition.** One generic type holds the name, the
  background effect, the text and its style, the duration, the priority, and
  the clear rule. One alert scene renders each definition. The built-in
  alerts are a table in the core, as `Schedule::legacy()` is. A config file
  can give the same data later.
- **Priorities decide what shows.** The base scene, each alert, and a message
  have a priority. An alert starts only if its priority is higher than the
  priority of the item on the panel. Otherwise the daemon gives an error
  reply. Messages never interrupt. They wait in the queue.
- **An interrupted message restarts.** It goes back to the front of the
  queue, also when the queue is full.
- **Each alert has a clear rule.** `clear` removes an alert only if its rule
  permits it. `alert stop` removes any alert.
- **A pure board holds the rules.** A core type holds the queue and the item
  on the panel, and applies the rules above to the compositor. `run` holds
  the board, so the daemon still owns the queue (ADR-0017).
- **Timed triggers are data, and the daemon wakes for them.** A trigger rule
  has weekdays, a time window, a period, and an alert. The wait deadline is
  the earlier of the scene deadline and the next trigger time. A late wake
  starts at most one alert for each rule.

## Consequences

- A new alert is one more table entry. The error replies list the names
  from the table.
- Tests of the priority, restart, and clear rules run in the core with
  injected time.
- The daemon wakes each 10 s during the warning windows. At other times the
  triggers add no wake.
- A refused alert is not kept. A trigger starts it again at its next time.
- The definitions and the trigger rules stay in code until
  `configurable-schedule`.

## Alternatives considered

- **One mode or scene type for each alert, as in the legacy clock.** Each new
  alert then needs new code.
- **Keep the rules in `run`, with a second `Option` for the alert.** The
  rules then have tests only through the daemon loop.
- **A trigger scene on the compositor stack.** A scene cannot start another
  scene, and the priority rules need the board.
- **Alerts that wait in the queue.** A late panic is not useful, and a timed
  trigger starts the bedtime alert again in 10 s.
