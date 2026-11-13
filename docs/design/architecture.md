# Architecture

> The state after `message-board`. Sections marked **Planned** give the agreed
> direction (ADRs 0004–0006). They become current as OpenSpec changes merge.

## Shape

```
                ┌──────────────── gameclockd (binary, on the Pi) ────────────────┐
  wall clock ──▶│                                                                │
  (tz-aware)    │   event loop: sleep until next visual change or event          │
                │        │                ▲ event channel                        │
  commands  ───▶│ control thread ─────────┤                                      │
  (Unix socket) │ signals thread ─────────┘                                      │
  signals   ───▶│        ▼                                                       │
                │   gameclock-core (pure) ──renders──▶ Frame ──changed?──▶ Panel ──┼──▶ LED matrix
                │   scenes · schedule ·                (64×32 RGB)          .present│
                │   composition                                                  │
                └────────────────────────────────────────────────────────────────┘
```

## Crates

- **`gameclock-core`** (exists): pure logic. It has no hardware, I/O, threads,
  or wall clock (ADR-0005). It holds `Frame`, the `Panel` trait,
  `Brightness`, the diagnostic `test_pattern`, the `Scene` trait, the
  `Compositor`, the `Clock` and `Heartbeat` scenes (ADR-0012), the
  time-of-day `schedule` (ADR-0014), and the `message` parts (ADR-0017). Time comes in as a `jiff::Zoned`
  (ADR-0013).
- **`gameclockd`** (exists): the daemon. It checks arguments, handles signals,
  reads the time zone, selects the panel by build feature, and runs the event
  loop below. It also runs the control socket and the `ctl` client
  (ADR-0016). Planned: config.
- **`rgbmatrix-sys`** (exists): the LED driver, and the only crate with
  `unsafe` (ADR-0004, ADR-0009).
  - Always: the Pi driver defaults (`options`: 64×32, chain 1, parallel 1,
    and the `adafruit-hat-pwm` mapping, ADR-0015), the conversion from a frame to
    the driver buffer (`convert`), and `Error`. Tests run on any host.
  - With the `link` feature: `build.rs` compiles hzeller from the
    `vendor/rpi-rgb-led-matrix` submodule with zig. The crate adds the
    hand-written FFI (`ffi`), a build-time struct layout check, and
    `HzellerPanel`.
- **`TerminalPanel`** (exists, `gameclockd::sim`, `simulator` feature): shows
  frames in a 24-bit color terminal on a dev machine. A window is not planned.

## Key types

- **`Frame`**: the buffer that each scene draws into. Drawing outside the
  frame is clipped. `pixels()` gives the row-major buffer that a panel blits.
  The daemon compares frames to decide whether to present (ADR-0006).
- **`Panel`**: `dimensions()` and `present(&Frame, Brightness)`. A panel only
  blits pixels.
- **`Brightness`**: a percentage from 0 to 100.
- **`HzellerPanel`** (`rgbmatrix-sys`, `link` only): created from `--led-*`
  flags over the Pi defaults. `present` does these steps:
  1. Checks the frame's size.
  2. Converts the frame into a preallocated buffer.
  3. Sets the driver brightness.
  4. Copies the buffer to an offscreen canvas in one call.
  5. Swaps the canvas in on vsync.

  When `HzellerPanel` drops, hzeller stops its refresh thread and blanks the
  panel.
- **`TerminalPanel`** (`gameclockd`, `simulator` only): writes each frame to
  a terminal. Each cell is `▀`, with the upper pixel as the foreground color
  and the lower pixel as the background color. A 64×32 frame uses 64 columns
  and 16 lines. It scales colors by the brightness. When it drops, it resets
  the colors and shows the cursor again.
- **`test_pattern::render`**: draws the diagnostic test pattern into any
  `Rgb888` `DrawTarget`, usually a `Frame`. The result depends only on the
  target's size.
