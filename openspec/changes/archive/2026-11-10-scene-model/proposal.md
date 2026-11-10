# Proposal

## Why

Now the daemon presents the test pattern once and then waits for a stop
signal. It has no way to show an image that changes with time. The clock,
messages, and alerts all need one contract for scenes and one place that
decides what is visible. They also need a loop that wakes only when the image
changes (ADR-0006). Each later roadmap change builds on these three parts.

## What Changes

- Add a **scene contract** to `gameclock-core`. A scene renders a frame for an
  injected time. It tells when its image changes next and when it is
  finished.
- Add a **compositor** to `gameclock-core`. It holds one base scene and a
  stack of foreground scenes. It shows the top scene, removes foreground
  scenes when they finish, and presents a frame only when it is different
  from the last one.
- The compositor caps the render rate at 30 per second. A scene that asks for
  a faster change gets the next slot that is 33 ms or more after the render.
- Replace the daemon's single present with an **event-driven loop**. The loop
  sleeps until the next scene change or a stop signal. With no next change, it
  blocks with no timeout.
- The daemon **injects time**. It reads the system time zone once at startup.
  If the time zone is not available, it uses UTC and writes a warning.
- Add a **heartbeat scene**: the test pattern plus one white marker that moves
  one step each second in the black margin under the top border. It is the
  daemon's base scene until `clock-scene` replaces it. It makes the loop
  visible on the panel and in `cargo sim`.
- **BREAKING** (behavior): the daemon no longer presents only once. With the
  heartbeat, it wakes and presents once each second.

## Capabilities

### New Capabilities

- `scene-model`: the scene contract and the compositor. This covers which
  scene is visible, when foreground scenes go away, the change deadline, the
  render-rate cap, and frame comparison.
- `heartbeat-scene`: the temporary heartbeat image and its once-a-second
  change.

### Modified Capabilities

- `daemon-lifecycle`:
  - "Startup presents the test pattern once" becomes "Startup presents the
    base scene at once".
  - "The daemon blocks between deadlines" replaces "Idle means blocked, not
    polling". The daemon now also wakes at scene deadlines.
  - New requirements cover the event loop and the time zone at startup.

## Relies on

- ADR-0005: the core is pure, and panels only blit. Scenes render into a
  `Frame`.
- ADR-0006: render on change, the 30 fps cap, and no heap allocation for each
  frame.
- ADR-0010: signal-driven shutdown through `signal-hook`.
- ADR-0011: messages and comments are in strict STE.

## New ADRs required

- **ADR-0012: Scenes and the compositor.** A new boundary: the `Scene` trait
  and the compositor in `gameclock-core`. It also adds the dev-dependency
  `allocation-counter` to test the no-allocation rule.
- **ADR-0013: Time with jiff, and the deadline wait.** A new dependency:
  `jiff` (Unlicense OR MIT). The daemon injects a time-zone-aware time.
  Deadlines are wall-clock timestamps. The loop waits on the signal-hook pipe
  with a read timeout, so no extra thread is necessary. This changes the
  signal wait in ADR-0010, so ADR-0010 gets a status note.

## Expected runtime impact on the Pi

- **CPU:** the heartbeat adds one wake, one render, and one present each
  second. We expect the main thread to use less than 1% of one core. The
  refresh rate must stay within 2% of the bring-up value (147 Hz with default
  flags).
- **Memory:** `jiff` and the time-zone data add to RSS. We expect less than
  1 MiB more than the bring-up value.
- **How we measure:** `scripts/measure.sh` over 60 s on the Pi Zero W. It
  gives per-thread CPU %, voluntary context switches, and RSS. We also read
  `--led-show-refresh`. The change records the numbers in `measurements.md`.

## Out of scope

- The clock face, colors, and the brightness schedule (`clock-scene`).
- A way to add foreground scenes at runtime (`command-input`). Only tests add
  them in this change.
- Priorities, layers, overlays, and transitions between scenes.
- A wake when the system clock is set (for example at the first NTP sync).
  The loop corrects itself at the next deadline.
- A reload of the time zone while the daemon runs. A change needs a restart.

## Impact

- **New:** `gameclock-core` modules for scenes, the compositor, and the
  heartbeat. A time source and a deadline wait in `gameclockd`.
- **Changed:** `gameclockd`'s `run` and `main.rs`. The workspace and crate
  `Cargo.toml` files (`jiff`, `allocation-counter`). `Cargo.lock`.
- **Docs:** ADR-0012, ADR-0013, the ADR-0010 status note, the ADR index,
  `architecture.md`, `glossary.md`, `roadmap.md`, and `CHANGELOG.md`.
