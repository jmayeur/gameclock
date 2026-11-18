# web-control Specification

## Purpose

Defines the web page of the daemon. It covers the HTTP listener and its
address, the page, the actions, and the legacy form values. It also covers
the health check, the limits for each request, and the origin check.

## Requirements

### Requirement: The daemon listens for HTTP
At start, before it creates the panel, the daemon SHALL listen for HTTP on a
TCP address. The default address is `0.0.0.0:80`. A thread SHALL accept one
connection at a time. For each connection, it SHALL read one request, write
one response with `Connection: close`, and close the connection. The daemon
SHALL write the address that it listens on to standard error.

#### Scenario: Listen on a given address
- **WHEN** the host build of `gameclockd` starts with `--http=127.0.0.1:0`
- **THEN** standard error has a line with `http://127.0.0.1:` and the port that the system gave
- **AND** a `GET /health` to that port gets status 200

#### Scenario: Off
- **WHEN** the host build of `gameclockd` starts with `--http=off`
- **THEN** the daemon listens on no TCP port, and standard error does not have `http://`

#### Scenario: One response, then close
- **WHEN** a client sends `GET /health` with `Connection: keep-alive`
- **THEN** the response has the header `Connection: close`, and the daemon closes the connection after the response

### Requirement: The clock runs without the web page
If the daemon cannot listen on the address, it SHALL write a warning that
names the address and the error. It SHALL then run with no web page.

#### Scenario: Address in use
- **WHEN** a test socket listens on a port, and the host build of `gameclockd` starts with `--http=127.0.0.1:<that port>`
- **THEN** standard error has a warning that names the address and says that the daemon runs with no web page
- **AND** the daemon presents a frame and stops with status 0 on SIGTERM

### Requirement: The page shows the status and the controls
`GET /` SHALL give status 200 and an HTML page in UTF-8. The page SHALL show
the mode, the brightness, the number of messages, the alert on the panel,
and the page on the panel.
It SHALL have a message form with the text, the passes, and the color. It
SHALL have one button for each alert in the current alert table. It SHALL
have one button for each page that has a URL in the config. It SHALL have a
clear button and a stop-alert button. The page SHALL work with no JavaScript.

#### Scenario: Page with the built-in alerts
- **WHEN** the loop has the built-in tables, and a client sends `GET /`
- **THEN** the status is 200, and the type is `text/html; charset=utf-8`
- **AND** the page has a form that posts to `/message` with the fields `msg`, `passes`, and `color`
- **AND** the page has three forms that post to `/alert` with `name` set to `panic`, `bedtime`, and `bed-flash`
- **AND** the page has forms that post to `/clear` and to `/alert/stop`

#### Scenario: Page shows the status
- **WHEN** the clock is the base scene at brightness 60, with two messages and no alert, and a client sends `GET /`
- **THEN** the page has the text `clock`, `60`, `2`, and `none` in its status line, and `none` for the page

#### Scenario: Page after a reload
- **WHEN** a `reload` makes the alert table `dinner` only, and a client then sends `GET /`
- **THEN** the page has one alert button, with `name` set to `dinner`

#### Scenario: Text from data is escaped
- **WHEN** a client posts `name=%3Cb%3E%26%22%27` to `/alert`, and the loop replies with the unknown-alert error that contains the name
- **THEN** the page has `&lt;b&gt;&amp;&quot;&#39;` and does not have `<b>&"'`

#### Scenario: Page buttons from the config
- **WHEN** the config gives URLs for `outdoor` and `indoor` only, and a client sends `GET /`
- **THEN** the page has two forms that post to `/page`, with `name` set to `outdoor` and `indoor`
- **AND** the page has no form with `name` set to `weather`

#### Scenario: No pages in the config
- **WHEN** the config has no `[pages]` section, and a client sends `GET /`
- **THEN** the page has no form that posts to `/page`

