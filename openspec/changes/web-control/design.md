# Design

## Context

- **On the Pi now** (read-only checks on 2026-11-16):
  - nginx listens on port 80 and sends each request to `localhost:3000`.
  - Node 10 runs `/home/pi/nserv/server.js` on port 3000. It is the `v2`
    server in `legacy-behavior.md`. `pm2-pi.service` starts PM2 at boot, and
    PM2 starts Node.
  - RSS: Node 42,540 KiB, PM2 58,292 KiB, nginx about 5 MB. Node used 3.7%
    of the core on average over 2.5 days.
  - `free -m`: 430 MB total, 273 MB available.
- The Node server has `GET /`, `POST /message` (a form field `msg`),
  `GET /health`, and `GET /assets/main.css`. It writes `msg=<value>` to a
  file that only the legacy clock reads. It sends
  `Access-Control-Allow-Origin: *`.
- `gameclockd` has a control socket thread that parses one command line and
  sends a `Request` on the event channel (ADR-0016). The loop answers with
  one line, which starts with `ok` or `error: `. The loop waits in
  `ChannelWait`, so a command wakes it at once.
- The service starts as root. The LED driver drops to the `daemon` user
  after it initializes. The socket is made before `panel::create` for that
  reason.

## Goals / Non-Goals

**Goals:**

- Give the family the same page at the same URL (`http://<the Pi>/`), with
  control of `gameclockd`.
- Remove nginx, PM2, and Node from the running Pi, and give their memory
  back.
- Keep the render loop as it is. An idle listener costs no CPU and no wakes.
- Make each limit a test. A client on the LAN cannot use all the memory or
  hold the listener for more than 2 s.

**Non-Goals:**

- HTTPS, a login, IPv6 by default, keep-alive, or more than one connection at
  a time.
- `reload`, `clock`, and `test-pattern` on the page.
- A config editor.
- Removal of the Node files or packages from the Pi. The rollback needs them.

## Decisions

### 1. Our own HTTP/1.1 subset on `std` (ADR-0020)

The listener uses `std::net::TcpListener` and a hand-written parser. ADR-0020
gives the reasons, and why `tiny_http` is not used.

The subset:

- The request line is `<METHOD> <path> HTTP/1.0` or `HTTP/1.1`. The path is
  the part before `?`. The query is ignored.
- Header names compare with no case. Only `Host`, `Origin`, `Content-Length`,
  `Content-Type`, and `Transfer-Encoding` have a meaning. A line end is CRLF
  or LF.
- A body comes only with `Content-Length`. `Transfer-Encoding` with no
  `Content-Length` gives 411. A `Content-Length` that is not a number, or two
  different values, gives 400.
- Each response has `Content-Type`, `Content-Length`, `Connection: close`,
  `Cache-Control: no-store`, and `X-Content-Type-Options: nosniff`. The page
  also has `Content-Security-Policy: default-src 'none'; style-src
  'unsafe-inline'; form-action 'self'; frame-ancestors 'none'`.

### 2. Limits from the socket clock, not from the client

- The thread reads into one buffer. It stops at 8 KiB for the head and at
  1 KiB for the body. The buffer never grows past 9 KiB for each request.
- The read time is one total limit of 2 s from `accept`, not 2 s for each
  read. Before each read, the thread sets the socket timeout to the time
  that is left, as `control::read_line` does. A client that sends one byte
  each second still stops at 2 s.
- The write has a timeout of 2 s, so a client that does not read cannot
  hold the thread.
- One connection at a time: the worst wait for a second client is about
  4 s (2 s to read, 2 s for the loop). The family sends one command at a
  time, so this is acceptable. ADR-0020 records the trade-off.

### 3. Modules, with the logic in pure functions

- `http.rs`: `parse(&[u8]) -> Parse` with the states `Partial`,
  `Complete(Request)`, and `Refused(Status)`. `Response` and its bytes.
  No I/O. Unit tests feed byte slices, including a slice for each limit.
- `web.rs`: the routes, the form decoder, the HTML escape, and the page.
  `route(&Request, ask: impl FnMut(Command) -> Reply) -> Response`.
  `Reply` is `Answer(String)` or `NoAnswer`. Unit tests give a fake `ask`
  that records the commands.
- The listener thread code is in `web.rs` or `control.rs`. It does
  `accept`, the timed reads, `route`, and the write. Integration tests use
  `--http=127.0.0.1:0` and a real `TcpStream`.

`command.rs` stays the only place with the command rules. The web code
builds a `Message` with `Message::new` and maps `MessageError` to the same
error text as the socket. The text then agrees on the socket and the page.

