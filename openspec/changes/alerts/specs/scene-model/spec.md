## MODIFIED Requirements

### Requirement: The deadline is the top scene's next change
After an update, the compositor SHALL give the top scene's next change time as
its deadline. A scene with no next change SHALL give no deadline. The daemon
then waits until the next trigger time (`alerts`).

#### Scenario: Static scene has no deadline
- **WHEN** a compositor with a base scene that has no next change is updated at 12:00:00 UTC
- **THEN** the compositor gives no deadline

#### Scenario: Deadline follows the top scene
- **WHEN** a base scene changes next at 12:01:00 UTC, a foreground scene changes next at 12:00:10 UTC, and the compositor is updated at 12:00:00 UTC
- **THEN** the deadline is 12:00:10 UTC