### Requirement: Each action sends a command to the loop
Each action SHALL send one command to the loop through the event channel,
the same as the control socket. `POST /message` sends `message`, and
`POST /alert` sends `alert <name>`. `POST /alert/stop` sends `alert stop`.
`POST /clear` sends `clear`, and `POST /page` sends `page <name>`. The response SHALL be the page with the reply of
the loop. The status SHALL be 200 for `ok`, 400 for an error about the
request, and 503 if the loop does not answer in 2 s.

#### Scenario: Send a message
- **WHEN** a client posts `msg=Go+team%21&passes=2&color=ff0000` to `/message`
- **THEN** the loop gets a message with the text `Go team!`, 2 passes, and the color `(255,0,0)`
- **AND** the status is 200, and the page has the reply `ok`

#### Scenario: Message with the defaults
- **WHEN** a client posts `msg=hi` to `/message`
- **THEN** the loop gets a message with the text `hi`, 4 passes, and the color `(0,150,0)`

#### Scenario: Message with white space
- **WHEN** a client posts `msg=%20a%0D%0Ab%09%09c%20` to `/message`
- **THEN** the loop gets a message with the text `a b c`

#### Scenario: Message that is too long
- **WHEN** a client posts a `msg` of 65 characters to `/message`
- **THEN** the status is 400, the page has `the message is longer than 64 characters.`, and the loop gets no command

#### Scenario: Empty message
- **WHEN** a client posts `msg=+++` to `/message`
- **THEN** the status is 400, the page has `the message has no text.`, and the loop gets no command

#### Scenario: Bad passes or color
- **WHEN** a client posts `msg=hi&passes=11` or `msg=hi&color=red` to `/message`
- **THEN** the status is 400, and the page has the `--passes` or the `--color` error of the message command

#### Scenario: Start an alert
- **WHEN** a client posts `name=panic` to `/alert`
- **THEN** the loop gets the command `alert panic`, and the status is 200

#### Scenario: Unknown alert
- **WHEN** a client posts `name=dinner` to `/alert`, and the alert table has no `dinner`
- **THEN** the status is 400, and the page has the unknown-alert reply that lists the alert names

#### Scenario: Stop and clear
- **WHEN** a client posts to `/alert/stop`, and then to `/clear`
- **THEN** the loop gets `alert stop`, and then `clear`

#### Scenario: The loop does not answer
- **WHEN** a client posts `msg=hi` to `/message`, and the loop does not answer in 2 s
- **THEN** the status is 503, and the page has `the daemon did not answer in 2 s.`

#### Scenario: Request a page
- **WHEN** a client posts `name=weather` to `/page`, and the config gives a URL for `weather`
- **THEN** the loop gets the command `page weather`, and the status is 200

#### Scenario: Unknown page from the form
- **WHEN** a client posts `name=garage` to `/page`
- **THEN** the status is 400, and the page has the unknown-page reply that lists the pages

### Requirement: Legacy form values start the alerts
`POST /message` SHALL treat the `msg` value `panic88` as the command
`alert panic`, and the value `bedtime` as `alert bedtime`. The legacy page and
legacy clients then continue to work.

#### Scenario: Legacy panic button
- **WHEN** a client posts `msg=panic88` to `/message`
- **THEN** the loop gets the command `alert panic`, and no message

#### Scenario: Legacy bedtime button
- **WHEN** a client posts `msg=bedtime` to `/message`
- **THEN** the loop gets the command `alert bedtime`, and no message

### Requirement: The health check asks the loop
`GET /health` SHALL send a `status` command to the loop. If the loop
answers, the response SHALL be status 200 with the JSON `{"status":"ok"}`.
If it does not answer in 2 s, the response SHALL be status 503 with the JSON
`{"status":"no answer"}`. The type SHALL be
`application/json; charset=utf-8`.

#### Scenario: Health with a loop
- **WHEN** a client sends `GET /health`, and the loop answers
- **THEN** the status is 200, and the body is `{"status":"ok"}`

#### Scenario: Health with no answer
- **WHEN** a client sends `GET /health`, and the loop does not answer in 2 s
- **THEN** the status is 503, and the body is `{"status":"no answer"}`

