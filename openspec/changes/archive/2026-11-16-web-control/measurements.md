# Measurements: web control

Device: the same Pi Zero W, OS, HAT, and panel as in panel bring-up.
Date: 2026-11-16.

## Method

- Driver flags: the unit's `--led-pwm-bits=8` and the built-in
  `adafruit-hat-pwm` mapping. The base scene is the clock.
- Memory: `ps -o rss` for each process, and `free -m`.
- Response time: `curl -w '%{time_total}'` from a laptop on the same Wi-Fi
  network, 5 runs of each request, one after the other. The `POST` sends
  `msg=test`.
- Idle loop: `scripts/measure.sh <pid> 60` on the service, from 10 s after
  the start, with the clock only and the listener on port 80.

## Before: the legacy web stack

nginx on port 80 sends each request to Node on port 3000. PM2 keeps Node
running. The legacy clock runs.

| Process | RSS | Average CPU since start |
|---|---|---|
| Node (`server.js`) | 42,880 KiB | 3.7% |
| PM2 | 58,520 KiB | 0.5% |
| nginx (2 processes) | 5,260 KiB | 0.0% |
| **Total** | **106,660 KiB** | |

`free -m`: 430 MB total, 93 MB used, 286 MB available.

| Request | Runs (s) | Median |
|---|---|---|
| `GET /` | 0.933, 0.684, 0.125, 0.140, 0.052 | 140 ms |
| `POST /message` | 0.274, 0.276, 0.072, 0.064, 0.062 | 72 ms |

The first runs are slower. The Wi-Fi of the Pi can sleep between requests.

## After: gameclockd on port 80

`scripts/deploy.sh` stopped nginx and PM2, and restarted `gameclockd`.
`sudo ss -ltnp` showed `gameclockd` on `0.0.0.0:80`, and nothing on port
3000. `GET /health` from the laptop gave 200 with `{"status":"ok"}` in 26 ms.

| | configurable-schedule (main) | web-control |
|---|---|---|
| Pi binary size | 1,292,576 bytes | 1,332,256 bytes (+39,680) |
| Main thread CPU, idle clock | 0.0% | 0.0% |
| Main thread voluntary context switches (60 s) | 2 | 2 |
| Web thread CPU, idle (60 s) | | 0.0%, 0 switches |
| Process CPU, idle clock | 49.0% | 49.0% |
| RSS, idle clock | 3,192 KiB | 3,420 KiB (+228 KiB) |

The proposal expected less than 60 KB of binary growth and less than
200 KiB of RSS growth. The binary is in the estimate. The RSS is 28 KiB over
it. The RSS was measured after one `/health` request. A possible cause is
the memory arena that glibc makes for a thread that allocates. We did not
examine it. The spec has no RSS budget.

`free -m` after the switch: 36 MB used and 330 MB available, against 93 MB
and 286 MB with the legacy web stack.

| Request | Runs (s) | Median | Legacy median |
|---|---|---|---|
| `GET /` | 0.021, 0.024, 0.022, 0.018, 0.019 | 21 ms | 140 ms |
| `POST /message` | 0.029, 0.019, 0.107, 0.052, 0.023 | 29 ms | 72 ms |

All runs are below the 500 ms budget of the spec.

## Checks on the Pi

- The user opened the page on a phone, sent a message, and pressed the panic
  button. The panel showed the message, and then the panic alert.
- The stop of `pm2-pi.service` in the deploy timed out after 90 s. systemd
  then stopped PM2 and Node with SIGTERM. The deploy took 90 s longer, but
  the result was correct.
- The first rollback started the legacy clock and nginx at once. PM2 did
  not start, and the legacy page gave 502. Each `pm2 resurrect` reached the
  limit of 90 s, and systemd started it again in a loop. It used about 31%
  of the CPU next to the legacy clock (54%).
- With the legacy clock on the core, `node -e 0` took 55 s, and a load of
  the PM2 module took 77 s. For a test, a drop-in gave PM2 a limit of
  5 minutes. PM2 started the app 4 min 57 s after the start, and then
  reached the limit. A start of PM2 runs Node three times.
- The stale `~/.pm2/rpc.sock` and `pub.sock` from the killed PM2 daemon were
  removed. They were not the cause. PM2 made new ones at its next start.
- The rollback now gives PM2 a start limit of 15 minutes and starts it with
  no wait. The test drop-in of 5 minutes is removed from the Pi.
- The changed rollback ended in 63 s. The legacy clock and nginx were
  active at its end, and the drop-in gave PM2 a limit of 15 minutes. PM2
  was active 3 min 25 s after the end. The legacy page gave 200 4 min 43 s
  after the end. A message through the legacy page showed on the legacy
  clock. The user saw it.
- After the checks, `gameclockd` serves the page on port 80, and the boot
  services are unchanged: the legacy clock, nginx, and PM2 start at boot.
  The rollback drop-in for PM2 stays on the Pi.
