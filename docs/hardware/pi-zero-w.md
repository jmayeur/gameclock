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
| GPIO mapping | `adafruit-hat`. The service passes no mapping flag, so it is probably compiled in by the Adafruit installer. Confirm at bring-up. |
| Hardware pulse | `notes.md` used `--led-no-hardware-pulse`. With audio off, hardware pulse may work and would improve image quality. Test at bring-up. |
| PWM solder mod (GPIO4↔18) | Unknown; the legacy hints suggest not. That would mean the `adafruit-hat` mapping rather than `adafruit-hat-pwm`. |

## Software currently on the device

- hzeller `rpi-rgb-led-matrix` at `a3eea99` (2023-05-29), installed in place
  under `/home/pi/rpi-rgb-led-matrix`.
- Service `clock.svc.service`:
  `sudo /usr/bin/python .../bindings/python/samples/clock.py --led-cols=64 --led-rows=32`.
- Command file `.../samples/semaphore/msg.txt` (mode 0777).
- Python 3.7.3, Node 10.24.0.

## Tuning notes for a single-core Zero

- `--led-slowdown-gpio`: the Zero is slow enough that 0 or 1 usually works.
  Lower values give faster refresh.
- `--led-pwm-bits`: fewer bits mean less refresh-thread CPU and a higher
  refresh rate. A clock doesn't need 11 bits of color depth. Measure this.
- `--led-limit-refresh-rate-hz`: caps the refresh thread so the app keeps some
  CPU.
- hzeller drops root privileges to `daemon` after initialising unless told
  otherwise. The systemd unit should still start it as root.

## Open items

- [ ] Confirm the GPIO mapping and whether hardware pulse works (panel bring-up).
- [ ] Measure the legacy baseline: CPU %, refresh Hz, RSS (panel bring-up).
- [ ] Decide whether to reflash to current Raspberry Pi OS Lite 32-bit
  (ADR-0007). Not required.
