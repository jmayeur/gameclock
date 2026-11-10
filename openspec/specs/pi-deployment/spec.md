# pi-deployment Specification

## Purpose

Covers how we build `gameclockd` for the Raspberry Pi Zero W, install it, run
it, and measure it there. A release can then be built again from a dev machine
or CI. Performance claims rest on device numbers.

## Requirements

### Requirement: Release binary runs on the Pi as-is
The Pi release build SHALL produce one ARMv6 hard-float Linux executable that
includes the LED driver. It SHALL need no glibc symbol newer than 2.28 and no
C++ standard library at runtime. It then runs on Raspbian Buster with no
installation of other software.

#### Scenario: Binary targets ARMv6 and glibc 2.28
- **WHEN** the Pi release build completes
- **THEN** the executable is a 32-bit ARM ELF with the ARMv6 CPU attribute and the hard-float ABI
- **AND** no versioned glibc symbol that it uses is newer than `GLIBC_2.28`
- **AND** its dynamic dependencies do not include `libstdc++` or `libc++`

#### Scenario: CI produces the Pi binary
- **WHEN** CI runs on a pull request
- **THEN** it builds the Pi release binary with the LED driver
- **AND** the run fails if a check in "Binary targets ARMv6 and glibc 2.28" fails

### Requirement: Driver source is pinned
The build SHALL compile the LED driver from a vendored copy, pinned to one
recorded upstream commit. If the vendored source is missing, the driver build
SHALL fail with a message that tells how to get it.

#### Scenario: Missing driver source fails clearly
- **WHEN** the Pi build runs in a checkout where the vendored driver directory is empty
- **THEN** the build fails with a message that tells the developer to initialize git submodules

### Requirement: Runs as a supervised service
The daemon SHALL run as a systemd service that starts as root. The driver
drops privileges after it initializes GPIO. systemd SHALL restart the daemon
after a failure and stop it with SIGTERM. Starting it SHALL stop the legacy
clock service. Starting the legacy service SHALL stop it. The two never drive
the panel at the same time.

#### Scenario: Service replaces the legacy clock (on-device)
- **WHEN** the legacy clock service runs and the `gameclockd` service is started
- **THEN** the legacy clock service stops and the panel shows the test pattern

#### Scenario: Legacy clock is a one-command rollback (on-device)
- **WHEN** the `gameclockd` service runs and the legacy clock service is started
- **THEN** the `gameclockd` service stops and the panel shows the legacy clock

#### Scenario: Crash is restarted (on-device)
- **WHEN** the `gameclockd` process is killed with SIGKILL
- **THEN** systemd starts it again in 10 seconds or less

### Requirement: Starting at boot is an explicit choice
Installing the service SHALL NOT enable it at boot. A separate, explicit deploy
option SHALL enable it at boot and disable the legacy clock at boot. Exactly
one of the two is then enabled.

#### Scenario: Plain deploy leaves boot unchanged (on-device)
- **WHEN** the deploy script runs without the enable option
- **THEN** after a reboot, the legacy clock runs and `gameclockd` does not

#### Scenario: Enable swaps the boot service (on-device)
- **WHEN** the deploy script runs with the enable option and the Pi reboots
- **THEN** `gameclockd` runs and the legacy clock service is disabled

### Requirement: Deploy keeps device details out of the repo
A deploy SHALL copy the release binary and the service unit to the Pi and
restart the service. It SHALL get the target host only from the environment.
Without a target host, it SHALL refuse to run and SHALL NOT open a network
connection.

#### Scenario: Deploy without a host refuses
- **WHEN** the deploy script runs with no target host in the environment
- **THEN** it exits non-zero with a message that names the environment variable to set
- **AND** it makes no network connection

#### Scenario: Deploy installs and restarts (on-device)
- **WHEN** the deploy script runs with the target host set and a new release build
- **THEN** the Pi runs the new binary under the `gameclockd` service
- **AND** the panel shows the test pattern

### Requirement: Performance is measured the same way for every process
A measurement tool SHALL report a process's CPU use and resident memory over a
given window. It SHALL give CPU as a percentage of one core, in total and for
each thread. It SHALL use only data from the Linux kernel. The legacy clock and
`gameclockd` are then measured the same way. The report shows the driver's
refresh thread apart from application work.

#### Scenario: Idle process measures near zero
- **WHEN** the tool measures a process that sleeps for all of a 5 s window on Linux
- **THEN** it reports CPU use below 1% and a non-zero resident memory in KiB

#### Scenario: Busy process measures near one core
- **WHEN** the tool measures a process that spins on the CPU for all of a 5 s window on an idle Linux machine
- **THEN** it reports CPU use of at least 80%

#### Scenario: Threads are reported separately
- **WHEN** the tool measures a process with one spinning thread and one sleeping thread over a 5 s window on Linux
- **THEN** it lists both threads with their names
- **AND** the spinning thread is at 80% or more and the sleeping thread is below 1%

### Requirement: Baselines are recorded with the change
Before the panel bring-up change is archived, it SHALL record measurements of
the legacy clock and `gameclockd` on the Pi Zero W. The record SHALL give
process and per-thread CPU %, resident memory, and driver refresh rate. It
SHALL also give the driver flags and the measurement window.

#### Scenario: Baseline table is complete (on-device)
- **WHEN** the change is reviewed for archive
- **THEN** its measurement record has CPU %, RSS, and refresh Hz for the legacy clock and for `gameclockd`
- **AND** each value has the driver flags and the window that were used
