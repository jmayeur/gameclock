# ADR-0015: Driver settings for the clock

- **Status:** Accepted
- **Date:** 2026-11-10
- **Related:** ADR-0004, ADR-0009, ADR-0010. Changes the default GPIO mapping
  of ADR-0009. OpenSpec change `clock-scene`
  (`measurements.md` has the numbers).

## Context

ADR-0009 sets the driver defaults for this Pi, with the `adafruit-hat` GPIO
mapping. Bring-up saw random shimmer glitches with that mapping. The first
look at the clock showed white lines at random times that move down the
panel, mainly at brightness 25.

The legacy clock does not show the lines. Its Python driver module defaults
to the `adafruit-hat-pwm` mapping. This mapping drives the panel's output
enable from the Pi's hardware pulse on GPIO18. It needs a link between GPIO4
and GPIO18 on the HAT.

Bring-up also measured `--led-pwm-bits=8`: a higher refresh rate and less
CPU than the default of 11 bits.

## Decision

- **The built-in GPIO mapping is `adafruit-hat-pwm`.** It replaces
  `adafruit-hat` in the `rgbmatrix-sys` defaults. A `--led-gpio-mapping` flag
  still replaces it.
- **The service unit passes `--led-pwm-bits=8`** before the flags from
  `/etc/default/gameclockd`. hzeller reads the flags in order, so a device
  can set a different value there.

## Consequences

- On this Pi, the lines go away. The refresh rate goes up from 155.8 Hz to
  176.2 Hz at 8 bits, and CPU does not change (49%).
- With 8 bits instead of 11, the refresh rate is 6.5% higher and the process
  uses 3.5 points less CPU. The clock's red and mellow at brightness 25 show
  no color steps.
- The HAT must have the GPIO4–18 link. This HAT has it, because the legacy
  driver uses the same mapping with a correct image. A HAT with no link can
  use `--led-gpio-mapping=adafruit-hat` in `/etc/default/gameclockd`.
- A manual start of `gameclockd` uses 11 bits, because the 8-bit flag is in
  the unit. The image is correct with both values.

## Alternatives considered

- **Keep `adafruit-hat` and pass the PWM mapping in the unit.** A manual
  start then shows the lines again.
- **Interlaced scan (`--led-scan-mode=1`).** No change to the lines.
- **11 PWM bits.** No change to the lines, a lower refresh rate, and more CPU.
- **8 PWM bits as a built-in default.** The unit is the place for tuning
  that can change by device (ADR-0010). The mapping is a fact of this HAT.
