# ADR-0021: Page data comes from the system curl in a fetch thread, and serde_json parses it

- **Status:** Accepted
- **Date:** 2026-11-17
- **Related:** ADR-0003, ADR-0006, ADR-0007, ADR-0010, ADR-0016, ADR-0019,
  ADR-0020. OpenSpec change `sensor-pages`.

## Context

The `sensor-pages` change shows data screens on the panel when a person asks
for them. The data comes from three URLs:

- Two enviro sensor Pis on the home network. Each serves a small JSON body
  (about 320 bytes) over plain HTTP.
- The National Weather Service (NWS) forecast API. It serves about 13.5 KB of
  JSON over HTTPS only, and it refuses requests with no `User-Agent`.

So the daemon needs an HTTP client that can do HTTPS, and a JSON parser.

The project is GPL-2.0-or-later, and `deny.toml` does not permit
Apache-2.0-only code (ADR-0003). The usual Rust TLS stack is `rustls` with
`ring` or `aws-lc-rs`. Recent `ring` releases have the license
`Apache-2.0 AND ISC`, which our policy does not permit. `aws-lc-rs` also
has Apache-2.0 and OpenSSL license parts.

The Pi runs Raspbian 10 Buster (ADR-0007). It has `curl` 7.64.0 with
OpenSSL 1.1.1n and `ca-certificates` 20200601. On 2026-11-17, that `curl`
fetched the NWS forecast with status 200. We measured each fetch on the Pi
while `gameclockd` ran:

| Fetch | Wall time | CPU (user + sys) | Body |
|---|---|---|---|
| Sensor Pi, HTTP on the LAN | 0.57 s | 0.18 s | 320 bytes |
| NWS forecast, HTTPS | 0.79 s | 0.28 s | 13.5 KB |

A fetch happens only when a person presses a button, so the rate is low.

## Decision

- **The system `curl` does each fetch, in a child process.** A `fetch`
  thread receives jobs from the loop through a channel. For each job, it
  runs `curl` with fixed arguments and no shell. The URL is one argument
  after `--`. `--proto =http,https` refuses other schemes. `--max-time 10`
  limits the time, and the thread reads at most 256 KiB of the body. The
  `User-Agent` is `gameclockd/<version>`.
- **The result comes back on the event channel of ADR-0016.** The loop never
  waits for the network. A sequence number lets the loop discard a result
  that is no longer the newest request.
- **`serde_json` parses the body in the daemon.** It is `MIT OR Apache-2.0`.
  Its dependencies `itoa`, `memchr`, and `zmij` pass `deny.toml`. The daemon
  gives the core typed values only, so the core stays pure (ADR-0006).
- **The config gives the URLs** (ADR-0019). The repo has no built-in URL,
  because the addresses and the NWS grid point are data about the house.

## Consequences

- No TLS code in our binary, and no license problem. The HTTPS security of
  a fetch is the security of the Pi's `curl` and OpenSSL.
- The daemon now starts a child process. The child runs as the `daemon`
  user, because the LED driver drops the privileges of the process before
  the first fetch (ADR-0010).
- The Pi must have `curl` and its CA certificates. Buster receives no updates,
  so a future change of the NWS certificate chain can make the weather page
  fail. The page then shows dashes, and the daemon writes the `curl` error.
- Each fetch costs 0.18–0.28 s of CPU on the core that the refresh thread
  uses. The `sensor-pages` change measures the refresh rate during fetches.
- On a dev machine, `curl` is also present, so the simulator build can fetch
  real data.

## Alternatives considered

- **`rustls` with `ring` or `aws-lc-rs`.** The license is not permitted by
  ADR-0003, and the cross build for ARMv6 gets larger and slower.
- **`rustls` with a pure-Rust crypto provider.** Less mature, and a large
  dependency tree for three URLs.
- **Link `libcurl` or OpenSSL.** It adds FFI and `unsafe` outside the driver
  crate, and the linked license terms need a review.
- **`std::net::TcpStream` for the LAN pages, and `curl` only for NWS.** Two
  paths need two sets of tests. The CPU that one `curl` costs is acceptable
  for a button press.
- **A proxy on another machine that turns HTTPS into HTTP.** It moves the
  work to a second service that must run all the time.
- **A hand-written JSON reader.** Less code to compile, but our own parser
  for an outside API that can change. `serde_json` is small and widely used.