- **`Scene`** (ADR-0012): `render(now, frame)` draws into a black frame.
  `next_change(now)` gives the next time the image can change, or `None`.
  `is_finished(now)` tells when the scene has nothing more to show.
  `brightness(now)` gives the panel brightness, full by default (ADR-0014).
- **`Compositor`** (ADR-0012): one base scene and a stack of foreground
  scenes. `remove_finished(now)` removes the finished foreground scenes with
  no render. `clear_foreground()` removes all of them. `update(now)` removes
  finished foreground scenes and renders the
  top scene into a preallocated frame. It gives the frame and the top
  scene's brightness only if one of them differs from the last ones. `deadline(now)` gives the top scene's next change, at
  least 33 ms after `now` (30 renders each second at most). Updates allocate
  no heap memory. A test in `gameclock-core/tests/no_alloc.rs` checks this.
- **`Clock`** (`gameclock_core::clock`): the base scene. It shows the local
  time as `09:05PM` in `FONT_7X13_BOLD` at x=8, baseline y=20. Its color and
  brightness come from a `Schedule`. Its next change is the next whole
  minute. `render` formats the text into a stack buffer, so it allocates no
  heap memory.
- **`Schedule`** (`gameclock_core::schedule`, ADR-0014): a list of `Rule`s
  and a default `Style` (color and brightness). A rule has a start and an end
  `TimeOfDay` in whole minutes and a `Weekdays` set. The first rule that
  matches the local time gives the style. `Schedule::legacy()` is the legacy
  table.
- **`Message`, `Scroll`, `Queue`** (`gameclock_core::message`, ADR-0017): a
  `Message` has a text of 1 to 64 characters, a pass count from 1 to 10
  (default 4), and a color (default green `(0,150,0)`). `Scroll` is the
  foreground scene of one message. It draws the text in `FONT_7X13_BOLD` at
  baseline y=20. The x position comes from the time since its start. Each
  40 ms (`STEP`), the text moves 1 pixel to the left. Each pass goes from the
  right edge until the text is past the left edge. Its brightness comes from the legacy
  schedule. `Queue` holds at most 8 waiting messages, first in, first out.
- **`Heartbeat`**: the base scene in the test-pattern mode. It is the test
  pattern with one white pixel on row 1. The pixel moves one column each
  second. It uses full brightness.
- **`Clock`, `Wait`** (`gameclockd`, ADR-0013, ADR-0016): the injected time
  source and the wait. `SystemClock` gives the system time in the time zone
  from startup. `ChannelWait` waits on the event channel with a timeout until
  the deadline. It gives `Deadline`, `Shutdown`, or `Command`. Tests use a
  fake clock and a scripted wait.
- **`Event`, `Request`** (`gameclockd`, ADR-0016): an event on the channel is
  a shutdown or a command request. A request holds a `Command` and the sender
  for its reply line.
- **`command`** (`gameclockd`): `parse_command` turns a line into a
  `Command` (`clock`, `test-pattern`, `status`, `message`, or `clear`) or a
  `CommandError`. The first word selects the command. `message` reads its
  `--passes` and `--color` options, then takes the rest of the line as the
  text. The reply text comes from pure functions.
- **`control`** (`gameclockd`): `listen` makes the socket (mode 0600, a
  stale file is replaced, a live socket is not taken). The `control` thread
  accepts one connection at a time. It gives each client 1 s for its line and
  the loop 2 s for the reply. `send_command` is the client of
  `gameclockd ctl`.

## Daemon lifecycle

`gameclockd` (ADR-0009, ADR-0010):

1. **Arguments.** `parse_args` gives an `Invocation`. If the first argument
   is `ctl`, the binary is the client. It joins the words after its options
   with one space. It sends them as one command, writes the
   reply, and exits: 0 for `ok`, 1 for an error, and 2 for a usage error.
   Otherwise it accepts hzeller `--led-*` flags, `--test-pattern`, and
   `--control-socket=<path>`. It gives the mode, the socket path, and the
   driver flags in their order. Any other argument exits 2 before the daemon
   touches the panel.
