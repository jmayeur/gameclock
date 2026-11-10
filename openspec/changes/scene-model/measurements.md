# Measurements: scene model

Device: the same Pi Zero W, OS, HAT, and panel as in panel bring-up
(`openspec/changes/archive/2026-11-09-panel-bring-up/measurements.md`).
Date: 2026-11-10.

## Method

The method is the same as at bring-up:

- CPU, memory, and context switches: `scripts/measure.sh <pid> 60`, after the
  service ran for 10 s or more.
- System idle: the idle and iowait share of `/proc/stat` over the same window.
- Refresh rate: `gameclockd --led-show-refresh` for 20 s, through an ssh
  pipe. Median and 5th percentile of all samples.
- Driver settings: hzeller's defaults over the Pi defaults (11 PWM bits, GPIO
  slowdown 1). No flags in `/etc/default/gameclockd`.
- `gameclockd` shows the heartbeat: one present each second.

## Results

| | Bring-up (static test pattern) | scene-model (heartbeat) |
|---|---|---|
| Process CPU | 52.1% | 51.9% |
| Main thread CPU | 0.0% | 0.2% |
| Main thread voluntary context switches (60 s) | 0 | 122 |
| Refresh thread CPU | 52.1% | 51.5% |
| RSS | 2,256 KiB | 2,760 KiB |
| System idle | 40.1% | 40.8% |
| Refresh rate, median | 146.9 Hz | 149.3 Hz (2,983 samples) |
| Refresh rate, 5th percentile | 143.5 Hz | 146.5 Hz |
| Pi binary size | 584,076 bytes | 916,752 bytes |

The bring-up binary size is from a build of `main` at the start of this
change, with the same toolchain.

## Findings

- The main thread uses 0.2% of one core. The limit in the spec is 1%.
- The main thread makes about 2 voluntary context switches each second: the
  wait and the driver's swap on vsync. 122 is in the spec range of 55 to 180.
  The scenario "Heartbeat daemon wakes only each second" passes.
- The refresh rate did not fall. It is 1.6% higher than at bring-up. That is
  probably normal variation between runs.
- RSS is 504 KiB more. The proposal expected less than 1 MiB more. jiff and
  the parsed time-zone data are the likely cause.
- The binary is 332,676 bytes (57%) larger. jiff is the likely cause. The
  Pi has space for it. We did not look at the parts of the binary in detail.
