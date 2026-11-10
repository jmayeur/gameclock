# Design

## Context

See `proposal.md` for the reasons. The current state:

- `gameclock-core` has `Frame`, `Panel`, `Brightness`, and `test_pattern`. It
  has one dependency, `embedded-graphics`.
- `gameclockd::run` renders the test pattern once, presents it, and calls a
  `wait_for_shutdown` closure. `main.rs` gives a closure that blocks on
  `Signals::forever()` (ADR-0010).
- The Pi Zero W has no real-time clock. At boot, the system time is wrong
  until the first NTP sync. Then it steps forward.
- `Frame` already implements `PartialEq`, so a frame comparison is one `==`.

## Goals / Non-Goals

**Goals:**

- A scene contract that the clock, the message board, and alerts can use with
  no change to the compositor.
- A loop that tests can drive with injected time and injected wakes, with no
  real sleep.
- No extra thread and no new `unsafe` code.

**Non-Goals:**

- A way to add foreground scenes from outside the process. The compositor has
  the method, but only tests call it.
- Brightness from scenes. The loop presents at `Brightness::MAX`, as now.

## Decisions

### D1. The scene contract is an object-safe trait (ADR-0012)

```rust
pub trait Scene {
    fn render(&mut self, now: &Zoned, frame: &mut Frame);
    fn next_change(&self, now: &Zoned) -> Option<Timestamp>;
    fn is_finished(&self, now: &Zoned) -> bool;
}
```

- `render` takes `&mut self`, so a scene can keep state (for example a scroll
  position). It returns nothing, because drawing into a `Frame` cannot fail.
- `next_change` gives `None` for a static image.
- The compositor holds `Box<dyn Scene>`. A box is allocated only when a scene
  is added, not for each frame. The loop calls each method once for each
  wake, so dynamic dispatch costs nothing that we can measure.
- **Alternative:** an `enum` of all scenes. It gives static dispatch, but each
  new scene then changes the compositor. That is against the goal.
- **Alternative:** a scene returns its own frame. Then each scene needs a
  frame buffer, and the compositor cannot clear and compare in one place.

### D2. The compositor owns two frames and compares them

- `Compositor::new(base, width, height)` allocates two frames: `work` and
  `shown`. `push(scene)` adds a foreground scene.
- `update(&now) -> Option<&Frame>` does these steps:
  1. Removes each finished foreground scene (`Vec::retain`, no allocation).
  2. Clears `work` to black.
  3. Renders the top scene into `work`.
  4. Compares `work` with `shown`. If they are equal, returns `None`.
  5. Otherwise swaps the two frames (`core::mem::swap`) and returns
     `Some(&shown)`.
- The first update always gives a frame, because a flag records that nothing
  was shown yet. A black frame then also presents at startup.
- `deadline(&now) -> Option<Timestamp>` gives the top scene's next change,
  delayed to at least `now + 33 ms` (D3).
- The base scene is a separate field, so the compositor cannot remove it.

### D3. The render-rate cap is in the compositor

`deadline` gives `max(next_change, now + MIN_INTERVAL)`, with
`MIN_INTERVAL = 33 ms`. This puts the 30 fps cap of ADR-0006 in one place. A
scene with a bug that reports a past time cannot make the loop spin.

### D4. Time is a jiff `Zoned`. Deadlines are `Timestamp`s (ADR-0013)

- Scenes get `&jiff::Zoned`: an instant plus a time zone. The clock scene
  needs the local civil time and the zone's transitions. The heartbeat uses
  only the instant.
- Deadlines are `jiff::Timestamp` values: instants on the wall clock.
- `gameclock-core` depends on `jiff` with `default-features = false` and
  `features = ["std"]`. The core does not read the system zone.
- `gameclockd` adds `tz-system` and `tzdb-zoneinfo`. It reads
  `/usr/share/zoneinfo` on the Pi (Buster has `tzdata`). It does not bundle a
  zone database, so the binary stays small.
- **Alternative:** `chrono` and `chrono-tz`. `chrono-tz` compiles the whole
  database into the binary. jiff reads the system database.
- **Alternative:** `std::time` only. It has no time zones and no civil time.

### D5. The daemon waits on the signal pipe with a read timeout (ADR-0013)

- `main.rs` creates a `UnixStream::pair()`. It registers SIGTERM and SIGINT
  with `signal_hook::low_level::pipe::register`, one clone of the write end
  for each signal.
- `SignalWait` holds the read end and implements a `Wait` trait:
  `wait(deadline: Option<Timestamp>, now: Timestamp) -> Wake`.
  - With no deadline, it clears the timeout and blocks in `read`.
  - With a deadline at or before `now`, it gives `Wake::Deadline` at once.
  - Otherwise it sets `set_read_timeout(deadline - now)` and reads.
  - A byte read gives `Wake::Shutdown`. `WouldBlock`, `TimedOut`, and
    `Interrupted` give `Wake::Deadline`. The loop then checks the time again.
  - End of file or another error gives `Wake::Shutdown`, so the loop cannot
    spin.
