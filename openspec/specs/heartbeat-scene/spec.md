# heartbeat-scene Specification

## Purpose

A temporary base scene that shows the event loop work. It is the test pattern
with one marker that moves each second. It stays until the clock scene
replaces it.

## Requirements

### Requirement: The test pattern is under the marker
The heartbeat scene SHALL render the test pattern into the full frame. It
SHALL then change exactly one pixel: the marker.

#### Scenario: Only the marker differs from the test pattern
- **WHEN** the heartbeat scene is rendered into a 64×32 frame at any time
- **THEN** exactly one pixel is different from the test pattern rendered into a 64×32 frame
- **AND** that pixel is the marker

### Requirement: The marker moves one step each second
The marker SHALL be one white pixel on row 1, in the black margin under the
top border. Its column SHALL be 3 plus the Unix second, modulo the width less
6. On a 64×32 frame, it then moves across columns 3 to 60 and starts again.

#### Scenario: Marker at the start of a cycle
- **WHEN** the heartbeat scene is rendered into a 64×32 frame at 1970-01-01T00:00:00Z
- **THEN** pixel (3, 1) is white (255,255,255)

#### Scenario: Marker at the end of a cycle
- **WHEN** the heartbeat scene is rendered into a 64×32 frame at 1970-01-01T00:00:57.999Z
- **THEN** pixel (60, 1) is white

#### Scenario: Marker starts the cycle again
- **WHEN** the heartbeat scene is rendered into a 64×32 frame at 1970-01-01T00:00:58Z
- **THEN** pixel (3, 1) is white
- **AND** pixel (60, 1) is black

### Requirement: The heartbeat changes on each whole second
The heartbeat scene SHALL report the next whole second after the given time as
its next change. It SHALL never report that it is finished.

#### Scenario: Next change inside a second
- **WHEN** the next change of the heartbeat scene is read at 12:00:00.250 UTC
- **THEN** it is 12:00:01.000 UTC

#### Scenario: Next change on a whole second
- **WHEN** the next change of the heartbeat scene is read at 12:00:00.000 UTC
- **THEN** it is 12:00:01.000 UTC

#### Scenario: Never finished
- **WHEN** the heartbeat scene is asked if it is finished at any time
- **THEN** it answers no

### Requirement: Small frames do not panic
The heartbeat scene SHALL NOT fail or panic for any frame size. A frame with a
width of 6 or less, or a height of 1 or less, SHALL get no marker.

#### Scenario: Tiny frames
- **WHEN** the heartbeat scene is rendered into frames of 0×0, 1×1, 6×32, and 64×1 pixels
- **THEN** rendering completes without a panic
- **AND** each frame is identical to the test pattern of the same size
