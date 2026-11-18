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
