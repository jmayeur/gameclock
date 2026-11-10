# ADR-0016: Command input over a Unix socket, and an event channel for the loop

- **Status:** Accepted
- **Date:** 2026-11-10
- **Related:** ADR-0006, ADR-0010, ADR-0013. Changes the deadline wait of
  ADR-0013. OpenSpec change `command-input`.

## Context

Messages, alerts, and the web control need a way to send a command to the
running daemon and to get an answer. The legacy clock reads a semaphore file
that a Node server writes.

ADR-0013 waits in one read on the signal pipe, with a timeout until the
deadline. That read can wait for one file descriptor only. ADR-0013 says
that `command-input` must decide how the loop waits for more sources. A wait
on many file descriptors needs `poll(2)`, which needs `unsafe` code or a new
dependency.

## Decision

- **A Unix stream socket** at `/run/gameclockd/control.sock`, for root only.
  The daemon makes it before the driver drops root privileges. The unit makes
  the directory (`RuntimeDirectory=gameclockd`, mode 0700).
- **A line protocol.** One connection carries one command line of 256 bytes
  or fewer, and gets one reply line that starts with `ok` or `error: `.
- **A client in the same binary:** `gameclockd ctl <command>`.
- **An event channel replaces the signal pipe.** The loop waits on a
  `std::sync::mpsc` receiver, with `recv_timeout` until the deadline. Two
  threads send events to it:
  - A signal thread with `signal_hook::iterator::Signals` sends a shutdown
    event.
  - A socket thread accepts one connection at a time. It sends each command
    with a reply sender, and it writes the reply.
- **The clock does not depend on the socket.** If the socket cannot be made,
  the daemon writes a warning and runs with no command input.

## Consequences

- The wait needs no `unsafe` code and no new dependency. Each new input
  source is one more thread that sends events.
- The process has two more threads. Both block in the kernel when no event
  comes, so the idle CPU stays near zero. Each thread has a 64 KiB stack.
- A slow client cannot delay the display: the socket thread limits each read
  to 1 s, and the loop does not read the socket.
- Only root can send commands. A client for other users needs a later
  decision.
- The web control can send the same commands from inside the daemon, with no
  socket.

## Alternatives considered

- **HTTP in the daemon now.** It needs an HTTP dependency and a decision about
  access on the LAN. That is the scope of `web-control`.
- **The legacy semaphore file.** It keeps the Node server, needs a file watch,
  and has no replies.
- **`poll(2)` through `rustix` or `libc`.** A new dependency or `unsafe` code
  for two file descriptors.
- **One more byte on the signal pipe for each command.** The loop then counts
  bytes to tell a signal from a command. A race between the two is easy to
  get wrong.