### 4. The page gets its data from the loop

For `GET /`, the thread asks the loop two things:

- `Command::Status`: the reply `ok mode=… zone=… brightness=… messages=…
  alert=…` has a set format (command-input spec). The page parses the
  `key=value` words.
- A new internal `Command::AlertNames`: the reply is `ok` and the names of
  the current alert table, with one space between them. The socket parser
  cannot make this command, so the socket protocol does not change.

A request then sees the alert table after a `reload` with no extra code.

### 5. Form decoding and the message text

- `application/x-www-form-urlencoded`, with a `; charset=…` part permitted.
  `+` is a space. `%XX` is a byte. The result must be UTF-8. A bad `%` code
  or bad UTF-8 gives 400.
- A field that is in the body two times: the last value wins.
- `msg`: CR, LF, and tab become spaces, each run of spaces becomes one
  space, and the text is trimmed. This is the legacy rule. A text over 64
  characters gives the message error, not a cut text.
- `passes`: empty or missing means 4. `color`: empty or missing means
  `009600`. A `#` at the start is removed, so an `<input type="color">`
  value works.
- `msg=panic88` and `msg=bedtime` become `alert panic` and `alert bedtime`.
  Other text is a message.

### 6. The `Origin` check

A `POST` with an `Origin` header must have the same host and port as the
`Host` header. The scheme must be `http`. An `Origin` of `null` is refused.
A `POST` with no `Origin` passes, so `curl` and old clients work. A browser
always sends `Origin` on a cross-site form post. Thus a page on the internet
cannot use a browser on the LAN to send commands (ADR-0020).

### 7. Arguments and start order

- `Args` gets `http: Option<SocketAddr>`. The default is
  `Some(0.0.0.0:80)`. `--http=off` gives `None`. A value that does not parse
  as a `SocketAddr` is a usage error that gives the example
  `--http=0.0.0.0:80`.
- `main.rs` binds the listener after the control socket and before
  `panel::create`, while the process is still root. Then it starts the
  `web` thread with `THREAD_STACK`. On an error it writes
  `gameclockd: cannot listen on http://<address>: <error>. The daemon runs
  with no web page.` and continues.
- After the bind, it writes `gameclockd: web page at http://<address>/`
  with the real port.

### 8. Deployment

- `deploy/gameclockd.service` does not change. Root binds port 80 before
  the driver drops the privileges.
- `scripts/deploy.sh`:
  - each install: `sudo systemctl stop nginx pm2-pi` before the restart of
    `gameclockd`
  - `--enable`: also `disable` for `clock.svc.service`, `nginx`, and
    `pm2-pi`
  - `--rollback`: `disable --now gameclockd`, then `enable --now
    clock.svc.service nginx pm2-pi`
- No `Conflicts=` on nginx. A package upgrade can restart nginx, and a
  `Conflicts=` would then stop the clock. A port conflict only stops the
  web page, and a deploy can stop nginx again.

## Risks / Trade-offs

- **Our parser can have a bug that a mature crate does not have** → the
  subset is small. Each limit and each refused form has a unit test. A fuzz
  test of `http::parse` with random bytes checks that it never panics and
  never grows the buffer past the limit.
- **One slow client blocks the page for 2 s** → acceptable for a home LAN
  with one user at a time. The clock does not wait for the web thread.
- **Anyone on the LAN can send commands** → the same as now. The user
  decided this. The `Origin` check stops cross-site posts from a browser.
- **A weekly auto-update restarts nginx** (`/home/pi/auto-update.sh` runs on
  Saturday) → a disabled unit stays down after an upgrade in Debian. We
  check the port after the first Saturday and note it in the measurements.
- **Port 80 on a dev machine** → a host build as a normal user can fail to
  bind port 80. The daemon then runs with no web page. Tests and
  `--simulator` runs use `--http=127.0.0.1:0` or a high port.
- **Rollback needs nginx, PM2, and Node** → the deploy does not remove them.
  A later change can remove them after some weeks of use.

## Migration Plan

1. Merge with the listener on by default. A plain deploy stops nginx and
   PM2, and `gameclockd` takes port 80. Boot still starts the legacy stack.
2. Check the page from a phone. Measure.
3. `scripts/deploy.sh --enable` makes `gameclockd` the boot service.
4. Rollback: `scripts/deploy.sh --rollback` starts the legacy clock, nginx,
   and PM2.

## Open Questions

- None for the behavior. The PM2 unit name `pm2-pi.service` comes from the
  Pi. The deploy test sets it, and the on-device check shows the real name.
