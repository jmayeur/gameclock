# scene-model Specification

## Purpose

Defines what a scene must tell the system. It also defines how the compositor
selects the visible scene, removes finished scenes, and decides when to render
and present. All later scenes (clock, messages, alerts) use this contract.

## Requirements

### Requirement: The top scene is visible
The compositor SHALL hold one base scene and a stack of foreground scenes. It
SHALL render only the top scene: the newest foreground scene, or the base
scene when the stack is empty.

#### Scenario: Base scene with an empty stack
- **WHEN** a compositor with a base scene that fills the frame red and no foreground scenes is updated
- **THEN** the result frame is all red

#### Scenario: Newest foreground scene is on top
- **WHEN** a red base scene has a green foreground scene and then a blue foreground scene added, and the compositor is updated
- **THEN** the result frame is all blue

### Requirement: Each render starts from a black frame
Before the compositor renders a scene, it SHALL make the whole frame black.
A scene then draws only what it shows, and no pixels remain from an earlier
scene.

#### Scenario: No pixels remain from the scene below
- **WHEN** a red base scene is presented, and then a foreground scene that draws only a white pixel at (0, 0) is added and updated
- **THEN** pixel (0, 0) is white and all other pixels are black

### Requirement: Finished foreground scenes go away
At each update, the compositor SHALL remove each foreground scene that reports
that it is finished, before it renders. The scene below then shows in the same
update.

#### Scenario: Base scene returns after the foreground scene finishes
- **WHEN** a red base scene has a green foreground scene that is finished from 12:00:05 UTC, and the compositor is updated at 12:00:05 UTC
- **THEN** the result frame is all red
- **AND** the stack is empty

#### Scenario: Unfinished foreground scene stays
- **WHEN** the same compositor is updated at 12:00:04 UTC
- **THEN** the result frame is all green

### Requirement: The base scene always stays
The compositor SHALL NOT remove the base scene, even when the base scene
reports that it is finished.

#### Scenario: Finished base scene is still shown
- **WHEN** a red base scene reports that it is finished, and the compositor is updated
- **THEN** the result frame is all red

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

### Requirement: The deadline is the top scene's next change
After an update, the compositor SHALL give the top scene's next change time as
its deadline. A scene with no next change SHALL give no deadline. The daemon
then waits with no timeout.

#### Scenario: Static scene has no deadline
- **WHEN** a compositor with a base scene that has no next change is updated at 12:00:00 UTC
- **THEN** the compositor gives no deadline

#### Scenario: Deadline follows the top scene
- **WHEN** a base scene changes next at 12:01:00 UTC, a foreground scene changes next at 12:00:10 UTC, and the compositor is updated at 12:00:00 UTC
- **THEN** the deadline is 12:00:10 UTC

### Requirement: A scene's finish time is a change
A foreground scene that finishes at a time SHALL report that time, or an
earlier time, as its next change. The loop then wakes to remove the scene.

#### Scenario: Deadline is the finish time
- **WHEN** a foreground scene finishes at 12:00:05 UTC and reports that time as its next change
- **AND** the compositor is updated at 12:00:00 UTC
- **THEN** the deadline is 12:00:05 UTC

### Requirement: Renders are at most 30 per second
The compositor SHALL give a deadline that is 33 ms or more after the update
time. A scene that reports an earlier next change, or a time in the past, then
renders at most 30 times each second.

#### Scenario: Too-early change is delayed
- **WHEN** the top scene reports a next change 1 ms after the update time of 12:00:00.000 UTC
- **THEN** the deadline is 12:00:00.033 UTC

#### Scenario: Past change is delayed
- **WHEN** the top scene reports a next change at 11:59:59 UTC, and the update time is 12:00:00.000 UTC
- **THEN** the deadline is 12:00:00.033 UTC

### Requirement: Updates do not allocate heap memory
After the first update, an update SHALL NOT allocate heap memory while the
scene stack does not change.

#### Scenario: One thousand updates allocate nothing
- **WHEN** a compositor with the heartbeat scene is updated once, and then 1000 more times with the time 1 s later each time
- **THEN** the 1000 updates make zero heap allocations

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
