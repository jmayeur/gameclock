# Measurements: clock scene

Device: the same Pi Zero W, OS, HAT, and panel as in panel bring-up
(`openspec/changes/archive/2026-11-09-panel-bring-up/measurements.md`).
Date: 2026-11-10.

## Method

The method is the same as in `scene-model`:

- CPU, memory, and context switches: `scripts/measure.sh <pid> 60`, after the
  service ran for 5 s or more.
- System idle: the idle and iowait share of `/proc/stat` over 60 s.
- Refresh rate: `gameclockd --led-show-refresh` for 20 s, with the service
  stopped. hzeller writes the rate to standard output. Median and 5th
  percentile of all samples.
- `gameclockd` shows the clock: one present each minute. The look checks use
  a temporary systemd drop-in with a `TZ` value. It puts the local time in
  the period under test. The drop-ins are removed after the checks.
- Driver flags: the unit's `--led-pwm-bits=8`, and the flags in each column
  heading. The mapping is the built-in default of the build under test.

## Results

| | scene-model (heartbeat, `adafruit-hat`, 11 bits) | clock, `adafruit-hat`, 11 bits | clock, `adafruit-hat`, 8 bits | clock, `adafruit-hat-pwm`, 8 bits |
|---|---|---|---|---|
| Process CPU | 51.9% | 52.6% | 49.1% | 49.0% |
| Main thread CPU | 0.2% | 0.0% | 0.0% | 0.0% |
| Main thread voluntary context switches (60 s) | 122 | 2 | 2 | 2 |
| Refresh thread CPU | 51.5% | 52.6% | 49.1% | 48.8% |
| RSS | 2,760 KiB | 2,704 KiB | 2,776 KiB | 2,696 KiB |
| System idle | 40.8% | 39.6% | 41.6% | 43.4% |
| Refresh rate, median | 149.3 Hz | 146.3 Hz (2,919 samples) | 155.8 Hz (3,113 samples) | 176.2 Hz (3,532 samples) |
| Refresh rate, 5th percentile | 146.5 Hz | 141.7 Hz | 152.7 Hz | 174.0 Hz |
| Moving white lines (red at 25) | not checked | yes | yes | no |

- The 11-bit column uses `--led-pwm-bits=11` in `/etc/default/gameclockd`
  after the unit's `--led-pwm-bits=8`. Its refresh rate is the 11-bit rate,
  so the later flag wins.
- The `adafruit-hat-pwm` column is the final build, with the mapping as the
  built-in default and no other flag.
- Pi binary size: 922,552 bytes. `main` is 916,752 bytes (the scene-model
  build). The change adds 5,800 bytes.

### Test-pattern mode

`--test-pattern` with `adafruit-hat` and 8 bits, for 60 s: main thread 0.2%
CPU and 120 voluntary context switches. Process CPU 49.4%, RSS 2,684 KiB.

## Look checks

| Setting | Clock, red at 25 |
|---|---|
| `gameclockd`, `adafruit-hat`, 8 bits | white lines at random times that move down the panel |
| `gameclockd`, `adafruit-hat`, 11 bits | about the same |
| `gameclockd`, `adafruit-hat`, 8 bits, `--led-scan-mode=1` | about the same |
| Legacy clock (Python, 2020 driver build), same `TZ` | no lines, clean and steady |
| `gameclockd`, `adafruit-hat-pwm`, 8 bits | no lines, clean and steady |

The legacy Python driver module defaults to `adafruit-hat-pwm`. With the
final build, the user saw red at 25 (`03:06AM`) and mellow at 25 (`11:42PM`).
Both were steady and correct, with no color steps, flicker, or moving lines.

## Findings

- The clock wakes once each minute. The main thread uses 0.0% of one core
  and makes 2 voluntary context switches in 60 s. The limits in the spec are
  1% and 10. "Clock daemon wakes only each minute" passes.
- The test-pattern mode still wakes once each second (120 switches, range 55
  to 180). "Heartbeat daemon wakes only each second" passes.
- 8 PWM bits give 6.5% more refresh rate and 3.5 points less CPU than 11
  bits. They did not change the moving lines.
- The moving lines come from the `adafruit-hat` mapping. With
  `adafruit-hat-pwm`, the lines go away, and the refresh rate is 13% higher
  than with `adafruit-hat` at 8 bits. CPU does not change.
- The shimmer glitches of bring-up are probably the same fault. Bring-up did
  not try the PWM mapping.
