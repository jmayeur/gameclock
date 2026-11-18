# Design

## Context

- **The data sources** (read-only checks on 2026-11-17):
  - Each enviro sensor Pi serves `GET /realtime.json` on port 80. The body is
    `{"data":[{...}]}` with one row, about 320 bytes. The fields that we use
    are `time` (epoch seconds), `output_temp` (°F), `output_humidity` (%),
    `P25`, and `P10` (µg/m³). The two Pis use the same schema.
  - The NWS daily forecast (`api.weather.gov/gridpoints/<office>/<x>,<y>/forecast`)
    is about 13.5 KB of JSON. It needs HTTPS and a `User-Agent` header, and
    no key. We use `properties.periods[]`: `endTime`, `temperature`,
    `temperatureUnit`, `probabilityOfPrecipitation.value`, and
    `shortForecast`.
  - The `outdoor-aqi` dashboard computes the AQI in the browser. The sensor
    Pis do not serve it.
- **On the clock Pi:** `curl` 7.64.0 with OpenSSL 1.1.1n and
  `ca-certificates` 20200601 fetch the NWS forecast with status 200. See
  proposal.md for the measured cost of each fetch.
- **In the daemon now:**
  - The loop waits in `ChannelWait` for `Event`s from the signal, control,
    and web threads (ADR-0016). Tests drive the loop with a `ScriptedWait`.
  - The `Board` in the core decides what shows above the base scene. It has
    `Showing::{Nothing, Message, Alert}` and the priorities `BASE` 0 and
    `MESSAGE` 20. The built-in alerts are `bed-flash` 10, `bedtime` 30, and
    `panic` 40 (ADR-0018).
  - The web thread asks the loop for the alert names with the internal
    command `AlertNames` (ADR-0020).
  - The service starts as root. The driver drops to the `daemon` user after
    it initializes, so a child process runs as `daemon`.

## Goals / Non-Goals

**Goals:**

- Keep `gameclock-core` pure. The core receives typed values and the time. It
  never sees a URL, bytes, or JSON.
- Make every spec scenario a host test, with a fake fetcher and scripted
  fetch results. Only the two on-device scenarios need the Pi.
- Add no work to the idle loop and no allocation to the hot path. A page
  makes its text lines once, when it starts.

**Non-Goals:**

- A general HTTP client, retries, caching, or redirects.
- A page layout system. The two layouts are fixed code in the page scene.

## Decisions

### 1. The system `curl` in a `fetch` thread (ADR-0021)

The loop sends a `Job { seq, page, url }` to a `fetch` thread through a
channel. The thread runs one `curl` child for each job, and sends
`Event::Fetched { seq, page, body }` back on the event channel. `body` is
`Result<Vec<u8>, String>`. ADR-0021 gives the reasons for `curl` instead of
a Rust TLS client.

The arguments are fixed. The URL is one argument after `--`, and no shell
runs:

```
curl --silent --show-error --fail --max-time 10 --proto =http,https
     --user-agent gameclockd/<version> -- <url>
```

The thread reads at most 256 KiB + 1 byte of standard output. If it reads
more, it kills the child and the job fails. `--max-filesize` is not
sufficient, because a server can send a body with no length.

- Alternative: a thread for each job. One thread is sufficient, because the
  newest request wins and fetches are rare.
- Alternative: `std::net::TcpStream` for the LAN pages and `curl` only for
  NWS. Two paths need two sets of tests. `curl` costs 0.18 s of CPU for a
  LAN fetch, and that is acceptable for a request that comes from a button.

### 2. A `Fetch` trait for tests

`run` takes a `&mut dyn Fetch` with one method, `start(job)`. The daemon
gives a `ChannelFetch` that sends to the `fetch` thread. Tests give a
`RecordingFetch` that keeps the jobs. `ScriptedWait` has a new step that
returns `Wake::Fetched`, so a test sets the time and the body of each
result.

### 3. The newest request wins with a sequence number

The loop keeps `pending: Option<(u64, PageName)>`. Each accepted `page`
command increments `seq` and replaces `pending`. A `Fetched` event with a
different `seq` is discarded. A second request for the same page while its
fetch runs starts a new fetch. This is simple, and a double tap costs one
more fetch.

### 4. JSON parsing in the daemon, values in the core

The daemon `pages` module parses with `serde_json` into small structs. Each
field is an `Option`, and other fields are ignored. It then makes core
values:

