# Glossary

- **Frame**: the in-memory 64×32 RGB buffer that all rendering goes into (an
  `embedded-graphics` `DrawTarget`).
- **Panel**: a thing that shows a `Frame` at a `Brightness`: the hzeller
  driver, a simulator, or a test recorder. Panels only blit pixels.
- **Test pattern**: the diagnostic image from `gameclock_core::test_pattern`:
  colored corner markers (orientation), a white border (missing edges), color
  bars (channel order), and a gray ramp (color depth). Not a scene.
- **LogPanel**: the `Panel` of the default host build. It only logs each
  present, so the daemon runs on any machine.
- **TerminalPanel** (simulator): the `Panel` of the `simulator` build. It draws
  frames in a terminal with 24-bit color. Run it with `cargo sim`.
- **Driver flags**: hzeller's `--led-*` command-line options (for example
  `--led-pwm-bits`, `--led-slowdown-gpio`, `--led-show-refresh`). `gameclockd`
  passes them to the driver over its Pi defaults (ADR-0009). On the Pi they
  live in `/etc/default/gameclockd`.
- **STE**: Simplified Technical English (ASD-STE100), the writing standard for
  all text in the repo (ADR-0011).
- **Scene**: a self-contained panel behavior (clock, message, alert). It
  renders frames from its own state and the injected time. It reports its
  next change and when it is finished (ADR-0012).
- **Compositor**: decides which scene is visible: the base scene, or the
  newest foreground scene. It is the only component that knows about more
  than one scene. Planned: priorities and transitions.
- **Base scene / foreground scene**: the compositor always shows the base
  scene when no foreground scene is on the stack. A foreground scene shows on
  top until it finishes. The compositor then removes it.
- **Deadline**: the time of the next render. The compositor gives the top
  scene's next change, at least 33 ms after the update. The daemon waits
  until the deadline or a stop signal.
- **Heartbeat**: the temporary base scene. It is the test pattern with one
  white pixel that moves one column each second on row 1.
- **Render on change**: the daemon presents a frame only when it differs from
  the last one, and sleeps until the next scheduled change (ADR-0006).
- **hzeller**: Henner Zeller's `rpi-rgb-led-matrix`, the GPL-2.0-only C++ driver
  that clocks pixels out to the panel and runs its own refresh thread.
- **Refresh thread**: hzeller's thread that continuously scans the panel. On the
  single-core Zero it competes with the app for CPU, and starving it causes
  flicker.
- **HAT / mapping**: the Adafruit RGB Matrix HAT and the GPIO pin mapping it
  needs (`adafruit-hat`).
- **Hardware pulse**: hzeller's use of the Pi's PWM peripheral for output-enable
  timing. Better image quality, but it conflicts with onboard audio (disabled on
  this Pi).
- **Semaphore file**: the legacy command channel (`msg.txt` holding
  `msg=<value>`).
