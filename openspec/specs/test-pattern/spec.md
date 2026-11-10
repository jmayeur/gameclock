# test-pattern Specification

## Purpose

A deterministic diagnostic image. On real hardware, it shows at a glance
whether a panel's geometry, orientation, color channel order, and color depth
are correct.

## Requirements

### Requirement: Corner markers identify orientation
The test pattern SHALL fill a 3×3 block in each corner of the frame. The
blocks SHALL be top-left red, top-right green, bottom-left blue, and
bottom-right yellow. A rotation, a mirror, or swapped color channels then
gives a visibly different image.

#### Scenario: Corners on a 64×32 frame
- **WHEN** the test pattern is rendered into a 64×32 frame
- **THEN** pixels (0..3, 0..3) are pure red (255,0,0)
- **AND** pixels (61..64, 0..3) are pure green (0,255,0)
- **AND** pixels (0..3, 29..32) are pure blue (0,0,255)
- **AND** pixels (61..64, 29..32) are pure yellow (255,255,0)

### Requirement: Border outlines the panel edge
The test pattern SHALL draw the outermost ring of pixels in white, outside the
corner blocks. A missing row or column at an edge is then visible.

#### Scenario: Edge pixels are white
- **WHEN** the test pattern is rendered into a 64×32 frame
- **THEN** each pixel in row 0, row 31, column 0, and column 63 that is outside a corner block is white (255,255,255)

### Requirement: Content area sits inside the markers
The bars and the ramp SHALL fill a content area that starts 3 pixels in from
each edge. They then never overlap the border or the corner markers. Pixels
between the border and the content area SHALL be black.

#### Scenario: Margin is black
- **WHEN** the test pattern is rendered into a 64×32 frame
- **THEN** each pixel with x in 1..3 or 61..63, or y in 1..3 or 29..31, is black
- **AND** this does not apply to border pixels or corner blocks

### Requirement: Color bars show channel order
The upper half of the content area SHALL contain vertical bars at full
intensity. From left to right, the bars are red, green, blue, cyan, magenta,
yellow, and white.

#### Scenario: Bars appear in order
- **WHEN** the test pattern is rendered into a 64×32 frame
- **THEN** the colors on the middle row of the upper content half, from left to right, are red, green, blue, cyan, magenta, yellow, and white
- **AND** the row has no other colors between them

### Requirement: Gray ramp shows color depth
The lower half of the content area SHALL contain a horizontal gray ramp. Its
brightness never decreases from left to right. It starts at or near black and
ends at or near white.

#### Scenario: Ramp is monotonic
- **WHEN** the test pattern is rendered into a 64×32 frame
- **THEN** on each row of the lower half of the content area, each pixel has equal red, green, and blue values
- **AND** each pixel's value is greater than or equal to the value of the pixel to its left
- **AND** the leftmost value is at most 16 and the rightmost value is at least 240

### Requirement: Rendering is deterministic and safe for any size
The test pattern SHALL depend only on the frame's dimensions. It SHALL NOT fail
or panic for any frame size. In frames that are smaller than the pattern's
features, the features are clipped.

#### Scenario: Same size, same image
- **WHEN** the test pattern is rendered twice into two 64×32 frames
- **THEN** the two frames are identical

#### Scenario: Tiny frames do not panic
- **WHEN** the test pattern is rendered into frames of 0×0, 1×1, 2×5, and 5×2 pixels
- **THEN** rendering completes without a panic
