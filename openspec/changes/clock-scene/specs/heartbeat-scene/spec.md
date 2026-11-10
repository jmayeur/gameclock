## ADDED Requirements

### Requirement: The heartbeat is a diagnostic mode at full brightness
The heartbeat scene SHALL be the base scene only when the daemon starts with
`--test-pattern`. It SHALL always give full brightness, so that the test
pattern shows each color level of the panel.

#### Scenario: Full brightness at each time
- **WHEN** the heartbeat scene's brightness is asked at 1970-01-01T00:00:00Z and at 2026-11-10T23:05:00Z
- **THEN** each brightness is 100
