## ADDED Requirements

### Requirement: Scenes give a brightness
A scene SHALL give a brightness for each time. A scene that does not set one
SHALL give full brightness. Each update of the compositor SHALL give the top
scene's brightness for the update time.

#### Scenario: Default brightness is full
- **WHEN** a compositor with a red base scene that does not set a brightness is updated
- **THEN** the compositor gives the brightness 100

#### Scenario: Brightness of the top scene
- **WHEN** a base scene at brightness 25 has a foreground scene at brightness 80, and the compositor is updated
- **THEN** the compositor gives the brightness 80

## MODIFIED Requirements

### Requirement: Only a changed frame is presented
An update SHALL give a frame to present only when the image or the
brightness changed. The image changed when the rendered frame is different
from the last frame that the compositor gave. The same rule applies to the
brightness. The first update SHALL always give a frame.

#### Scenario: First update gives a frame
- **WHEN** a new compositor with a red base scene is updated
- **THEN** the update gives a frame to present

#### Scenario: Same image gives no frame
- **WHEN** a compositor with a static red base scene is updated two times
- **THEN** the second update gives no frame to present

#### Scenario: Changed image gives a frame
- **WHEN** a compositor with a base scene that is red before 12:00:01 UTC and blue after it is updated at 12:00:00 and at 12:00:01 UTC
- **THEN** each of the two updates gives a frame, and the second frame is all blue

#### Scenario: Changed brightness gives a frame
- **WHEN** a red base scene is at brightness 25 before 12:00:01 UTC and 60 after it
- **AND** the compositor is updated at 12:00:00 and at 12:00:01 UTC
- **THEN** each of the two updates gives a frame
- **AND** the second update gives the brightness 60
