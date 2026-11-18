# sensor-pages Specification

## Purpose

Defines the data pages of the daemon: the `weather`, `indoor`, and `outdoor`
screens. It covers the page command, the fetch on request, the values, and
their layout. It also covers the AQI, the rules for missing or old data, and
the page duration and priority.

## Requirements

### Requirement: The page command
The command `page <name>` SHALL request the page `weather`, `indoor`, or
`outdoor`. If the config gives a URL for that page, the daemon SHALL start a
fetch of the URL and reply `ok`. The reply SHALL not wait for the fetch.

#### Scenario: Request a page
- **WHEN** the config gives a URL for `outdoor`, and the command `page outdoor` arrives
- **THEN** the reply is `ok`, and one fetch of that URL starts

#### Scenario: Unknown page
- **WHEN** the command `page garage` arrives
- **THEN** the reply is `error: unknown page 'garage'. The pages are weather, indoor, and outdoor.`, and no fetch starts

#### Scenario: No page name
- **WHEN** the command `page` arrives
- **THEN** the reply is `error: the page command needs a name: weather, indoor, or outdoor.`

#### Scenario: Page with no URL
- **WHEN** the config gives no URL for `weather`, and the command `page weather` arrives
- **THEN** the reply is `error: the page 'weather' has no URL in the config.`, and no fetch starts

### Requirement: The daemon fetches data only on a request
The daemon SHALL fetch page data only after a `page` command. It SHALL not
fetch on a timer. The loop SHALL continue to present frames and answer
commands while a fetch runs. If more requests come before their data
arrives, only the data of the newest request SHALL show.

#### Scenario: No request, no fetch
- **WHEN** the daemon runs for 10 min with URLs for all three pages, and no `page` command arrives
- **THEN** no fetch starts

#### Scenario: The loop does not wait
- **WHEN** a fetch for `weather` runs and has no data after 5 s, and a `status` command arrives
- **THEN** the reply comes at once, and the clock continues to update

#### Scenario: The newest request wins
- **WHEN** `page indoor` arrives, then `page outdoor` arrives, and then the data of `indoor` arrives
- **THEN** the `indoor` data does not show
- **AND** the `outdoor` page shows when its data arrives

### Requirement: A fetch has limits
A fetch SHALL stop after 10 s. It SHALL refuse a body over 256 KiB. It SHALL
send the HTTP header `User-Agent: gameclockd/<version>`, because the NWS
service refuses requests with no User-Agent. A stopped or refused fetch SHALL
count as a failed fetch.

#### Scenario: Slow server
- **WHEN** the server of a page URL sends no response for 10 s
- **THEN** the fetch stops, and the page shows with no data

#### Scenario: Body too large
- **WHEN** the server sends a body of 300 KiB
- **THEN** the fetch fails, and the page shows with no data

#### Scenario: User-Agent
- **WHEN** a fetch starts for any page
- **THEN** the request has a `User-Agent` header that starts with `gameclockd/`

### Requirement: A page shows for 30 s
When the data of the newest request arrives, the page SHALL show at that
time T. At T + 30 s, the page SHALL end, and the next queued message or the
base scene SHALL show. A new page SHALL replace the page on the panel.

#### Scenario: Page ends after 30 s
- **WHEN** the `outdoor` data arrives at time T, and nothing else arrives
- **THEN** the `outdoor` page shows from T, and the clock shows at T + 30 s

#### Scenario: A page replaces a page
- **WHEN** the `outdoor` page shows from T, and the `indoor` data arrives at T + 10 s
- **THEN** the `indoor` page shows from T + 10 s, and the clock shows at T + 40 s

#### Scenario: Status during a page
- **WHEN** the `indoor` page shows, and a `status` command arrives
- **THEN** the reply ends with `page=indoor`

### Requirement: A reading page shows temperature, humidity, and AQI
The `outdoor` and `indoor` pages SHALL read the newest reading. Its
`output_temp` is in °F, `output_humidity` is in percent, and `P25` and
`P10` are in µg/m³. The page SHALL show three lines: the place and the
temperature, `RH` and the humidity, and `AQI` and the AQI. Each value is
rounded to an integer.

#### Scenario: Outdoor page
- **WHEN** the outdoor data is a reading of 2 min ago with `output_temp` 41.51, `output_humidity` 92.15, `P25` 5.02, and `P10` 5.86
- **THEN** the page lines are `OUT 42F`, `RH 92%`, and `AQI 28`

#### Scenario: Indoor page
- **WHEN** the indoor data is a reading of 1 min ago with `output_temp` 67.79, `output_humidity` 44.26, `P25` 1.23, and `P10` 1.23
- **THEN** the page lines are `IN 68F`, `RH 44%`, and `AQI 7`

#### Scenario: Below zero
- **WHEN** the outdoor reading has `output_temp` -3.4
- **THEN** the first line is `OUT -3F`

### Requirement: The AQI comes from PM2.5 and PM10
The AQI SHALL be the higher of the US EPA AQI for PM2.5 and for PM10. It
SHALL use the EPA breakpoints of 2024. PM2.5 is truncated to 0.1 µg/m³ and
PM10 to 1 µg/m³. An AQI over 500 SHALL show as 500. The AQI text SHALL have
the color of its EPA category.

#### Scenario: AQI values
- **WHEN** the reading has `P25` and `P10` of (5.02, 5.86), (1.23, 1.23), (40.0, 20), (20, 200), and (600, 0)
- **THEN** the AQI is 28, 7, 112, 123, and 500