### Requirement: Each request has limits
The listener SHALL refuse a request that it cannot read in full in 2 s with
status 408. It SHALL refuse a request line and headers over 8 KiB with
status 431, and a body over 1 KiB with status 413. It SHALL send a short text
body, close the connection, and send no command to the loop.

#### Scenario: Slow client
- **WHEN** a client sends `GET / HTTP/1.1` and then nothing for 2 s
- **THEN** the response is status 408, and the connection closes in less than 3 s from the first byte

#### Scenario: Headers that do not end
- **WHEN** a client sends a request line and then 9000 bytes of one header with no line end
- **THEN** the response is status 431, and the daemon keeps no more than 8 KiB of the request head

#### Scenario: Body too large
- **WHEN** a client posts to `/message` with `Content-Length: 2000`
- **THEN** the response is status 413, and the loop gets no command

#### Scenario: A slow client does not stop the clock
- **WHEN** a client holds a connection open with no bytes for 2 s
- **THEN** the loop presents each scene change on time during that period

### Requirement: The listener refuses requests that it does not support
The listener SHALL refuse these requests, send a short text body, close the
connection, and send no command to the loop:

- a `POST` with no `Content-Length`: status 411
- a `POST` body that is not `application/x-www-form-urlencoded`: status 415
- a request that is not HTTP/1.0 or HTTP/1.1, or a bad form value: status 400
- a path that is not a route: status 404
- a method that the route does not have: status 405, with an `Allow` header

#### Scenario: Post with no length
- **WHEN** a client posts to `/message` with `Transfer-Encoding: chunked` and no `Content-Length`
- **THEN** the response is status 411

#### Scenario: Post that is not a form
- **WHEN** a client posts JSON to `/message` with `Content-Type: application/json`
- **THEN** the response is status 415

#### Scenario: Bad percent code
- **WHEN** a client posts `msg=%G1` or `msg=%FF` to `/message`
- **THEN** the response is status 400, and the loop gets no command

#### Scenario: Unknown path and wrong method
- **WHEN** a client sends `GET /nothing`, and then `DELETE /`
- **THEN** the first response is status 404, and the second is status 405 with `Allow: GET`

### Requirement: The daemon refuses posts from other sites
A `POST` with an `Origin` header SHALL get status 403 if the host and port in
the `Origin` are not the same as the `Host` header. A `POST` with no `Origin`
header SHALL work. The daemon SHALL NOT send CORS headers.

#### Scenario: Post from the page
- **WHEN** a client posts `msg=hi` with `Host: clock.local` and `Origin: http://clock.local`
- **THEN** the status is 200

#### Scenario: Post from another site
- **WHEN** a client posts `msg=hi` with `Host: clock.local` and `Origin: https://example.com`
- **THEN** the status is 403, and the loop gets no command

#### Scenario: No CORS headers
- **WHEN** a client sends `GET /` or `OPTIONS /` with an `Origin` header
- **THEN** no response has a header that starts with `Access-Control-`

### Requirement: The web listener does not change the render loop (on-device)
With the listener on and no requests, the main thread SHALL use less than 1%
CPU. It SHALL have 10 or fewer voluntary context switches in 60 s. The web thread
SHALL use less than 1% CPU. A request from the LAN SHALL get its response in
less than 500 ms.

#### Scenario: Idle clock with the listener (on-device)
- **WHEN** the service shows the clock, the listener is on port 80, and no client connects for 60 s
- **THEN** `scripts/measure.sh` reports the main thread and the web thread below 1% CPU, each with 10 or fewer voluntary context switches

#### Scenario: Page from a phone (on-device)
- **WHEN** a person opens `http://<the Pi>/` on a phone on the LAN and sends a message and the panic alert
- **THEN** the panel shows the message and then the panic alert

#### Scenario: Response time on the LAN (on-device)
- **WHEN** a laptop on the LAN sends `GET /` and `POST /message` 5 times each
- **THEN** the median time of each is less than 500 ms