2. **Signals and time.** The daemon registers SIGTERM and SIGINT with the
   `signal-hook` iterator before the driver starts. The `signals` thread
   sends one shutdown event on the event channel (ADR-0016). Then the daemon
   reads the system time zone once and names it on stderr. If no time zone
   is available, it uses UTC and writes a warning. It also names the base
   scene: the clock, or the test pattern with `--test-pattern`.
3. **Control socket.** While the process is still root, the daemon listens on
   the socket (default `/run/gameclockd/control.sock`) and starts the
   `control` thread. If it cannot, it writes a warning and runs with no
   command input.
4. **Panel, selected at build time.** There are three builds:
   - `hzeller` (the Pi build, `scripts/build-pi.sh`): `HzellerPanel::new`
     receives the flags.
   - `simulator` (`cargo sim`): `TerminalPanel` draws on stdout.
   - Neither: `LogPanel` writes each present to stderr.

   Without `hzeller`, the daemon ignores `--led-*` flags with a notice. A
   build with `hzeller` and `simulator` together stops with a compile error.
5. **Run.** `run` puts the base scene in a panel-sized `Compositor`, next to
   an empty message `Queue`. Then it loops:
   1. It gets the time from the clock and removes the finished scenes. If
      no message shows, it starts the first message of the queue as a
      `Scroll` at that time. Then it updates the compositor. So the next
      message starts at the same update as the finish of the last one.
   2. If the update gives a frame, it presents the frame at the brightness
      that the update gives.
   3. It waits until the compositor's deadline or an event. With no
      deadline, the wait has no timeout.
   4. For a command, it does the command and sends the reply. `clock` and
      `test-pattern` replace the base scene and present it before the reply.
      `status` gives the mode, the zone name, the last brightness, and the
      number of messages. `message` starts the message and presents it, or
      puts it in the queue. A full queue gives an error reply. `clear`
      empties the queue, removes the message scene, and presents the base
      scene before the reply.
6. **Shutdown.** On a signal, the panel drops, the daemon removes the socket
   file, and the process exits 0. hzeller stops its refresh thread and
   blanks the LEDs. A panel error exits 1 with the message on stderr.

On the Pi, the daemon runs as the `gameclockd` systemd service
(`deploy/gameclockd.service`, installed by `scripts/deploy.sh`). The service
conflicts with the legacy `clock.svc.service` (ADR-0010). It starts the
daemon with `--led-pwm-bits=8`, before the flags from
`/etc/default/gameclockd` (ADR-0015). systemd makes `/run/gameclockd`, root
only, for the socket and removes it at stop (ADR-0016). On the Pi, the
driver drops root privileges after the socket exists, so the daemon cannot
remove the socket file. The removal of the directory does that.

## Data flow

1. The loop wakes at a deadline (the next visual change), on a command, or
   on a stop signal.
2. It gives the injected time to the compositor. The compositor renders the
   top scene into a `Frame`.
3. If the frame or the brightness is different from the last presented
   ones, the loop calls `panel.present(frame, brightness)`.
4. It gets the next deadline from the compositor and waits.

The wait uses a duration. When the system clock steps (for example at the
first NTP sync), the wake can be late by up to one interval (ADR-0013).

## Scene model

A **scene** is a self-contained renderer with its own state. It reports when
it next changes and when it finishes. It also gives its brightness
(ADR-0012, ADR-0014). The daemon has one base scene: the clock, or the
heartbeat in the test-pattern mode. A message is a foreground scene. At most
one message is on the stack. The others wait in the queue of the loop
(ADR-0017).

The **compositor** decides what is visible: the base scene, or the newest
foreground scene. Scenes never refer to each other.

Planned: Panic and Bedtime become scenes. They will likely be configured
instances of one alert capability, not special cases. Priorities, layers, and transitions are not
in the compositor yet (see `roadmap.md`).
