# hzeller-panel Specification

## Purpose

Shows finished frames on the physical LED matrix through hzeller's
rpi-rgb-led-matrix driver. It has no rendering logic of its own. It gives
access to the driver's tuning options, so we can configure and measure the
panel.

## Requirements

### Requirement: Present shows exactly the frame's pixels
Presenting a frame SHALL make the panel show the frame's pixels, one for one,
in one tear-free update. Pixel (x, y) of the frame SHALL appear at column x,
row y of the panel. (0, 0) is the top-left.

#### Scenario: Frame pixels are converted in row-major order
- **WHEN** a 64×32 frame whose pixel at (x, y) has color (x·4, y·8, 7) is prepared for the driver
- **THEN** the driver buffer has 2048 entries
- **AND** entry y·64 + x holds red x·4, green y·8, blue 7

#### Scenario: Test pattern on the real panel (on-device)
- **WHEN** `gameclockd` built with the hzeller panel runs on the Pi Zero W with default flags
- **THEN** the panel shows red at the top-left, green at the top-right, blue at the bottom-left, and yellow at the bottom-right
- **AND** the white border is complete
- **AND** the color bars and the gray ramp are steady, with no visible tearing

### Requirement: Frames must match the panel size
Presenting a frame whose size is different from the panel's SHALL fail with an
error that gives both sizes. The panel's current image SHALL stay the same.

#### Scenario: Wrong-sized frame is rejected
- **WHEN** a 32×16 frame is presented to a 64×32 panel
- **THEN** present returns a dimension-mismatch error that gives 32×16 and 64×32
- **AND** nothing is written to the driver

### Requirement: Brightness is applied per present
Presenting SHALL apply the given brightness percentage to the frame. A
brightness of 0 SHALL make the panel all black, because the driver's own
brightness range starts at 1.

#### Scenario: Brightness maps to the driver range
- **WHEN** a frame is prepared for brightness 1, 40, and 100
- **THEN** the driver brightness is 1, 40, and 100

#### Scenario: Zero brightness blanks the panel
- **WHEN** a frame that has non-black pixels is prepared for brightness 0
- **THEN** each entry in the driver buffer is black

### Requirement: Driver flags pass through over Pi defaults
The panel SHALL accept hzeller's standard `--led-*` command-line flags. It
SHALL apply them over built-in defaults for this hardware: 64 columns, 32
rows, one chain, one parallel, and the `adafruit-hat` GPIO mapping. A flag on
the command line SHALL win over a built-in default.

#### Scenario: Built-in defaults describe this Pi
- **WHEN** the panel's default driver options are read, before any flags apply
- **THEN** they give 64 columns, 32 rows, chain length 1, parallel 1, and hardware mapping `adafruit-hat`

#### Scenario: Driver flags reach the driver (on-device)
- **WHEN** `gameclockd` is started with `--led-show-refresh`
- **THEN** the driver writes its refresh rate to standard error while the panel shows the test pattern

#### Scenario: A flag overrides a default (on-device)
- **WHEN** `gameclockd` is started with `--led-gpio-mapping=regular` on the Adafruit HAT
- **THEN** the panel no longer shows the test pattern correctly
- **AND** this shows that the flag replaced the `adafruit-hat` default

#### Scenario: Invalid flags fail at startup (on-device)
- **WHEN** `gameclockd` is started with `--led-rows=abc`
- **THEN** it exits with a non-zero status and an error message, and does not drive the panel

### Requirement: Driver initialization failure is an error
If the driver cannot create the matrix, creating the panel SHALL return an
error that names panel initialization. The daemon SHALL NOT panic or continue
without a panel.

#### Scenario: Rejected driver options fail cleanly (on-device)
- **WHEN** `gameclockd` built with the hzeller panel is started with `--led-rows=abc`
- **THEN** it exits with a non-zero status and an error that names panel initialization

### Requirement: A non-root start runs degraded with a warning
The service starts as root. A manual start as a non-root user SHALL show the
driver's warning that timing is degraded. The process SHALL keep running. In
this case, the driver uses user-space GPIO access.

#### Scenario: Unprivileged start warns and keeps running (on-device)
- **WHEN** `gameclockd` built with the hzeller panel is started as a non-root user on the Pi
- **THEN** standard error contains the driver's warning that it is not running as root
- **AND** the process keeps running until it is stopped

### Requirement: Shutdown blanks the panel and releases the driver
When the panel drops, it SHALL clear the LEDs and stop the driver's refresh
thread before the process exits. No old image or half-lit rows remain.

#### Scenario: Panel is dark after stop (on-device)
- **WHEN** the running daemon receives SIGTERM
- **THEN** the panel goes dark
- **AND** the driver's refresh thread no longer runs when the process exits
