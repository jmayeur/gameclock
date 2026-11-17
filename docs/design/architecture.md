# Architecture

> The state after `web-control`. Sections marked **Planned** give the agreed
> direction (ADRs 0004–0006). They become current as OpenSpec changes merge.

## Shape

```
                ┌──────────────── gameclockd (binary, on the Pi) ────────────────┐
  wall clock ──▶│                                                                │
  (tz-aware)    │   event loop: sleep until next visual change or event          │
                │        │                ▲ event channel                        │
  commands  ───▶│ control thread ─────────┤                                      │
  (Unix socket) │ web thread ─────────────┤                                      │
  web page  ───▶│ signals thread ─────────┘                                      │
  (HTTP, :80)   │                                                                │
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
  time-of-day `schedule` (ADR-0014), and the `message` parts (ADR-0017). It
  also holds the `effect`, `alert`, and `board` parts (ADR-0018), and the
  `Tables` that the config gives (ADR-0019). Time comes in as a `jiff::Zoned` (ADR-0013).
- **`gameclockd`** (exists): the daemon. It checks arguments, handles signals,
  reads the time zone, selects the panel by build feature, and runs the event
  loop below. It also runs the control socket and the `ctl` client
  (ADR-0016), and the web page (ADR-0020). Its `config` module reads the config file with serde and
  `toml` (ADR-0019).
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
  `put_back` puts an interrupted message first, also above the limit.
  `ScrollPath` holds the scroll math. Messages and alerts both use it.
- **`effect`** (`gameclock_core::effect`, ADR-0018): the color wheel and the
  contrast color. The wheel moves 3 of its 765 positions each 50 ms
  (`WHEEL_STEP`). `contrast` gives black or white for a background color.
- **`Alert`, `Alerts`, `AlertScene`** (`gameclock_core::alert`, ADR-0018,
  ADR-0019): an `Alert` is data. It has a name, a background (black or the
  color wheel), a text, and a text style (font, baseline, color, still or
  scroll). It also has a duration, a `Priority`, and a `ClearRule`.
  `Alert::validate` checks the limits. `Alerts` is a table of alerts with
  unique names, shared as `Rc<Alert>`. `Alerts::legacy()` holds `panic`,
  `bedtime`, and `bed-flash`. `find` looks up a name, and `names()` lists the
  names for a reply. `AlertScene` renders any alert. It finishes at the end of its
  duration, or after its last pass if that is earlier. Its brightness comes
  from the legacy schedule.
- **`Trigger`, `Triggers`** (`gameclock_core::alert`, ADR-0018): a trigger
  rule starts an alert each `every` from `from` to `to`, on its weekdays, in
  local time. `Trigger::new` checks the window and the period.
  `Trigger::legacy(&alerts)` is the bedtime warning, with only the rules whose
  alert is in the table. `Triggers` keeps
  the next fire time of each rule. `due(now)` gives each alert that is due,
  one for each rule, and `next()` gives the earliest next fire time.
- **`Board`** (`gameclock_core::board`, ADR-0018): the message queue and the
  item on the panel (nothing, a message, or an alert). It changes the
  compositor stack, which holds at most one foreground scene. An alert starts
  only above a lower priority: the base scene, `bed-flash`, a message,
  `bedtime`, then `panic`. An interrupted message goes back to the front of
  the queue. `clear` keeps an alert whose rule is `Kept`. `set_schedule`
  gives the schedule for new scenes.
- **`Tables`** (`gameclock_core::tables`, ADR-0019): the schedule, the alert
  table, and the trigger rules. `Tables::legacy()` is the built-in set.
- **`config`** (`gameclockd`, ADR-0019): `parse` turns the TOML text into
  `Tables`. serde types with `deny_unknown_fields` mirror the file, and each
  value keeps its byte range. The core types check the ranges. A
  `ConfigError` gives `<path>:<line>:<column>: <message>`. A section that is
  not in the file uses its built-in table. `load` reads the file. A missing
  file at the default path gives the built-in tables and a note.
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
  `Command` (`clock`, `test-pattern`, `status`, `message`, `clear`, `alert`,
  or `reload`) or a `CommandError`. The first word selects the command.
  `message` reads its `--passes` and `--color` options, then takes the rest
  of the line as the text. `alert` takes a name or `stop`. The loop looks up
  the name, because `reload` can change the alert table. The reply text comes from pure functions.
- **`control`** (`gameclockd`): `listen` makes the socket (mode 0600, a
  stale file is replaced, a live socket is not taken). The `control` thread
  accepts one connection at a time. It gives each client 1 s for its line and
  the loop 2 s for the reply. `send_command` is the client of
  `gameclockd ctl`.
- **`http`** (`gameclockd`, ADR-0020): a pure parser for a small HTTP/1.1
  subset, and `Response`. A head over 8 KiB, a body over 1 KiB, a post with
  no `Content-Length`, and a bad request line are refused with a status.
  Each response closes the connection.
- **`web`** (`gameclockd`, ADR-0020): `route` gives the response to a
  request. It sends the same `Command`s as the socket through an `ask`
  function. The page asks the loop for the status and for the alert names
  (`Command::AlertNames`, which the socket parser cannot make). A post with
  an `Origin` of a different host gets 403. The `web` thread accepts one
  connection at a time. It gives each client 2 s for the whole request and
  2 s to read the response. Then it discards the rest of the request for
  500 ms at most.

## Daemon lifecycle

`gameclockd` (ADR-0009, ADR-0010):

1. **Arguments.** `parse_args` gives an `Invocation`. If the first argument
   is `ctl`, the binary is the client. It joins the words after its options
   with one space. It sends them as one command, writes the
   reply, and exits: 0 for `ok`, 1 for an error, and 2 for a usage error.
   Otherwise it accepts hzeller `--led-*` flags, `--test-pattern`,
   `--control-socket=<path>`, `--config=<path>`, and `--http=<address>` or
   `--http=off`. It gives the mode, the socket path, the config path, the
   web address (default `0.0.0.0:80`), and the driver flags in their order. Any
   other argument exits 2 before the daemon touches the panel.
2. **Config.** `config::load` reads the config file (default
   `/etc/gameclockd/config.toml`). An error writes the path, the line, the
   column, and the problem, and exits 2. A missing file at the default path
   gives the built-in tables, with a note on stderr. A missing file from
   `--config` is an error.
3. **Signals and time.** The daemon registers SIGTERM and SIGINT with the
   `signal-hook` iterator before the driver starts. The `signals` thread
   sends one shutdown event on the event channel (ADR-0016). Then the daemon
   reads the system time zone once and names it on stderr. If no time zone
   is available, it uses UTC and writes a warning. It also names the base
   scene: the clock, or the test pattern with `--test-pattern`.
4. **Control socket.** While the process is still root, the daemon listens on
   the socket (default `/run/gameclockd/control.sock`) and starts the
   `control` thread. If it cannot, it writes a warning and runs with no
   command input.
5. **Web page.** While the process is still root, the daemon listens for
   HTTP on the web address, starts the `web` thread, and writes the address.
   If it cannot, it writes a warning and runs with no web page (ADR-0020).
6. **Panel, selected at build time.** There are three builds:
   - `hzeller` (the Pi build, `scripts/build-pi.sh`): `HzellerPanel::new`
     receives the flags.
   - `simulator` (`cargo sim`): `TerminalPanel` draws on stdout.
   - Neither: `LogPanel` writes each present to stderr.

   Without `hzeller`, the daemon ignores `--led-*` flags with a notice. A
   build with `hzeller` and `simulator` together stops with a compile error.
7. **Run.** `run` puts the base scene in a panel-sized `Compositor`, next to
   a `Board` and the `Triggers` of the config tables. Then it loops:
   1. It gets the time from the clock. The board removes the finished
      scenes. If nothing shows, it starts the first message of the queue as
      a `Scroll` at that time. So the next message starts at the same update
      as the finish of the last one.
   2. For each trigger rule that is due, it gives the alert to the board. A
      refused trigger changes nothing.
   3. It updates the compositor. If the update gives a frame, it presents
      the frame at the brightness that the update gives.
   4. It waits until the earlier of the compositor's deadline and the next
      trigger time, or an event. With neither, the wait has no timeout.
   5. For a command, it does the command and sends the reply. Each change
      shows on the panel before the reply.
      - `clock` and `test-pattern` replace the base scene.
      - `status` gives the mode, the zone name, the last brightness, the
        number of messages, and the alert on the panel.
      - `message` starts the message, or puts it in the queue. A full queue
        gives an error reply.
      - `clear` empties the queue and removes the message, or an alert that
        its rule permits.
      - `alert <name>` starts the alert, or gives an error reply if the item
        on the panel has the same or a higher priority. `alert stop` removes
        the alert.
      - `reload` reads the config file again. A correct file gives new
        tables. New scenes use the new schedule and alerts. The triggers
        start again from now, and the clock gets the new schedule. The scene
        on the panel continues. A file with an error gives an error reply,
        and the tables do not change.
8. **Shutdown.** On a signal, the panel drops, the daemon removes the socket
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

The service binds port 80 as root, before the driver drops the privileges.
The deploy stops the legacy nginx and PM2, which hold port 80 and run the
legacy Node page. The rollback starts them again. PM2 needs about 5 minutes
on the Pi, so the rollback starts it with no wait (ADR-0020).

Since 2026-11-16, `gameclockd` is the boot service on the Pi. The legacy
clock, nginx, and PM2 are deprecated and disabled at boot. Their files stay
on the Pi for the rollback.

## Data flow

1. The loop wakes at a deadline: the next visual change or the next trigger
   time. It also wakes on a command from the socket or the web page, and on
   a stop signal.
2. It gives the injected time to the compositor. The compositor renders the
   top scene into a `Frame`.
3. If the frame or the brightness is different from the last presented
   ones, the loop calls `panel.present(frame, brightness)`.
4. It gets the next deadline from the compositor and the triggers, and
   waits.

The wait uses a duration. When the system clock steps (for example at the
first NTP sync), the wake can be late by up to one interval (ADR-0013).

## Scene model

A **scene** is a self-contained renderer with its own state. It reports when
it next changes and when it finishes. It also gives its brightness
(ADR-0012, ADR-0014). The daemon has one base scene: the clock, or the
heartbeat in the test-pattern mode. A message or an alert is a foreground
scene. At most one foreground scene is on the stack. The board decides which
one, and the other messages wait in its queue (ADR-0017, ADR-0018).

The **compositor** decides what is visible: the base scene, or the newest
foreground scene. Scenes never refer to each other.

Alerts are data of one type, and one alert scene renders each of them
(ADR-0018). The priorities are in the board, not in the compositor. Layers
and transitions are not in the compositor yet (see `roadmap.md`).
