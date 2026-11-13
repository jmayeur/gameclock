## ADDED Requirements

### Requirement: The base scene can be replaced
The compositor SHALL replace its base scene when it is told to. The
foreground scenes SHALL stay. The next update after a replacement SHALL give
a frame, even if the image and the brightness are the same as before.

#### Scenario: New base scene shows at the next update
- **WHEN** a compositor with a red base scene is updated, its base scene is replaced with a blue scene, and it is updated again
- **THEN** the second update gives an all-blue frame

#### Scenario: Foreground scenes stay on top
- **WHEN** a red base scene has a green foreground scene, and the base scene is replaced with a blue scene
- **THEN** the next update gives an all-green frame

#### Scenario: Same image still gives a frame
- **WHEN** a compositor with a red base scene is updated, its base scene is replaced with another red scene, and it is updated again
- **THEN** the second update gives a frame

### Requirement: The compositor tells the last brightness
The compositor SHALL give the brightness of the last frame that an update
gave. Before the first update, it SHALL give full brightness.

#### Scenario: Last brightness after an update
- **WHEN** a compositor with a base scene at brightness 25 is updated
- **THEN** the compositor gives 25 as the last brightness

#### Scenario: Last brightness before an update
- **WHEN** a new compositor is not updated yet
- **THEN** the compositor gives 100 as the last brightness
