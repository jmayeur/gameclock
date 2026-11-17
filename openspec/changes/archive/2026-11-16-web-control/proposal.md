# Proposal

## Why

The family controls the clock from a web page. On the Pi, nginx on port 80
sends each request to a Node 10 server on port 3000, and PM2 keeps Node
running. Node no longer ships builds for ARMv6. The three processes use about
106 MB of RSS on a Pi with 430 MB of usable memory. The Node server writes to
a file that only the legacy clock reads, so the web page cannot control
`gameclockd`. This change puts the web page in the daemon and removes nginx,
PM2, and Node from the Pi.

## What Changes

- Add an **HTTP listener** in `gameclockd`. It runs in its own thread, like
  the control socket (ADR-0016). It accepts one connection at a time. It
  reads one request, sends one response, and closes the connection.
- The default address is `0.0.0.0:80`. `--http=<address>:<port>` gives a
  different address, and `--http=off` turns the listener off. The daemon
  binds the port at start, while it is still root, before the driver drops
  the privileges.
- If the daemon cannot listen, it writes a warning and runs with no web page.
  This is the same as for the control socket.
- **The page** (`GET /`) shows the status and has these controls:
  - a message field, with the passes and the color
  - one button for each alert in the current alert table
  - a clear button and a stop-alert button
- **The actions** are form posts: `POST /message`, `POST /alert`,
  `POST /alert/stop`, and `POST /clear`. Each action sends the same command
  to the loop as the control socket. The response is the page with the
  reply of the loop.
- **Legacy clients continue to work:** `POST /message` with the field `msg`
  shows a message. The values `panic88` and `bedtime` start the alerts
  `panic` and `bedtime`, as on the legacy page.
- `GET /health` gives `{"status":"ok"}` if the loop answers, and status 503
  if it does not.
- **Limits** for each request: 8 KiB for the request line and the headers,
  1 KiB for the body, and 2 s for the whole request. A request over a limit
  gets an error status and the connection closes.
- **Access:** any device on the LAN can use the page, with no login, the
  same as the legacy page. The daemon sends no CORS headers. It refuses a
  `POST` with an `Origin` header from a different host. Then a web page on
  the internet cannot send commands through a browser on the LAN.
- **Deploy:** `scripts/deploy.sh` stops nginx and PM2 before it restarts
  `gameclockd`. `--enable` also disables them at boot. `--rollback` starts
  and enables them again with the legacy clock.
- No change to the control socket protocol.

## Capabilities

### New Capabilities

- `web-control`: the HTTP listener, its address, the page, the actions, the
  legacy form values, the health check, the limits, the status codes, and
  the `Origin` check.

### Modified Capabilities

- `daemon-lifecycle`: the `--http=<address>` and `--http=off` arguments.
- `pi-deployment`: the deploy stops nginx and PM2, `--enable` disables them
  at boot, and `--rollback` starts and enables them again.

## ADRs

- Uses ADR-0003 (license), ADR-0005 (pure core), ADR-0006 (render on
  change), and ADR-0010 (systemd service).
- Uses ADR-0016 (command socket and event channel), ADR-0017 (messages),
  ADR-0018 (alerts), and ADR-0019 (config).
- **New ADR-0020: The daemon serves the web page with its own small HTTP
  server.** It adds a protocol boundary and a thread. It decides that the
  HTTP code is ours, on `std`, with no new dependency. It records why we
  did not use `tiny_http`. It also decides port 80 and the bind as root. It
  decides access with no login and an `Origin` check, and the removal of
  nginx, PM2, and Node from the Pi.

No new dependency.

## Expected effect on the Pi

- **Memory:** the Pi gets back about 106 MB of RSS: Node 42.5 MB, PM2
  58 MB, and nginx about 5 MB. The RSS of `gameclockd` grows by less than
  200 KiB for the thread stack and the buffers of one request.
- **CPU:** Node used 3.7% of the core on average since its start. That CPU
  goes back to the driver's refresh thread. The web thread blocks in
  `accept` and wakes only for a request. The idle loop does not change.
- **Binary:** we expect less than 60 KB of growth for the parser, the routes,
  and the page.
- **Measurement:**
  - the RSS of Node, PM2, and nginx before the switch, and the free memory
    before and after
  - `scripts/measure.sh` for 60 s with the clock only and the listener on.
    The main thread and the web thread must stay below 1% CPU. Each must
    have 10 or fewer voluntary context switches.
  - the time of `GET /` and `POST /message` from a laptop on the LAN, 5 runs
    each, against the same requests to the legacy Node server
  - the binary size and the RSS against `main`

## Out of scope

- HTTPS, a login, or a token.
- IPv6. The default address is IPv4 only. `--http` can give an IPv6
  address.
- `reload`, `clock`, and `test-pattern` on the web page. They stay socket
  commands.
- A JSON API for each command. The only JSON route is `/health`.
- Keep-alive connections, chunked request bodies, and more than one
  connection at a time.
- The config editor on the web page.
- A stylesheet route. The page has its style in the HTML.
- A change to the control socket or to the `ctl` client.

## Impact

- `gameclockd`: new modules for the HTTP request parser and response writer,
  the routes and the page, and the listener thread. New arguments in
  `parse_args`. An internal request for the alert names, which the socket
  parser cannot make. `main.rs` binds the listener before the panel.
- `gameclock-core`: no change.
- `deploy/` and `scripts/deploy.sh`: stop, disable, and enable nginx and
  `pm2-pi.service`. `scripts/test-deploy.sh` checks the commands.
- Docs: ADR-0020, the ADR index, a back link in ADR-0016,
  `architecture.md`, `glossary.md`, `roadmap.md`, `legacy-behavior.md` (the
  Pi facts), `CHANGELOG.md`, the README, and `docs/hardware/pi-zero-w.md`.
