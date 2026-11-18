# Proposal: sensor-pages

## Why

Two enviro sensor Pis on the home network record the outdoor and indoor
temperature, humidity, and particulate matter. The National Weather Service
(NWS) gives a forecast for the house. Now you must open a dashboard to see
these values. A button on the clock's web page can show them on the panel
for a short time. That is faster, and it starts the "data on the panel" work on the
roadmap.

## What Changes

- Three **pages**: `weather`, `indoor`, and `outdoor`. A page is a still
  data screen on the panel. It shows for 30 s and then the clock comes back.
  One page shows at a time. A new page request replaces the page on the
  panel.
  - `outdoor` and `indoor`: the temperature in °F, the relative humidity, and
    the US AQI from PM2.5 and PM10. Each value is rounded, and the AQI has
    the EPA category color.
  - `weather`: the temperature and the chance of rain of the current NWS
    forecast period, and its short forecast text. A text that is too wide
    for the panel scrolls.
- The daemon fetches the data **when you ask for a page**, not on a timer. When
  no page is requested, the daemon does no work for pages.
  - A new `fetch` thread runs the system `curl` for each request. It sends the
    body to the loop as an event. The loop does not wait for the network.
  - The daemon parses the JSON with `serde_json`, into typed values for the
    core.
  - The page shows `--` in place of each value after a failed fetch or a
    bad body. It also shows `--` for a reading older than 15 min.
- A new control command, `page <name>`. The `status` reply ends with
  `page=<name>` or `page=none`.
- **BREAKING (status format):** the `status` reply has one more field at the
  end. The web page reads the reply, and it changes in this change.
- The web page has one button for each page that has a URL in the config.
  The buttons post to `/page`.
- A new optional `[pages]` config section gives the URL of each page. There
  is no built-in URL. The addresses of the sensor Pis and the NWS grid point
  are data about the house. They go only in `/etc/gameclockd/config.toml` on
  the Pi, never in this public repo. `deploy/config.toml` has
  examples in comments, with placeholder hosts.
- Priorities: a page is above a message and below `bedtime`. A page
  interrupts a message, and the message restarts after the page.
  `bedtime` and `panic` interrupt a page. `clear` removes a page.

## Capabilities

### New Capabilities

- `sensor-pages`: the pages and the fetch on request. The reading and
  forecast values, and the AQI. The rules for old and missing data. The page
  layout, duration, and priority.

### Modified Capabilities

- `command-input`: the `status` reply ends with `page=<name>`. The unknown-command
  reply names the `page` command.
- `web-control`: the page has page buttons. A new `POST /page` action.
- `daemon-config`: a new `[pages]` section.
- `alerts`: the priority order includes the page, between a message and
  `bedtime`.

## ADRs

- Uses ADR-0005 and ADR-0006 (pure core, the daemon injects input). Uses
  ADR-0012 and ADR-0013 (scenes, render on change). Uses ADR-0016 (event
  channel), ADR-0017 (message queue), and ADR-0018 (alert priorities). Uses
  ADR-0019 (config file) and ADR-0020 (web page).
- **New ADR-0021: Page data comes from the system `curl` in a fetch thread,
  and `serde_json` parses it.** The ADR records the new process boundary
  (a child process) and the new dependency. It also records the deployment
  assumption (`curl` and the CA certificates on the Pi) and the license
  reason. A Rust TLS stack is not used. Recent `ring` is `Apache-2.0 AND ISC`,
  and our policy does not permit Apache-2.0-only code.

## Performance

- **Idle:** no change. The daemon does nothing for pages until a request
  comes.
- **One request:** one `curl` child process. Measured on the Pi Zero W on
  2026-11-17, with `gameclockd` running:
  - LAN `realtime.json`: 0.57 s wall time and 0.18 s CPU. The body is
    320 bytes.
  - NWS forecast over HTTPS: 0.79 s wall time and 0.28 s CPU. The body is
    13.5 KB.
- **On the panel:** the `outdoor` and `indoor` pages are still. They present
  once. The `weather` page presents at the message scroll rate only if its
  text is wider than the panel. That rate is already measured for messages.
- **To measure in this change:**
  - The refresh rate while a fetch runs, with `scripts/measure.sh`, because
    `curl` shares the core with the refresh thread.
  - The daemon RSS before and after 10 page requests, to show that the
    memory does not grow.

## Out of Scope

- A timer or rotation that shows pages without a request, and pages on the
  panel at all times. These come with `panel-layout` later.
- Icons and new fonts (`text-fonts`, `icons`). This change uses the fonts
  that the daemon has now.
- A barometric forecast from the pressure history, and the hourly NWS
  forecast.
- °C, and a config key for the unit.
- A `page stop` command. `clear` removes a page.
- Changes to the sensor Pis or to the `outdoor-aqi` repo.
- A Rust HTTP or TLS client.
