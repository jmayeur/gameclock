# terminal-simulator Specification

## Purpose

Shows frames in a terminal on a development machine, so we can see scenes
without the Pi. It is a `Panel` that only blits pixels, like the LED panel.

## Requirements

### Requirement: Each cell shows two vertical pixels
The terminal panel SHALL draw one character row for each two pixel rows. Each
cell SHALL be the upper half block `▀`. Its 24-bit foreground color is the
upper pixel, and its 24-bit background color is the lower pixel.

#### Scenario: Cell colors come from a pixel pair
- **WHEN** a 2×2 frame with pixels (0,0) red, (0,1) blue, (1,0) green, and (1,1) white is presented at brightness 100
- **THEN** the output has one row of two `▀` cells
- **AND** the first cell has foreground (255,0,0) and background (0,0,255)
- **AND** the second cell has foreground (0,255,0) and background (255,255,255)

#### Scenario: A 64×32 frame uses 16 rows of 64 cells
- **WHEN** a 64×32 frame is presented
- **THEN** the output has 16 rows, and each row has 64 `▀` cells

#### Scenario: An odd last row has a black lower half
- **WHEN** a 1×3 frame with all pixels white is presented at brightness 100
- **THEN** the output has two rows
- **AND** the cell in the second row has foreground (255,255,255) and background (0,0,0)

### Requirement: Brightness scales the colors
The terminal panel SHALL scale each color channel by the brightness percentage,
rounded down. Brightness 0 SHALL make each cell black.

#### Scenario: Half brightness halves each channel
- **WHEN** a 1×2 frame with pixels (200,100,51) and (0,0,0) is presented at brightness 50
- **THEN** the cell has foreground (100,50,25) and background (0,0,0)

#### Scenario: Zero brightness is black
- **WHEN** a frame with non-black pixels is presented at brightness 0
- **THEN** each cell has foreground (0,0,0) and background (0,0,0)

### Requirement: Frames draw at the same place
The terminal panel SHALL clear the screen and hide the cursor once, before its
first frame. Each present SHALL move the cursor to the top-left before it
draws. So a new frame replaces the old frame.

#### Scenario: Setup happens once
- **WHEN** a terminal panel starts and presents two frames
- **THEN** the output has exactly one clear-screen code and one hide-cursor code, both before the first frame
- **AND** each frame starts with the cursor-home code

### Requirement: The terminal is restored on drop
When the terminal panel drops, it SHALL reset the colors, show the cursor
again, and move the cursor below the last frame. The shell prompt then appears
in normal colors.

#### Scenario: Drop restores the terminal
- **WHEN** a terminal panel presents a 64×32 frame and then drops
- **THEN** the output after the frame has the reset code, the show-cursor code, and a move to line 17

### Requirement: Write errors are panel errors
If the terminal panel cannot write to its output, present SHALL return an error
that contains the write error.

#### Scenario: A closed output fails the present
- **WHEN** the terminal panel writes to an output that always fails
- **THEN** present returns an error that contains the output's error message

### Requirement: The simulator looks right in a real terminal
The simulator build SHALL show the test pattern correctly in a terminal with
24-bit color. This check is by eye on the dev machine.

#### Scenario: Test pattern in a terminal (manual)
- **WHEN** `cargo sim` runs in a 24-bit color terminal of at least 64 columns and 17 lines
- **THEN** the terminal shows red, green, blue, and yellow corners, a white border, the color bars, and the gray ramp
- **AND** Ctrl-C stops the daemon and the terminal returns to normal
