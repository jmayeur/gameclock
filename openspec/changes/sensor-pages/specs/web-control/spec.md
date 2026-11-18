## MODIFIED Requirements

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
