# Measurements: sensor-pages

All device numbers are from the Pi Zero W. The addresses of the sensor Pis
and the NWS grid point are not in this file.

## Before the change (2026-11-17)

One `curl` fetch on the Pi, with `gameclockd` running (`time curl ...`):

| Fetch | Wall time | User CPU | System CPU | Body |
|---|---|---|---|---|
| Sensor Pi `realtime.json`, HTTP on the LAN | 0.569 s | 0.148 s | 0.035 s | 320 bytes |
| NWS forecast, HTTPS | 0.793 s | 0.170 s | 0.112 s | 13,553 bytes |

## Binary size

`scripts/build-pi.sh`, then `scripts/check-pi-binary.sh`: ARMv6 hard-float,
glibc 2.28 or older.

| Build | Size |
|---|---|
| `main` (`02a7774`) | 1,332,256 bytes |
| `sensor-pages` | 1,500,496 bytes (+168,240) |

The growth comes from `serde_json`, the two page layouts with the 6×10 font,
and the fetch code. The binary is loaded once, so this is not a CPU cost.

## On the Pi after the deploy (2026-11-18)

- The deploy installed the binary, and the config kept its tables. A
  `[pages]` section with the three URLs went into
  `/etc/gameclockd/config.toml` on the Pi only. `ctl reload` replied `ok`.
  The web page had three page buttons.
- `ctl page weather`, `outdoor`, and `indoor` each replied `ok`. `status`
  then gave `page=<name>`. `ps` showed `curl` as the `daemon` user. The
  journal had no page warnings.

### Refresh rate during fetches

Method: the service stopped. `gameclockd --led-pwm-bits=8
--led-show-refresh --http=off` ran for 40 s, two times. The first run had no
requests. In the second run, `ctl page weather` came each 5 s, 7 times, and
each reply was `ok`. Each run gives one rate for each hzeller refresh cycle.

| | No requests | `page weather` each 5 s |
|---|---|---|
| Samples | 7,067 | 7,040 |
| Mean | 177.6 Hz | 177.3 Hz (99.8%) |
| Median | 176.7 Hz | 177.7 Hz |
| 5th percentile | 174.2 Hz | 163.4 Hz (93.8%) |
| 1st percentile | 165.0 Hz | 155.8 Hz |
| Lowest sample | 144.4 Hz | 118.0 Hz |
| Samples below 159 Hz (90%) | 16 (0.2%) | 152 (2.2%) |

The rate stays above 90% of the rate with no requests, so the spec passes
and `nice` is not necessary. Each `curl` run gives short dips, at most to
118 Hz, while it uses the core.

### Memory after requests

`VmRSS` of the service before and after 10 page requests, 3 s apart:
3,456 KiB, then 3,508 KiB (+52 KiB). The limit is 1 MiB.
