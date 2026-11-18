## ADDED Requirements

### Requirement: The pages section
An optional `[pages]` section SHALL give the URL of each page, with the keys
`weather`, `indoor`, and `outdoor`. Each key is optional. A page with no key
has no URL. There SHALL be no built-in URLs. `reload` SHALL read the section
again.

#### Scenario: Pages from the file
- **WHEN** the file has `[pages]` with `outdoor = "http://outdoor.example/realtime.json"`
- **THEN** the `outdoor` page has that URL, and `weather` and `indoor` have no URL

#### Scenario: No pages section
- **WHEN** the file has no `[pages]` section
- **THEN** no page has a URL

#### Scenario: Reload changes the URLs
- **WHEN** the file gets a `weather` URL, and `reload` arrives
- **THEN** the reply is `ok`, and the `weather` page has the new URL

### Requirement: The daemon checks each page URL
Each page URL SHALL start with `http://` or `https://`. It SHALL have 256
characters or fewer and no spaces or control characters. A key that is not a
page name SHALL be an error. Each error SHALL give the line and the column.

#### Scenario: URL with a different scheme
- **WHEN** the file has `weather = "file:///etc/passwd"` in `[pages]`
- **THEN** the config has the error `the page URL must start with http:// or https://.`

#### Scenario: URL that starts with a dash
- **WHEN** the file has `indoor = "-o/tmp/x"` in `[pages]`
- **THEN** the config has the error `the page URL must start with http:// or https://.`

#### Scenario: URL with a space
- **WHEN** the file has `outdoor = "http://a b/"` in `[pages]`
- **THEN** the config has the error `the page URL has a space or a control character.`

#### Scenario: Unknown page key
- **WHEN** the file has `garage = "http://g/"` in `[pages]`
- **THEN** the config has an error that names `garage`
