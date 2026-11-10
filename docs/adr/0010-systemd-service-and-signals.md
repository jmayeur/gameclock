# ADR-0010: Run as a root systemd service with signal-driven shutdown

- **Status:** Accepted. The signal wait is changed by ADR-0013.
- **Date:** 2026-11-09
- **Related:** ADR-0004, ADR-0006, ADR-0007, ADR-0009. OpenSpec change
  `panel-bring-up`.

## Context

The daemon has to run unattended on the Pi and survive crashes. It also has to
coexist with the legacy clock (`clock.svc.service`) while the new stack is
still only a test pattern. Both drive the same GPIO pins, so they must never run at once.

hzeller needs root to map GPIO memory, then by default drops privileges to the
`daemon` user once it is initialized. The public repo can't contain device
addresses (`CLAUDE.md`).

ADR-0006 requires an idle daemon to be blocked, not polling. The only event the
bring-up daemon waits for is "stop". Rust's standard library has no signal API,
and every crate except the driver crate denies `unsafe_code`, so `gameclockd`
can't call `sigwait` itself.

## Decision

- **systemd unit** `deploy/gameclockd.service`: runs `/usr/local/bin/gameclockd`
  as root (no `User=`) and leaves hzeller's privilege drop on.
  `Restart=on-failure`. Stopped with systemd's default SIGTERM.
- **Mutual exclusion with the legacy clock** through
  `Conflicts=clock.svc.service`: starting either one stops the other, and
  rolling back is one `systemctl start`.
- **Boot enablement is explicit.** The deploy script installs and restarts the
  service. It doesn't enable it at boot unless asked (`--enable`). Then it
  also disables the legacy clock, so exactly one is enabled. `--rollback`
  reverses it.
- **Driver flags come from** `EnvironmentFile=-/etc/default/gameclockd`
  (`GAMECLOCKD_ARGS=...`), so tuning on the device needs no rebuild (ADR-0009).
- **Deploy is `scp` + `ssh` + `sudo`** to a host named only by the
  `GAMECLOCK_HOST` environment variable. Without it, the script refuses to run.
- **Signals:** `gameclockd` registers SIGTERM and SIGINT with `signal-hook`
  and blocks on `Signals::forever()` on the main thread. That is a blocking
  read on signal-hook's self-pipe: no timer and no extra thread. It registers
  before the driver starts, so a stop request during initialization isn't
  lost. On a signal, it drops the panel (blanking it) and exits 0.
- **New dependency:** `signal-hook` (MIT OR Apache-2.0, used under MIT), which
  brings in `signal-hook-registry`, `libc`, and `errno` (all MIT OR
  Apache-2.0).

## Consequences

- Install and rollback are each one command, and the legacy clock stays the
  boot service until a real clock replaces it.
- The daemon starts as root. Its exposure is limited to driver init, after
  which hzeller drops to `daemon`. Revisit this if the daemon gains network
  input (`command-input`, `web-control`).
- The Pi user needs passwordless `sudo` for deploys (the default `pi` user has
  it).
- A later change may add input sources. Then the main thread's wait has to
  become a wait on several sources (for example `poll` on the signal pipe plus
  a socket). signal-hook supports that.

## Alternatives considered

- **Run as an unprivileged user with GPIO group access.** hzeller's timing
  paths need `/dev/mem`, not just `/dev/gpiomem`, so this would lose the
  hardware-pulse option.
- **Disable the legacy unit on every deploy.** Makes rollback a multi-step
  manual job while the new daemon only shows a test pattern.
- **`ctrlc` crate.** Spawns a thread to handle signals. More moving parts than
  a blocking wait on the main thread.
- **`unsafe` `sigwait` in `gameclockd`.** Breaks the `unsafe` confinement rule.
- **`tokio` signal handling.** A full async runtime for one blocking wait is
  far over budget on a Pi Zero.
