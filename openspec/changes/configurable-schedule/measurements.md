# Measurements: configurable schedule

Device: the same Pi Zero W, OS, HAT, and panel as in panel bring-up.
Date: 2026-11-15.

## Method

- Driver flags: the unit's `--led-pwm-bits=8` and the built-in
  `adafruit-hat-pwm` mapping. The base scene is the clock.
- `/etc/gameclockd/config.toml` is `deploy/config.toml`, which the deploy
  installed.
- Idle loop: `scripts/measure.sh <pid> 60` on the service, from 10 s after
  the start, with the clock only.
- Reload time: the wall time of `sudo gameclockd ctl reload`, 5 times, from a
  shell on the Pi. The time of `sudo gameclockd ctl status` is the reference
  for the start of `sudo` and the client.

## Results

| | alerts (main) | configurable-schedule |
|---|---|---|
| Pi binary size | 1,058,608 bytes | 1,292,576 bytes (+233,968) |
| Main thread CPU, idle clock | 0.0% (`command-input`) | 0.0% |
| Main thread voluntary context switches (60 s) | 2 (`command-input`) | 2 |
| Process CPU, idle clock | 49.4% (`command-input`) | 49.0% |
| RSS, idle clock | 3,100 KiB (`command-input`) | 3,192 KiB |

The `alerts` measurements had alerts on the panel, so the idle numbers in the
first column come from the `command-input` measurements.

| Command | Wall time on the Pi |
|---|---|
| `reload`, 5 runs | 398, 310, 358, 325, 307 ms. The median is 325 ms. |
| `status`, 1 run | 388 ms |

The `daemon-config` spec gives the budget for the idle main thread: less than
1% CPU and at most 10 voluntary context switches. A `reload` reply must come
in less than 1 s. All pass. Almost all of the reload time is the start of `sudo`
and the client, because a `status` takes the same time.

The binary grows by 234 KB for the TOML parser and the serde code. The
proposal expected 100 to 300 KB. RSS grows by 92 KiB against the
`command-input` measurement.

## Checks on the Pi

- The deploy installed `/etc/gameclockd/config.toml` with mode 0644, in
  `/etc/gameclockd` with mode 0755. The user saw the clock as before.
- `reload` with the 21:00 to 23:00 rule changed to green: the user saw the
  clock turn green with no restart. A first try changed only the blue rules,
  which do not apply at 21:06, so the clock stayed mellow orange as it must.
- A deploy with the green edit in the file did not change the file. The
  SHA-256 was the same before and after.
- A file with `brightness = 120` on line 16: after `systemctl restart`, the
  service failed with exit status 2, `NRestarts=0`. The journal had
  `gameclockd: /etc/gameclockd/config.toml:16:44: the brightness must be
  from 0 to 100.` With the file restored, the service started.

## After the checks

`/etc/gameclockd/config.toml` has the content of `deploy/config.toml`. The
legacy clock runs again and starts at boot. `gameclockd` is stopped and
disabled.
