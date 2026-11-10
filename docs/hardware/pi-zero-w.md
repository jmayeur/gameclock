# Hardware: Raspberry Pi Zero W + Adafruit RGB Matrix HAT

Facts collected from the running device on 2026-11-09, plus what the legacy
code implies. They drive the driver defaults (ADR-0004) and the build target
(ADR-0007).

## Board and OS

| Item | Value |
|---|---|
| Model | Raspberry Pi Zero W Rev 1.1 (BCM2835) |
| CPU | ARM1176JZF-S, **ARMv6**, single core, `armv6l` |
| RAM | 512 MB (430 MiB visible; about 285 MiB available with the legacy stack running) |
| OS | Raspbian 10 **Buster** (end-of-life), glibc 2.28 |
| RTC | None. `fake-hwclock` restores time at boot, so the clock depends on NTP. |
| Audio | `dtparam=audio=off`, which frees the PWM peripheral for hardware pulse |
| Display overlay | `dtoverlay=vc4-fkms-v3d` (not needed headless; a candidate to drop) |

## Panel and HAT

| Item | Value |
|---|---|
| Panel | 64×32 HUB75, single panel, no chain or parallel |
| Interface | Adafruit RGB Matrix HAT (the legacy installer supported HAT and Bonnet) |
| GPIO mapping | `adafruit-hat`, confirmed at bring-up: the `gameclockd` test pattern is right with it. The legacy service passes no mapping flag. Its Python module has its own driver build. |
| Hardware pulse | Needs the GPIO4↔18 jumper and `adafruit-hat-pwm`. Audio is already off, as hardware pulse requires. Not tested yet. |
| PWM solder mod (GPIO4↔18) | Not fitted. It is the candidate fix for the shimmer glitches seen at bring-up (see below). |

## Driver used by gameclockd

hzeller `rpi-rgb-led-matrix` is vendored as a submodule at
`vendor/rpi-rgb-led-matrix`, pinned to
`3da7a3bdfca4637bb79ff038bc2bb26965197db5` (2026-10-24). It is statically
linked into the binary, so the Pi's own 2023 copy is not used (ADR-0009).

## Software currently on the device

- hzeller `rpi-rgb-led-matrix` at `a3eea99` (2023-05-29), installed in place
  under `/home/pi/rpi-rgb-led-matrix`.
- Service `clock.svc.service`:
  `sudo /usr/bin/python .../bindings/python/samples/clock.py --led-cols=64 --led-rows=32`.
- Command file `.../samples/semaphore/msg.txt` (mode 0777).
- The clock runs on Python 2.7 (`/usr/bin/python`). Its driver is linked
  into `/usr/local/lib/python2.7/dist-packages/rgbmatrix/core.so`. Python
  3.7.3 and Node 10.24.0 are also installed.
- `samplebase.py` sets `--led-scan-mode=1` (interlaced) by default, and the
  clock changes brightness by time of day.

## Tuning notes for a single-core Zero

Measured at bring-up
(`openspec/changes/panel-bring-up/measurements.md`, archived with the change):

- The refresh thread uses about 52% of the core with the default settings.
  The application uses close to none.
- `--led-slowdown-gpio`: 1 (the default) is right for this panel. 0 gives
  wrong colors. 2 gives more glitches.
- `--led-pwm-bits=8`: 157 Hz instead of 147 Hz, and 49.5% CPU instead of
  52%. A clock doesn't need 11 bits of color depth.
- `--led-scan-mode=1` (interlaced): no visible change and no change in refresh
  rate.
- Known issue: random shimmer glitches, mainly when the whole panel is lit.
  The legacy clock has them too, less often. Software settings did not
  remove them. Try the GPIO4↔18 jumper with `adafruit-hat-pwm`.
- `--led-limit-refresh-rate-hz`: caps the refresh thread so the app keeps some
  CPU.
- hzeller drops root privileges to `daemon` after initialising unless told
  otherwise. The systemd unit should still start it as root.

## Open items

- [x] Confirm the GPIO mapping (panel bring-up): `adafruit-hat`.
- [x] Measure the legacy baseline: CPU %, refresh Hz, RSS (panel bring-up).
- [ ] Fit the GPIO4↔18 jumper and test hardware pulse with `adafruit-hat-pwm`.
- [ ] Decide whether to reflash to current Raspberry Pi OS Lite 32-bit
  (ADR-0007). Not required.
