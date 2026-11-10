# test-pattern Specification

## Purpose

A deterministic diagnostic image that shows, at a glance on real hardware,
whether a panel's geometry, orientation, color channel order, and color depth
are configured correctly.

## Requirements

### Requirement: Corner markers identify orientation
The test pattern SHALL fill a 3×3 block in each corner of the frame with a
distinct color: top-left red, top-right green, bottom-left blue, bottom-right
yellow. Any rotation, mirroring, or swapped color channels then produces a
visibly different image.

#### Scenario: Corners on a 64×32 frame
- **WHEN** the test pattern is rendered into a 64×32 frame
- **THEN** pixels (0..3, 0..3) are pure red (255,0,0)
- **AND** pixels (61..64, 0..3) are pure green (0,255,0)
- **AND** pixels (0..3, 29..32) are pure blue (0,0,255)
- **AND** pixels (61..64, 29..32) are pure yellow (255,255,0)

### Requirement: Border outlines the panel edge
The test pattern SHALL draw the outermost ring of pixels, outside the corner
blocks, in white, so a missing row or column at any edge is visible.

#### Scenario: Edge pixels are white
- **WHEN** the test pattern is rendered into a 64×32 frame
- **THEN** every pixel in row 0, row 31, column 0, and column 63 that lies outside a corner block is white (255,255,255)

### Requirement: Content area sits inside the markers
The test pattern's bars and ramp SHALL occupy a content area inset 3 pixels
from every edge, so they never overlap the border or the corner markers. Pixels
between the border and the content area SHALL be black.

#### Scenario: Margin is black
- **WHEN** the test pattern is rendered into a 64×32 frame
- **THEN** every pixel with x in 1..3 or 61..63, or y in 1..3 or 29..31, that is neither on the border nor in a corner block is black

### Requirement: Color bars show channel order
The upper half of the content area SHALL contain vertical bars of red, green,
blue, cyan, magenta, yellow, and white, left to right, at full intensity.

#### Scenario: Bars appear in order
- **WHEN** the test pattern is rendered into a 64×32 frame
- **THEN** sampling the middle row of the upper half of the content area from left to right yields the colors red, green, blue, cyan, magenta, yellow, white in that order with no other colors between them

### Requirement: Gray ramp shows color depth
The lower half of the content area SHALL contain a horizontal gray ramp whose brightness never decreases from left to right,
starting at or near black and ending at or near white.

#### Scenario: Ramp is monotonic
- **WHEN** the test pattern is rendered into a 64×32 frame
- **THEN** along any row of the lower half of the content area, each pixel has equal red, green, and blue values
- **AND** each pixel's value is greater than or equal to the pixel to its left
- **AND** the leftmost value is at most 16 and the rightmost value is at least 240

### Requirement: Rendering is deterministic and safe for any size
Rendering the test pattern SHALL depend only on the frame's dimensions. It SHALL
NOT fail or panic for any frame size, including frames smaller than the
pattern's features, which are clipped.

#### Scenario: Same size, same image
- **WHEN** the test pattern is rendered twice into two 64×32 frames
- **THEN** the two frames are identical

#### Scenario: Tiny frames do not panic
- **WHEN** the test pattern is rendered into frames of 0×0, 1×1, 2×5, and 5×2 pixels
- **THEN** rendering completes without panicking