- A signal that arrives during a render writes a byte to the pipe. The next
  wait reads it at once, so the signal is not lost.
- This uses only `std` and `signal-hook`, which we already have. It adds no
  thread and no `unsafe` code.
- **Alternative:** a signal thread that sends on an `mpsc` channel, and
  `recv_timeout` in the loop. It is also simple, but it adds a thread now
  that nothing else needs. `command-input` can choose its own model.
- **Alternative:** `poll(2)` through `libc` or `rustix`. It needs `unsafe` or
  a new dependency for one file descriptor.

### D6. `run` takes a panel, a base scene, a clock, and a waiter

```rust
pub fn run<P: Panel>(panel: &mut P, base: Box<dyn Scene>,
                     clock: &impl Clock, wait: &mut impl Wait)
    -> Result<(), RunError<P::Error>>
```

The loop:

```text
loop {
    now = clock.now()
    if let Some(frame) = compositor.update(&now) { panel.present(frame, MAX)? }
    match wait.wait(compositor.deadline(&now), now.timestamp()) {
        Deadline => continue,
        Shutdown => return Ok(()),
    }
}
```

- `Clock::now() -> Zoned`. `SystemClock` holds the `TimeZone` and gives
  `Timestamp::now().to_zoned(tz.clone())`. A `TimeZone` clone only changes a
  reference count.
- Tests use a `FakeClock` and a `ScriptedWait` that share the current time.
  Each scripted step moves the time to the deadline, moves it to a given
  time, or gives `Shutdown`. The waiter records each deadline that it gets.
- An early wake (D5, or a clock that is set back) renders again. The image is
  the same, so nothing is presented, and the loop waits again.

### D7. The heartbeat is a stateless scene in the core

- `heartbeat::Heartbeat` renders `test_pattern::render`, then sets one pixel.
  The column is `3 + second.rem_euclid(width - 6)` on row 1.
- `next_change` gives the next whole second after `now`. `is_finished` is
  always `false`.
- It has no state, so the image depends only on the time and the frame size.
- `clock-scene` replaces it as the base scene. Then we remove it, or we keep
  it as a diagnostic mode. That change decides.

### D8. The time zone is read once, with a UTC fallback

- At startup, `main.rs` calls `TimeZone::try_system()`. A library function
  turns the result into a `TimeZone` and one stderr line:
  - `gameclockd: the time zone is America/Los_Angeles.`
  - `gameclockd: the time zone has no name. The daemon uses it.` when the
    zone has no IANA name.
  - `gameclockd: the system time zone is not available. The daemon uses UTC.`
- jiff reads `TZ` first, so the integration test sets `TZ` to check the name.

### D9. A test counts heap allocations (ADR-0012)

- An integration test in `gameclock-core` uses the dev-dependency
  `allocation-counter` (MIT OR Apache-2.0, used under MIT). It wraps the 1000
  updates in `allocation_counter::measure` and asserts zero allocations.
- The crate installs the counting allocator itself, so our test needs no
  `unsafe` code.
- It is a dev-dependency only. The Pi binary does not contain it.

## Risks / Trade-offs

- [The clock steps forward at the first NTP sync] → The wait uses a
  duration. So it ends late by up to one interval: 1 s for the heartbeat. The
  next render uses the new time. A later change can add a wake on a clock step
  (`timerfd` with `TFD_TIMER_CANCEL_ON_SET`). It does so only if the clock
  scene shows the need.
- [The kernel ends the read timeout a little early] → The scene sees a time
  just before its change. The render gives the same image, and the 33 ms cap
  delays the next wake. The marker then moves up to 33 ms late. That is not
  visible.
- [A heartbeat present each second costs CPU on the Pi] → We measure it with
  `scripts/measure.sh` (proposal, "Expected runtime impact"). If the main
  thread uses 1% or more of one core, we stop and find the cause before the
  merge.
- [jiff makes the Pi binary larger] → We record the binary size before and
  after in `measurements.md`.
- [`allocation-counter` replaces the global allocator in the test binary] →
  Only `gameclock-core`'s tests link it. The allocator counts and then calls
  the system allocator.

## Migration Plan

- Deploy with `scripts/deploy.sh`, as before. The service unit does not
  change.
- Rollback: `scripts/deploy.sh --rollback`, or start the legacy clock. The
  `Conflicts=` line stops `gameclockd`.

## Open Questions

- After `clock-scene`, do we keep the heartbeat as a diagnostic mode? The
  `clock-scene` change decides. It does not change this design.
