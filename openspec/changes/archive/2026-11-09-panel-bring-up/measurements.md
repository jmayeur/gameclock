# Measurements: panel bring-up

Device: Raspberry Pi Zero W Rev 1.1, Raspbian 10 (glibc 2.28), Adafruit RGB
Matrix HAT (no GPIO4–GPIO18 jumper), one 64×32 panel. Date: 2026-11-09.

## Method

- CPU and memory: `scripts/measure.sh <pid> 60`, which samples `/proc` at the
  start and the end of a 60 s window. CPU is a percentage of the single core.
- System idle: the idle and iowait share of `/proc/stat` over the same window.
- Refresh rate: the driver's `--led-show-refresh` output over 15–20 s. The
  output goes through an ssh pipe for both programs. Median and 5th percentile
  of all samples.
- Both programs show a static image during the window. Nothing else runs
  except the OS.

## Driver settings

Both programs use the same settings: 64×32, chain 1, parallel 1,
`adafruit-hat` mapping, and hzeller's defaults for the rest (11 PWM bits,
GPIO slowdown 1). The legacy clock passes `--led-cols=64 --led-rows=32` and
has the mapping compiled in. `gameclockd` sets the same values as defaults.

## Results

| | Legacy clock (Python) | gameclockd |
|---|---|---|
| Driver | hzeller `a3eea99` (2023), gcc | hzeller `3da7a3b` (2026), zig/clang |
| Process CPU | 53.9% | 52.1% |
| Main thread CPU | 0.9% | 0.0% |
| Main thread wakeups (60 s) | 396 | 0 |
| Refresh thread CPU | 52.9% | 52.1% |
| RSS | 9,800 KiB | 2,256 KiB |
| System idle | 39.8% | 40.1% |
| Refresh rate, median | 162.2 Hz | 146.9 Hz |
| Refresh rate, 5th percentile | 155.8 Hz | 143.5 Hz |

The legacy RSS is for the Python process only. Its `sudo` parent process is
not counted.

## Findings

- The refresh thread uses about 52% of the core in both programs. The
  application work is close to zero in both, so the refresh thread sets the
  CPU budget.
- `gameclockd`'s main thread did not wake in 60 s. This passes the "Idle
  daemon does not wake" scenario.
- `gameclockd` uses about a quarter of the legacy memory.
- `gameclockd` refreshes about 9% slower with the same settings. The driver
  version is not the cause: our build of the 2023 driver is slower still
  (141 Hz). The legacy driver was compiled on the Pi with gcc 8.3 `-O3`. Ours
  is compiled with zig (clang). The compiler is the likely cause. We did not
  check this yet.
- Both programs show random shimmer glitches. The glitches are more visible
  with `gameclockd`, and mainly when the whole panel is lit (the test pattern,
  the legacy panic screen). A clock face or a message shows few glitches. We
  accept this for now. The next step is the GPIO4–GPIO18 jumper with
  `--led-gpio-mapping=adafruit-hat-pwm`, so that the hardware times the
  pulses.

## Legacy configuration facts

- The legacy clock runs on Python 2.7 (`/usr/bin/python`). Its driver is the
  one linked into the Python module
  `/usr/local/lib/python2.7/dist-packages/rgbmatrix/core.so`.
- Its `samplebase.py` sets `--led-scan-mode=1` (interlaced) by default.
  hzeller's default, and ours, is 0 (progressive).
- It changes brightness by time of day (60% during these measurements).
  `gameclockd` shows 100%.

## Tuning on gameclockd

Each row changes one setting from the defaults. "Glitches" compares by eye
with the default row.

| Setting | Refresh median | Refresh 5th pct | Refresh thread CPU | Glitches |
|---|---|---|---|---|
| Defaults | 146.9 Hz | 143.5 Hz | 52.1% | Some, mainly with the full panel lit |
| `--led-slowdown-gpio=0` | 159.1 Hz | 156.1 Hz | not measured | Worse: wrong colors flicker near the border |
| `--led-slowdown-gpio=2` | 134.2 Hz | 128.2 Hz | not measured | More |
| `--led-scan-mode=1` | 147.2 Hz | 144.3 Hz | not measured | About the same |
| `--led-pwm-bits=8` | 157.5 Hz | 154.0 Hz | 49.5% | Accepted as is |
| Driver `a3eea99` (2023), defaults | 141.0 Hz | 136.0 Hz | 54.1% | About the same |

The defaults stay. `--led-pwm-bits=8` is a candidate for the clock scene: it
gives a higher refresh rate for less CPU. A clock needs few colors.

## On-device scenarios

| Scenario | Result |
|---|---|
| Test pattern on the real panel | Pass: corners, border, and bars are correct (checked by eye) |
| Driver flags reach the driver | Pass: `--led-show-refresh` prints the refresh rate |
| A flag overrides a default | Pass, with a different flag: `--led-slowdown-gpio=0` overrides the default of 1 and visibly changes the image. `--led-gpio-mapping=regular` was not run. |
| Invalid flags fail at startup | Pass: `--led-rows=abc` exits 1 with a panel initialization error |
| Rejected driver options fail cleanly | Pass: same run as above |
| Unprivileged start warns and keeps running | Pass: the driver prints "not running as root ... Expect color degradation" and keeps running |
| Panel is dark after stop | Pass: the panel went dark after `systemctl stop` (checked by eye) |
| Idle daemon does not wake | Pass: 0 voluntary context switches and 0.0% CPU on the main thread in 60 s |
| Service replaces the legacy clock | Pass: starting `gameclockd` stopped `clock.svc.service` |
| Legacy clock is a one-command rollback | Pass: starting `clock.svc.service` stopped `gameclockd`. `scripts/deploy.sh --rollback` restored the legacy clock. |
| Crash is restarted | Pass: systemd restarted `gameclockd` 2 s after `kill -9` |
| Deploy installs and restarts | Pass: `scripts/deploy.sh` installed the binary and the unit, and the panel showed the pattern |
| Plain deploy leaves boot unchanged | Partial: after the deploy, `gameclockd` was disabled and `clock.svc.service` enabled. The reboot was skipped by decision. |
| Enable swaps the boot service | Not run: skipped by decision, because it needs two reboots of the Pi |
| Service starts as root, driver drops privileges | Pass: the process runs as `daemon` |

## End state

The legacy clock runs and is enabled at boot. `gameclockd` is installed in
`/usr/local/bin` with its unit, stopped and disabled. `/etc/default/gameclockd`
does not exist.
