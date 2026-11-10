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
- `gameclockd` refreshes about 9% slower with the same settings. The cause is
  in the driver build (version or compiler), not in application CPU. Not yet
  explained.
- Both programs show random shimmer glitches. The glitches are more visible
  with `gameclockd`.

## GPIO slowdown on gameclockd

| `--led-slowdown-gpio` | Refresh median | Refresh 5th pct | What the panel shows |
|---|---|---|---|
| 0 | 159.1 Hz | 156.1 Hz | Worse: wrong colors flicker near the border |
| 1 (default) | 146.9 Hz | 143.5 Hz | Correct pattern, random shimmer glitches |
| 2 | 134.2 Hz | 128.2 Hz | More glitches than the default |

The default (1) is the best of the three. Slowdown 0 is too fast for this
panel.

## On-device scenarios

| Scenario | Result |
|---|---|
| Test pattern on the real panel | Pass: corners, border, and bars are correct (checked by eye) |
| Driver flags reach the driver | Pass: `--led-show-refresh` prints the refresh rate |
| Idle daemon does not wake | Pass: 0 voluntary context switches and 0.0% CPU on the main thread in 60 s |
| Service starts as root, driver drops privileges | Pass: the process runs as `daemon` |
| Plain deploy leaves boot unchanged (enablement part) | Pass: `gameclockd` disabled, `clock.svc.service` enabled |
