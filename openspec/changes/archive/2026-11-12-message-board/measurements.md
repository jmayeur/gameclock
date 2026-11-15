# Measurements: message board

Device: the same Pi Zero W, OS, HAT, and panel as in panel bring-up.
Date: 2026-11-12.

## Method

- Driver flags: the unit's `--led-pwm-bits=8` and the built-in
  `adafruit-hat-pwm` mapping. The base scene is the clock.
- Message: 64 characters, `--passes=10`. One pass is 512 steps of 40 ms
  (20.48 s), so the message scrolls for 204.8 s.
- CPU, memory, and context switches: `scripts/measure.sh <pid> 60`, from 5 s
  after the message started. The service ran the message.
- Refresh rate: `gameclockd --led-show-refresh` for 30 s, with the service
  stopped and the socket in `/tmp`. The message started 5 s after the
  daemon. The table uses the samples after the first quarter of the run, so
  that all of them come while the message scrolls.

## Results

| | command-input (clock, no message) | message-board (message scrolls) |
|---|---|---|
| Process CPU | 49.4% | 55.6% |
| Main thread CPU | 0.0% | 6.3% |
| Main thread voluntary context switches (60 s) | 2 | 3,032 |
| `signals` thread CPU, switches | 0.0%, 0 | 0.0%, 0 |
| `control` thread CPU, switches | 0.0%, 0 | 0.0%, 0 |
| Refresh thread CPU | 49.4% | 49.3% |
| RSS | 3,100 KiB | 3,108 KiB |
| Refresh rate, median | 176.2 Hz | 178.3 Hz (3,971 samples) |
| Refresh rate, 5th percentile | 174.1 Hz | 173.9 Hz |
| Refresh rate, lowest sample | not recorded | 148.7 Hz |
| Pi binary size | 1,013,912 bytes | 1,030,912 bytes |

The budget in the `message-board` spec is 15% CPU or less for the main
thread, and a median refresh rate of 170 Hz or more. Both pass.

The main thread does about 50 voluntary context switches each second. It
wakes 25 times for the steps. Each present blocks one more time while the
driver waits for the vertical sync.

## Checks on the panel

The user looked at the panel and saw each of these items:

- `hello world` scrolls 4 times, smoothly and with no flicker. Then the clock
  returns. The speed of 25 pixels each second is good.
- `two` follows `one` with no clock between. `clear` shows the clock at once.
- `status` gave `messages=3` with three messages, `messages=2` after the
  first finished, and `messages=0` after `clear`.

## After the checks

The legacy clock runs again and starts at boot. `gameclockd` is stopped and
disabled.