#### Scenario: Category colors
- **WHEN** the AQI is 28, 75, 112, 175, 250, and 400
- **THEN** the AQI color is `(0,228,0)`, `(255,255,0)`, `(255,126,0)`, `(255,0,0)`, `(143,63,151)`, and `(126,0,35)`

### Requirement: The weather page shows the current forecast period
The `weather` page SHALL read an NWS forecast. It SHALL use the first period
that has not ended. The first line SHALL be the temperature in °F and the
chance of rain, for example `48F 20%`. The second line SHALL be the short
forecast text.

#### Scenario: Weather page
- **WHEN** the current period has `temperature` 48, `temperatureUnit` `F`, a chance of rain of 20, and `shortForecast` `Rain`
- **THEN** the first line is `48F 20%`, and the second line is `Rain`

#### Scenario: No chance of rain
- **WHEN** the current period has no value for the chance of rain
- **THEN** the first line shows `0%`

#### Scenario: Celsius from the service
- **WHEN** the current period has `temperature` 10 and `temperatureUnit` `C`
- **THEN** the first line starts with `50F`

#### Scenario: Skip an old period
- **WHEN** the first period ended 1 h ago, and the second period has not ended
- **THEN** the page shows the second period

### Requirement: A long forecast text scrolls
The forecast text SHALL show still and centered if it fits in 64 px. If it
is wider, it SHALL scroll from right to left at 1 px each 40 ms, again and
again, until the page ends. The text SHALL have 64 characters or fewer. Each
character that is not printable ASCII SHALL show as `?`.

#### Scenario: Short text is still
- **WHEN** the short forecast is `Rain`
- **THEN** the frames at T + 1 s and at T + 20 s are the same

#### Scenario: Long text scrolls
- **WHEN** the short forecast is `Chance Rain Showers then Mostly Cloudy`
- **THEN** the text at T + 400 ms is 10 px left of its place at T

#### Scenario: Text that is not ASCII
- **WHEN** the short forecast is `Fog°`
- **THEN** the second line is `Fog?`

### Requirement: Missing or old data shows dashes
A failed fetch, a body that is not JSON of the expected form, or a reading
older than 15 min SHALL not stop the page. The page SHALL show for 30 s. Each value
SHALL show as `--`. The daemon SHALL write one line to standard error that
names the page and the reason.

#### Scenario: Fetch fails
- **WHEN** the fetch for `outdoor` fails
- **THEN** the page lines are `OUT --`, `RH --`, and `AQI --`
- **AND** standard error has a line that starts with `gameclockd: the page 'outdoor' has no data:`

#### Scenario: Old reading
- **WHEN** the indoor reading is 20 min old
- **THEN** the page lines are `IN --`, `RH --`, and `AQI --`

#### Scenario: Body is not JSON
- **WHEN** the body of the `weather` fetch is `<html>`
- **THEN** the page lines are `--F --%` and `--`

#### Scenario: No current period
- **WHEN** all periods of the forecast ended
- **THEN** the page lines are `--F --%` and `--`

### Requirement: A page has its own priority
A page SHALL have a priority above a message and below `bedtime`. A page
SHALL interrupt a message, and the message SHALL restart after the page. A
message that arrives during a page SHALL wait. `clear` SHALL remove a page.

#### Scenario: Page over a message
- **WHEN** a message scrolls, and the `outdoor` data arrives
- **THEN** the `outdoor` page shows, and the message starts again from its start after the page ends

#### Scenario: Message during a page
- **WHEN** the `indoor` page shows, and a message arrives
- **THEN** the reply is `ok`, and the message starts when the page ends

#### Scenario: Request during a higher alert
- **WHEN** `panic` shows, and `page outdoor` arrives
- **THEN** the reply is `error: the page 'outdoor' did not show. The alert 'panic' on the panel has the same or a higher priority.`, and no fetch starts

#### Scenario: Data arrives during a higher alert
- **WHEN** the reply to `page outdoor` is `ok`, and then `bedtime` starts before the data arrives
- **THEN** the `outdoor` page does not show, and standard error has a line that says so

#### Scenario: Clear removes a page
- **WHEN** the `weather` page shows, and `clear` arrives
- **THEN** the base scene shows

### Requirement: A page uses the brightness of the clock schedule
A page SHALL take its brightness and its text color from the clock schedule
at the time of each frame. Only the AQI value SHALL use its category color.

#### Scenario: Page at night
- **WHEN** the schedule gives red at 25 at 02:00, and the `indoor` page shows at 02:00
- **THEN** the frame has brightness 25, and the `IN` and `RH` lines are red

### Requirement: A fetch does not hurt the panel (on-device)
On the Pi Zero W, with the hzeller panel, a fetch SHALL not lower the
refresh rate by more than 10% while it runs. The RSS of the daemon after 10
page requests SHALL be within 1 MiB of the RSS before them.

#### Scenario: Refresh rate during a fetch
- **WHEN** `scripts/measure.sh` runs on the Pi, and a `page weather` request is sent each 5 s
- **THEN** the measured refresh rate is at least 90% of the rate with no requests

#### Scenario: Memory after requests
- **WHEN** 10 page requests finish on the Pi
- **THEN** the RSS of `gameclockd` is within 1 MiB of its RSS before the requests
