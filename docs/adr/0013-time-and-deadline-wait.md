# ADR-0013: Time with jiff, and the deadline wait

- **Status:** Accepted. The deadline wait is changed by ADR-0016.
- **Date:** 2026-11-10
- **Related:** ADR-0005, ADR-0006, ADR-0010, ADR-0012. OpenSpec change
  `scene-model`.

## Context

The core cannot read the clock (ADR-0005), so the daemon must inject time.
The clock scene needs the local civil time and the time-zone rules,
including daylight saving time. Rust's standard library has no time zones.

The loop must now wait for two things: a deadline and a stop signal.
ADR-0010 waits only for a signal, with `Signals::forever()`. That call has no
timeout.

The Pi Zero W has no real-time clock. Its time is wrong at boot until the
first NTP sync, and then it steps forward.

## Decision

- **New dependency: `jiff`** (Unlicense OR MIT, used under MIT). It brings in
  `jiff-core` (Unlicense OR MIT).
  - `gameclock-core` uses it with `default-features = false` and the `std`
    feature. The core does not read the system time zone.
  - `gameclockd` adds `tz-system` and `tzdb-zoneinfo`. On the Pi, jiff reads
    `/usr/share/zoneinfo`. The binary does not bundle a zone database.
- **Injected time.** Scenes get a `jiff::Zoned`: an instant and a time zone.
  Deadlines are `jiff::Timestamp` values on the wall clock.
- **The time zone is read once at startup** with `TimeZone::try_system()`.
  If it fails, the daemon uses UTC and writes a warning. A time zone change
  needs a restart.
- **The deadline wait.** `gameclockd` registers SIGTERM and SIGINT with
  `signal_hook::low_level::pipe::register` on one end of a
  `std::os::unix::net::UnixStream` pair. The loop reads the other end, with a
  read timeout until the deadline. A byte means stop. A timeout means the
  deadline came. With no deadline, the read has no timeout. This changes
  only the signal wait of ADR-0010.

## Consequences

- Scenes are tested with any instant and any time zone, with no real clock.
- The wait needs no extra thread, no new `unsafe` code, and no new
  dependency other than `jiff`.
- A signal that arrives during a render stays in the pipe. The next wait
  reads it at once.
- The wait uses a duration. When the clock steps, the wake can be late by up
  to one interval. The next render then uses the new time. If the clock scene
  needs a faster correction, a later change can add `timerfd` with
  `TFD_TIMER_CANCEL_ON_SET`.
- `command-input` adds more input sources. It must then decide how the loop
  waits on all of them, for example `poll` or a channel.

## Alternatives considered

- **`chrono` with `chrono-tz`.** `chrono-tz` compiles the whole zone database
  into the binary. jiff reads the system database.
- **`time` with a zone crate.** Its local-offset support is limited on
  multi-threaded Unix programs, and the hzeller driver runs a thread.
- **A signal thread that sends on an `mpsc` channel**, and `recv_timeout` in
  the loop. Also simple, but it adds a thread that nothing else needs now.
- **`poll(2)` through `libc` or `rustix`.** It needs `unsafe` code or a new
  dependency for one file descriptor.
