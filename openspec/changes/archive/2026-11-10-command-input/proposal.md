# Proposal

## Why

The daemon shows the clock, but nothing can talk to it while it runs. Messages,
alerts, and later the web control all need one way to send a command to the
running daemon and to get an answer. Now is the right time: the next two
roadmap items (`message-board`, `alerts`) are commands.

## What Changes

- Add a **control socket**: a Unix stream socket. The default path is
  `/run/gameclockd/control.sock`. One connection carries one command line and
  gets one reply line.
- Add the first **commands**: `clock` and `test-pattern` replace the base scene
  while the daemon runs. `status` gives the mode, the time zone, and the
  current brightness.
- Add a **client** in the same binary: `gameclockd ctl <command>` sends one
  command, writes the reply, and exits with a status that tells the result.
- The daemon **waits on one event channel** instead of the signal pipe. A
  signal thread and a socket thread send events to it. The loop still blocks
  until the deadline or an event, so idle CPU stays near zero.
- Add the `--control-socket=<path>` argument. If the daemon cannot make the
  socket, it writes a warning and shows the clock with no command input.
- The compositor can **replace its base scene**.
- **BREAKING** (behavior): the argument parser accepts `ctl` and
  `--control-socket`. The unit gets a runtime directory for the socket.

## Capabilities

### New Capabilities

- `command-input`: the control socket, the line protocol, the commands and
  their replies, the limits for each connection, and the `ctl` client.

### Modified Capabilities

- `daemon-lifecycle`: the loop also wakes for a command. The argument rule
  accepts `--control-socket` and the `ctl` client. Signals reach the loop
  through the event channel.
- `scene-model`: the compositor can replace its base scene, and the next
  update then gives a frame.
- `pi-deployment`: the service unit makes `/run/gameclockd` for the socket.

## ADRs

- Uses ADR-0005 (pure core), ADR-0006 (render on change), ADR-0010 (the
  service and signals), ADR-0012 (scenes), and ADR-0013 (the deadline wait).
- **New ADR-0016: Command input over a Unix socket, and an event channel for
  the loop.** It adds a protocol and a process boundary. It replaces the
  signal-pipe wait of ADR-0013 with a channel and two threads. ADR-0013 gets
  a status note.

No new dependency. The socket is `std::os::unix::net`. The signal thread uses
the `iterator` module of `signal-hook`, which is already a dependency.

## Expected effect on the Pi

- Two more threads. Both block in the kernel when no command comes, so they
  use no CPU. The main thread still wakes once each minute.
- RSS goes up by a small amount: two thread stacks, set to 64 KiB each.
- A command gets its reply in less than 100 ms.
- **Measurement:** `scripts/measure.sh` for 60 s with the clock and no
  commands: per-thread CPU %, voluntary context switches, RSS, and the
  refresh rate. Then the time for 20 `status` commands with `gameclockd ctl`.

## Out of scope

- HTTP and the web form (`web-control`). The legacy semaphore file.
- Messages, alerts, and brightness commands (`message-board`, `alerts`).
- Access for users other than root. The socket is for root only.
- A command that changes the schedule (`configurable-schedule`).

## Impact

- `gameclock-core`: `Compositor::set_base`, and a getter for the last
  brightness.
- `gameclockd`: the event channel, the signal and socket threads, the command
  parser and replies, the `ctl` client, and the new arguments. `SignalWait`
  goes away.
- `deploy/gameclockd.service` and `scripts/test-unit.sh`: the runtime
  directory.
- Docs: ADR-0016, a status note on ADR-0013, the ADR index,
  `architecture.md`, `glossary.md`, `roadmap.md`, `CHANGELOG.md`, and the
  README.
