# ADR-0020: The daemon serves the web page with its own small HTTP server

- **Status:** Accepted
- **Date:** 2026-11-16
- **Related:** ADR-0003, ADR-0006, ADR-0010, ADR-0016, ADR-0017, ADR-0018.
  OpenSpec change `web-control`.

## Context

The family controls the clock from a web page. On the Pi, nginx listens on
port 80 and sends each request to a Node 10 server on port 3000. PM2 keeps
Node running. Node writes a command to a file that only the legacy clock
reads. Node no longer ships builds for ARMv6.

On the Pi, Node uses 42.5 MB of RSS, PM2 58 MB, and nginx about 5 MB. The Pi
has 430 MB of usable memory. Node used 3.7% of the single core on average,
and the LED driver's refresh thread shares that core.

The daemon already gets commands from a Unix socket thread through an event
channel (ADR-0016). The loop answers each command with one line. The service
starts as root, and the LED driver drops the privileges after it starts
(ADR-0010).

The page needs only a few routes, form posts, and small bodies. The server
must not let one client on the LAN use all the memory of the Pi or block
the render loop.

We read the source of two HTTP crates:

- **`tiny_http` 0.12.0** (the last release is from 2022). It reads a header
  line into a buffer with no limit. It sets no read timeout. Each open
  connection gets its own thread, with no limit on the number of threads.
  One client can send a header line that does not end, and the memory then
  grows until the Pi has none left.
- **`tiny_http_dh` 0.12.4**, a fork with releases from September to November
  2026. It limits the thread pool. It still has no header limit and no read
  timeout for plain HTTP. One person maintains it.

Both pass our license and advisory checks.

## Decision

- **Our own HTTP/1.1 subset on `std`.** A `web` thread accepts on a
  `std::net::TcpListener`. A pure parser in `gameclockd` reads the request
  line, the headers, and a body with `Content-Length`. No new dependency.
- **Hard limits for each request.** The request line and the headers have
  8 KiB, and the body has 1 KiB. The whole request has 2 s from `accept`.
  A write also has a limit of 2 s. Each response has `Connection: close`.
  The thread serves one connection at a time.
- **The web thread uses the event channel of ADR-0016.** Each action sends
  the same `Command` as the socket and waits for the reply line. The loop
  does not know where a command came from. The page asks the loop for the
  status and for the names of the alerts.
- **Port 80, bound as root.** The default address is `0.0.0.0:80`.
  `--http=<address>` gives a different address, and `--http=off` turns the
  listener off. The daemon binds the port before the driver drops the
  privileges. If the bind fails, the daemon writes a warning and runs with
  no web page.
- **No login on the LAN, and an `Origin` check.** Any device on the LAN can
  use the page, the same as the legacy page. A `POST` with an `Origin`
  header must name the same host and port as the `Host` header. The daemon
  sends no CORS headers.
- **nginx, PM2, and Node stop on the Pi.** The deploy stops them, and the
  boot option disables them. The rollback starts them again with the legacy
  clock. The deploy does not remove their files.

## Consequences

- The Pi gets back about 106 MB of RSS and the CPU that Node used.
- The page controls `gameclockd`, with the same commands and the same error
  text as the socket.
- We own the parser. A bug in it is our bug. Each limit and each refusal
  needs a test, and a test with random bytes checks that the parser never
  panics.
- One slow client can hold the page for 2 s. A second client then waits.
  The render loop does not wait for the web thread.
- A person on the LAN can send commands with no login, as before. A web page
  on the internet cannot send commands through a browser on the LAN.
- No HTTPS. The page is only for the home network.
- If a later change needs keep-alive, many clients at a time, or HTTPS, this
  decision must change. We would notice it as slow pages or a need for a
  login.

## Alternatives considered

- **`tiny_http` 0.12.** Less parser code, but no limit on header size, no
  read timeout, and no limit on threads. One bad client can stop the clock.
- **`tiny_http_dh`.** It limits the threads, but not the header size or the
  read time. Its releases are new and it has one maintainer.
- **`tiny_http` behind our own guard thread.** A guard can limit each
  connection and then send the bytes to `tiny_http` on localhost. That is
  more code than a parser, and it doubles the sockets.
- **An async server (`axum`, `hyper`, `tokio`).** Many crates and a large
  binary for three routes on one core.
- **Keep nginx and put the daemon on port 3000.** nginx then keeps about
  5 MB and one more hop. It also needs its own config and upgrades.
- **A token or a login.** The family uses the page from phones on the home
  network. A token in a bookmark adds steps and gives little protection on
  plain HTTP.
