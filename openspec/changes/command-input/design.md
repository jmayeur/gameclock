# Design

## Context

- The loop waits in `SignalWait`: a read with a timeout on one end of a
  `UnixStream` pair. The signal handlers of `signal-hook` write to the other
  end (ADR-0013). The read can wait for one file descriptor only.
- ADR-0013 says that `command-input` must decide how the loop waits for more
  than one source. It names `poll` and a channel. `poll` needs `unsafe` code
  or a new dependency. A channel needs threads.
- hzeller drops root privileges to the `daemon` user when the matrix is
  created. The change applies to the whole process. A socket that is open
  before that time stays usable.
- `run` takes the base scene, the clock, and the wait. The `Clock` trait gives
  a `Zoned`, so the time zone of `status` comes from `now.time_zone()`.

## Goals / Non-Goals

**Goals:**

- One wait for all events, with no `unsafe` code and no new dependency.
- No CPU when no command comes. The display never waits for a client.
- A protocol that web control can use later from inside the daemon: a command
  and a reply, with no knowledge of the socket.

**Non-Goals:**

- More than one connection at a time. Commands are rare and short.
- Authentication. File permissions limit the socket to root.
- A command queue with priorities.

## Decisions

### 1. An event channel replaces the signal pipe (ADR-0016)

`enum Event { Shutdown, Command(Request) }` on a `std::sync::mpsc` channel.
`Request` holds the parsed `Command` and a `Sender<String>` for the reply.
`ChannelWait` implements `Wait` with `recv_timeout` until the deadline, or
`recv` with no deadline. `Wake` gets a third case, `Command(Request)`. A
closed channel means shutdown.

Alternatives: keep the pipe and add a byte for each command (the loop then
counts bytes to tell a signal from a command). `poll(2)` through `rustix` (a
new dependency for two file descriptors).

### 2. Two small threads

- **Signal thread.** `signal_hook::iterator::Signals` for SIGTERM and SIGINT,
  made in `main` before the panel. The thread sends `Event::Shutdown` and
  ends.
- **Socket thread.** It owns the `UnixListener` and accepts one connection at
  a time. It sets a read timeout of 1 s and reads at most 257 bytes or to the
  first line feed. It parses the command, sends the request, and waits for
  the reply for 2 s at most. Then it writes the reply line and closes the
  connection.

Both threads use `std::thread::Builder` with a 64 KiB stack. Neither thread
is joined. The process exits after `run` returns and the panel drops.

### 3. The socket file

- The daemon makes the socket before it creates the panel, so it is still
  root. It sets the file mode to 0600.
- **Stale file.** If a file is at the path, the daemon tries to connect. If
  the connection is refused, it removes the file and binds. If the connection
  works, another daemon listens. The daemon then writes a warning and runs
  with no command input. It does not take the path of a running daemon.
- **Shutdown.** `main` removes the file after `run` returns. On the Pi, the
  process is the `daemon` user then, and it cannot remove the file. That is
  not an error: systemd removes `/run/gameclockd` at stop.
- **Failure.** If the bind fails, the daemon writes one warning and runs with
  no command input. The clock is the primary job.

### 4. Commands are pure functions in `gameclockd`

`parse_command(&[u8]) -> Result<Command, CommandError>` checks the length,
UTF-8, and the name. `Command` is `Mode(Mode)` or `Status`. The reply text
comes from one function for each case, so the unit tests check exact
strings. The socket code only moves bytes.

### 5. The loop handles a command at once

On `Wake::Command`, `run` does this:

1. For `Mode(m)` with `m` different from the current mode, it calls
   `compositor.set_base(m.base_scene())`. Then it updates and presents with
   the current time.
2. For `Status`, it builds the reply from the mode, the zone name of
   `clock.now()`, and `compositor.last_brightness()`.
3. It sends the reply. Then it waits again for the deadline.

A command allocates memory (a box and a string). That is not on the path for
each frame (ADR-0006).

### 6. `set_base` and `last_brightness` in the compositor

`set_base` replaces the box and sets `started = false`. The next update then
gives a frame. `last_brightness` gives `shown_brightness`, which is full
brightness before the first update.

### 7. Arguments and the client

`parse_args` gives `Invocation::Daemon(Args)` or `Invocation::Ctl(CtlArgs)`.
`Args` gets `socket: PathBuf`, with the default
`/run/gameclockd/control.sock`. The client connects, writes the command and
a line feed, and closes its write half. It reads one line with a 3 s timeout
and writes it to standard output. Its exit status comes from the first word
of the reply.

### 8. The unit

`RuntimeDirectory=gameclockd` and `RuntimeDirectoryMode=0700`. systemd makes
the directory as root before the start and removes it after the stop.

## Risks / Trade-offs

- [A command thread that panics] → The loop does not see it, and the clock
  runs on. Commands stop working until a restart. The socket code has no
  `unwrap` on a runtime path.
- [A blocked loop, for example in a slow present] → The socket thread waits
  2 s for the reply, then sends `error: the daemon did not answer in 2 s.`
- [Two threads more on a single core] → Both block in the kernel. The
  on-device scenario checks that each uses less than 1% of the core.
- [The signal wait moves from a handler pipe to a thread] → The SIGTERM and
  SIGINT CLI tests stay the same and must pass.

## Migration Plan

1. Deploy as usual. The unit change makes `/run/gameclockd`.
2. `sudo gameclockd ctl status` on the Pi checks the socket.
3. Rollback: start the legacy clock, as before. The socket has no other user.

## Open Questions

- The web control will send the same `Command` values from inside the
  daemon. Whether it uses the channel or a direct call can wait for
  `web-control`.