- `Reading { time: Timestamp, temp_f: f64, humidity: f64, p25: f64, p10: f64 }`
- `Forecast { periods: Vec<Period> }`, and
  `Period { end: Timestamp, temp_f: f64, rain: u8, text: String }`. A `C`
  unit is converted to °F here.

A body that does not parse, or that has a missing field, gives `None`.
The daemon then writes the stderr line. The rules that need the time are in
the core, because the core has the time. They are the 15 min age limit and
the choice of the current period. Each page scene starts from `PageData::{Reading(Option<Reading>), Forecast(Option<Forecast>)}`.

### 5. The core `page` module

- `PageName { Weather, Indoor, Outdoor }`, with `name()` and `parse()`.
- `aqi(p25, p10) -> u16` and `aqi_color(u16) -> Rgb888`. These are pure
  functions with the EPA tables as `const` arrays. Each spec value is a
  table test.
- `PageScene`: a `Scene` that ends 30 s after its start. It makes its
  lines once, at `new`.
  - **Reading layout:** `FONT_6X10`, three lines at the top-left positions
    (1, 0), (1, 11), and (1, 22). The lines use the color of the schedule,
    and the AQI number uses its category color.
  - **Weather layout:** `FONT_7X13_BOLD`. The first line is at (1, 1). The
    second line has its top at y = 17. It is centered if it fits, or it
    scrolls with the `ScrollPath` math of messages.
  - `next_change`: the end time for a still page, and the next 40 ms step
    for a scrolling page. The compositor already wakes at schedule changes.
- These layouts must be checked in `cargo sim` before the review. The
  positions can change with no change to the specs.

### 6. The board shows a page

- `Showing::Page(PageName)` and `Priority::PAGE` = 25. That is above a
  message (20) and below `bedtime` (30).
- `Board::page_blocker() -> Option<Blocker>` gives the item that blocks a
  page. The loop asks it before a fetch starts, and the board checks again
  when the data arrives.
- `Board::page(scene, name, now, compositor) -> Result<(), PageRefused>`. It
  replaces a page, and it puts an interrupted message back in the queue,
  the same as an alert.
- `Blocker` has a new `Page` variant. The text of `Refused` is then
  `A page on the panel has the same or a higher priority.`
- `clear` removes a page. `stop_alert` does not.

### 7. Config, commands, and the web page

- `[pages]` is a `PagesSection` with `deny_unknown_fields` and three
  optional `Value<String>` keys. `Loaded` has a new field `pages: PageUrls`. `reload`
  replaces them.
- `Command::Page(Option<String>)` comes from `page` and `page <name>`.
  `Command::PageNames` is internal, the same as `AlertNames`.
- `status_reply` takes `page: Option<PageName>`. The web status line shows
  it.
- `POST /page` with `name=<page>` sends `Command::Page`. `GET /` asks
  `PageNames` and shows one button for each configured page, above the alert
  buttons.

## Risks / Trade-offs

- [A `curl` fetch uses 0.18–0.28 s of CPU on the same core as the refresh
  thread] → The on-device scenario measures the refresh rate during
  fetches. If the rate falls by more than 10%, start `curl` with `nice 10`.
  The fetch is not urgent.
- [A rollback to an older binary when the config has a `[pages]` section.
  `deny_unknown_fields` stops the start with status 2.] → The migration plan
  says to remove the section before a rollback. The CHANGELOG says it.
- [Buster is end-of-life, so its CA bundle receives no updates. A future NWS
  certificate can fail] → The page shows `--`, and the stderr line has the
  `curl` error. Out of scope here: a fix is a reflash (ADR-0007) or a
  newer CA bundle.
- [NWS changes its JSON or is slow] → Each field is optional. The 10 s limit
  and the `--` page cover a slow or changed service.
- [A sensor Pi has the wrong time] → A reading that seems older than 15 min
  shows `--`. That is the right result for data that we cannot trust.
- [The URLs are data about the house] → They go only in the config on the
  Pi. The tests use `example` hosts, and `deploy/config.toml` has placeholder
  comments.

## Migration Plan

1. Deploy with `scripts/deploy.sh`. The deploy does not replace
   `/etc/gameclockd/config.toml`, so the page buttons do not show yet.
2. On the Pi, add a `[pages]` section with the real URLs to
   `/etc/gameclockd/config.toml`. Then run `sudo gameclockd ctl reload`.
3. Rollback: remove the `[pages]` section first, and then run
   `scripts/deploy.sh` with the older binary.
