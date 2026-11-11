# Measurements: command input

Device: the same Pi Zero W, OS, HAT, and panel as in panel bring-up.
Date: 2026-11-10.

## Method

The method is the same as in `clock-scene`:

- CPU, memory, and context switches: `scripts/measure.sh <pid> 60`, after the
  service ran for 8 s or more. One `status` command came before the window.
- System idle: the idle and iowait share of `/proc/stat` over 60 s.
- Refresh rate: `gameclockd --led-show-refresh` for 20 s, with the service
  stopped. Median and 5th percentile of all samples.
- Driver flags: the unit's `--led-pwm-bits=8` and the built-in
  `adafruit-hat-pwm` mapping. `gameclockd` shows the clock.

## Results

| | clock-scene | command-input |
|---|---|---|
| Process CPU | 49.0% | 49.4% |
| Main thread CPU | 0.0% | 0.0% |
| Main thread voluntary context switches (60 s) | 2 | 2 |
| `signals` thread CPU, switches | n/a | 0.0%, 0 |
| `control` thread CPU, switches | n/a | 0.0%, 0 |
| Refresh thread CPU | 48.8% | 49.4% |
| RSS | 2,696 KiB | 3,100 KiB |
| System idle | 43.4% | 45.7% |
| Refresh rate, median | 176.2 Hz | 176.2 Hz (3,532 samples) |
| Refresh rate, 5th percentile | 174.0 Hz | 174.1 Hz |
| Pi binary size | 922,552 bytes | 1,013,912 bytes |

### Memory

A later read of `/proc/<pid>/smaps_rollup` gave an RSS of 2,864 KiB. Of
that, 1,005 KiB of the proportional size is shared file pages: libc (972 KiB
resident) and the binary's code (908 KiB resident). Private dirty memory is
488 KiB. Most of the increase is more code pages of a larger binary, not the
thread stacks.

### Command time

| Measurement | Median | Slowest |
|---|---|---|
| Socket round trip of `status`, from one Python process, 20 times | 3.6 ms | 4.3 ms |
| `gameclockd ctl status` as root, 20 times | 59.0 ms | 123.9 ms |
| `gameclockd ctl` to a path with no daemon, 20 times (process start only) | 56.1 ms | 122.9 ms |
| `/bin/true` as root, 20 times | 40.3 ms | 88.4 ms |

## Findings

- The new threads block in the kernel. In 60 s, each used 0.0% CPU and made
  0 voluntary context switches. "Clock daemon wakes only each minute" passes,
  with its new condition for the other threads.
- The refresh rate and the main thread did not change.
- The daemon answers a command in less than 5 ms. The time of
  `gameclockd ctl` is almost all process start on the shared core. One of 20
  runs took 124 ms.
- The binary is 91,360 bytes (10%) larger. The `signal-hook` iterator, the
  socket code, and the client are the likely cause.
- On-device checks: user `pi` gets "Permission denied" and exit 1. `sudo
  gameclockd ctl status` gets `ok mode=clock zone=America/Los_Angeles
  brightness=60`. The socket is mode 0600 and the directory is mode 0700,
  both owned by root. The user saw the panel change to the test pattern and
  back to the clock. `/run/gameclockd` is gone after the stop.
