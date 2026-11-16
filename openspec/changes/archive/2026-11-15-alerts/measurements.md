# Measurements: alerts

Device: the same Pi Zero W, OS, HAT, and panel as in panel bring-up.
Date: 2026-11-15.

## Method

- Driver flags: the unit's `--led-pwm-bits=8` and the built-in
  `adafruit-hat-pwm` mapping. The base scene is the clock.
- CPU, memory, and context switches: `scripts/measure.sh <pid> 60` on the
  service. A shell loop sent the alert again each 20 s, 4 times. The
  measurement started 5 s after the first alert. One run used `panic`, and a
  second run used `bedtime`.
- Refresh rate: `gameclockd --led-show-refresh` for 24 s, with the service
  stopped and the socket in `/tmp`. The alert started 3 s after the daemon.
  The table uses the samples from 1 s after the alert start to 1 s before its
  finish (3,186 samples for each alert).

## Results

| | message-board (message scrolls) | `panic` | `bedtime` |
|---|---|---|---|
| Process CPU | 55.6% | 53.6% | 54.9% |
| Main thread CPU | 6.3% | 4.1% | 5.5% |
| Main thread voluntary context switches (60 s) | 3,032 | 2,393 | 2,974 |
| `signals` thread CPU, switches | 0.0%, 0 | 0.0%, 0 | 0.0%, 0 |
| `control` thread CPU, switches | 0.0%, 0 | 0.0%, 12 | 0.0%, 12 |
| Refresh thread CPU | 49.3% | 49.5% | 49.5% |
| RSS | 3,108 KiB | 2,848 KiB | 2,852 KiB |
| Refresh rate, median | 178.3 Hz | 178.5 Hz | 178.1 Hz |
| Refresh rate, 5th percentile | 173.9 Hz | 172.8 Hz | 172.0 Hz |
| Refresh rate, lowest sample | 148.7 Hz | 149.7 Hz | 152.6 Hz |

The budget in the `alerts` spec is 15% CPU or less for the main thread, and a
median refresh rate of 170 Hz or more. Both alerts pass.

`panic` renders 20 times each second, so it uses less CPU than a message,
which renders 25 times each second. The `control` thread switches count the
12 `ctl` commands of the shell loop.

The Pi binary is 1,058,608 bytes. The binary on `main` is 1,030,912 bytes, so
the change adds 27,696 bytes.

## Checks on the panel

The user looked at the panel and saw each of these items:

- `panic`: the wheel moves smoothly, and the text changes between black and
  white. The wheel speed of the legacy clock is good.
- `bedtime`: the large lime text fits the panel and scrolls with no flicker.
- `bed-flash`: `BED!!!!!` shows in red for 1 s. The clock returns after each
  alert.
- A message that `panic` interrupts starts again from the right edge after
  `panic` finishes.
- `clear` removes `panic` and shows the clock at once. `clear` does not
  remove `bedtime`. `alert stop` removes it.

The `status` replies during these items:

- During `panic` with an interrupted message: `messages=1 alert=panic`.
- After `panic` finished: `messages=1 alert=none`. The message scrolled again.
- After `clear` during `panic`: `messages=0 alert=none`.
- After `clear` during `bedtime`, with one waiting message:
  `messages=0 alert=bedtime`.
- After `alert stop`: `messages=0 alert=none`.

## After the checks

The legacy clock runs again and starts at boot. `gameclockd` is stopped and
disabled.
