# hzeller-panel Specification

## Purpose

Displays finished frames on the physical LED matrix through hzeller's
rpi-rgb-led-matrix driver, without any display logic of its own, and exposes
the driver's tuning options so the panel can be configured and measured.

## Requirements

### Requirement: Present shows exactly the frame's pixels
Presenting a frame SHALL make the panel show that frame's pixels, one for one,
in a single tear-free update. Pixel (x, y) of the frame SHALL appear at column
x, row y of the panel, with (0, 0) at the top-left.

#### Scenario: Frame pixels are converted in row-major order
- **WHEN** a 64×32 frame whose pixel at (x, y) has color (x·4, y·8, 7) is prepared for the driver
- **THEN** the driver buffer has 2048 entries
- **AND** entry y·64 + x holds red x·4, green y·8, blue 7

#### Scenario: Test pattern on the real panel (on-device)
- **WHEN** `gameclockd` built with the hzeller panel runs on the Pi Zero W with default flags
- **THEN** the panel shows the test pattern with red at the top-left, green at the top-right, blue at the bottom-left, and yellow at the bottom-right, a complete white border, and steady color bars and gray ramp without visible tearing

### Requirement: Frames must match the panel size
Presenting a frame whose dimensions differ from the panel's SHALL fail with an
error that names both sizes, and SHALL leave the panel's current image
unchanged.

#### Scenario: Wrong-sized frame is rejected
- **WHEN** a 32×16 frame is presented to a 64×32 panel
- **THEN** present returns a dimension-mismatch error reporting 32×16 and 64×32
- **AND** nothing is written to the driver

### Requirement: Brightness is applied per present
Presenting SHALL apply the given brightness percentage to the displayed frame.
A brightness of 0 SHALL show an all-black panel, since the driver's own
brightness range starts at 1.

#### Scenario: Brightness maps to the driver range
- **WHEN** a frame is prepared for brightness 1, 40, and 100
- **THEN** the driver brightness is 1, 40, and 100 respectively

#### Scenario: Zero brightness blanks the panel
- **WHEN** a frame containing non-black pixels is prepared for brightness 0
- **THEN** every entry in the driver buffer is black

### Requirement: Driver flags pass through over Pi defaults
The panel SHALL accept hzeller's standard `--led-*` command-line flags and SHALL
apply them over built-in defaults for this hardware: 64 columns, 32 rows, one
chain, one parallel, `adafruit-hat` GPIO mapping. A flag on the command line
SHALL win over the built-in default.

#### Scenario: Built-in defaults describe this Pi
- **WHEN** the panel's default driver options are inspected, before any flags are applied
- **THEN** they specify 64 columns, 32 rows, chain length 1, parallel 1, and hardware mapping `adafruit-hat`

#### Scenario: Driver flags reach the driver (on-device)
- **WHEN** `gameclockd` is started with `--led-show-refresh`
- **THEN** the driver reports its refresh rate on standard error while the test pattern is shown

#### Scenario: A flag overrides a default (on-device)
- **WHEN** `gameclockd` is started with `--led-gpio-mapping=regular` on the Adafruit HAT
- **THEN** the panel no longer shows the test pattern correctly, showing the flag replaced the `adafruit-hat` default

#### Scenario: Invalid flags fail at startup (on-device)
- **WHEN** `gameclockd` is started with `--led-rows=abc`
- **THEN** it exits with a non-zero status and an error message, without driving the panel

### Requirement: Driver initialization failure is an error
If the driver cannot create the matrix, creating the panel SHALL return an
error that mentions panel initialization. The daemon SHALL NOT panic or
continue without a panel.

#### Scenario: Rejected driver options fail cleanly (on-device)
- **WHEN** `gameclockd` built with the hzeller panel is started with `--led-rows=abc`
- **THEN** it exits with a non-zero status and an error that mentions panel initialization

### Requirement: A non-root start runs degraded with a warning
The service starts as root. A manual start as a non-root user SHALL show the
driver's warning that timing is degraded, and SHALL keep running. The driver
falls back to user-space GPIO access in that case.

#### Scenario: Unprivileged start warns and keeps running (on-device)
- **WHEN** `gameclockd` built with the hzeller panel is started as a non-root user on the Pi
- **THEN** standard error contains the driver's warning that it is not running as root
- **AND** the process keeps running until it is stopped

### Requirement: Shutdown blanks the panel and releases the driver
Dropping the panel SHALL clear the display and stop the driver's refresh thread
before the process exits, so no stale image or half-lit rows remain.

#### Scenario: Panel is dark after stop (on-device)
- **WHEN** the running daemon receives SIGTERM
- **THEN** the panel goes dark and the driver's refresh thread is no longer running when the process exits
