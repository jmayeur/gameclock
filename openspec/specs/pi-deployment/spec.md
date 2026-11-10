# pi-deployment Specification

## Purpose

Covers how `gameclockd` is built for, installed on, run on, and measured on the
Raspberry Pi Zero W, so a release is reproducible from a development machine or
CI and its performance claims rest on device numbers.

## Requirements

### Requirement: Release binary runs on the Pi as-is
The release build for the Pi SHALL produce a single ARMv6 hard-float Linux
executable that includes the LED driver, requires no glibc symbol newer than
2.28, and has no runtime dependency on a C++ standard library, so it runs on
Raspbian Buster without installing anything.

#### Scenario: Binary targets ARMv6 and glibc 2.28
- **WHEN** the release build for the Pi completes
- **THEN** the executable is a 32-bit ARM ELF whose CPU architecture attribute is ARMv6 with hard-float ABI
- **AND** no versioned glibc symbol it references is newer than `GLIBC_2.28`
- **AND** its dynamic dependencies do not include `libstdc++` or `libc++`

#### Scenario: CI produces the Pi binary
- **WHEN** CI runs on a pull request
- **THEN** it builds the Pi release binary with the LED driver and fails the run if the checks in "Binary targets ARMv6 and glibc 2.28" do not hold

### Requirement: Driver source is pinned
The LED driver SHALL be built from a vendored copy pinned to one recorded
upstream commit. Building the driver without the vendored source present SHALL
fail with a message saying how to fetch it.

#### Scenario: Missing driver source fails clearly
- **WHEN** the Pi build is run in a checkout where the vendored driver directory is empty
- **THEN** the build fails with a message that tells the developer to initialize git submodules

### Requirement: Runs as a supervised service
The daemon SHALL run as a systemd service started as root (the driver drops
privileges after initializing GPIO), restarted after a failure, and stopped with
SIGTERM. Starting it SHALL stop the legacy clock service, and starting the
legacy service SHALL stop it, so the two never drive the panel at once.

#### Scenario: Service replaces the legacy clock (on-device)
- **WHEN** the legacy clock service is running and the `gameclockd` service is started
- **THEN** the legacy clock service is stopped and the panel shows the test pattern

#### Scenario: Legacy clock is a one-command rollback (on-device)
- **WHEN** the `gameclockd` service is running and the legacy clock service is started
- **THEN** the `gameclockd` service is stopped and the legacy clock is shown

#### Scenario: Crash is restarted (on-device)
- **WHEN** the `gameclockd` process is killed with SIGKILL
- **THEN** systemd starts it again within 10 seconds

### Requirement: Starting at boot is an explicit choice
Installing the service SHALL NOT enable it at boot. Enabling it at boot SHALL be
a separate, explicit deploy option that also disables the legacy clock at boot,
so exactly one of the two is enabled.

#### Scenario: Plain deploy leaves boot unchanged (on-device)
- **WHEN** the deploy script runs without the enable option
- **THEN** after a reboot the legacy clock is running and `gameclockd` is not

#### Scenario: Enable swaps the boot service (on-device)
- **WHEN** the deploy script runs with the enable option and the Pi reboots
- **THEN** `gameclockd` is running and the legacy clock service is disabled

### Requirement: Deploy keeps device details out of the repo
Deploying SHALL copy the release binary and service unit to the Pi and restart
the service, taking the target host only from the environment. Without a
target host the deploy SHALL refuse to run and SHALL NOT attempt any network
connection.

#### Scenario: Deploy without a host refuses
- **WHEN** the deploy script runs with no target host set in the environment
- **THEN** it exits non-zero with a message naming the environment variable to set
- **AND** it makes no network connection

#### Scenario: Deploy installs and restarts (on-device)
- **WHEN** the deploy script runs with the target host set and a fresh release build
- **THEN** the Pi runs the new binary under the `gameclockd` service and the panel shows the test pattern

### Requirement: Performance is measured the same way for every process
A measurement tool SHALL report, for a given process over a given window, its
CPU usage as a percentage of one core, in total and for each of its threads,
and its resident memory, using only what the Linux kernel exposes, so the
legacy clock and `gameclockd` are measured identically on the Pi and the
driver's refresh thread is reported separately from application work.

#### Scenario: Idle process measures near zero
- **WHEN** the measurement tool samples a process that is sleeping for the whole of a 5 s window on Linux
- **THEN** it reports CPU usage below 1% and a non-zero resident memory in KiB

#### Scenario: Busy process measures near one core
- **WHEN** the measurement tool samples a process that spins on the CPU for the whole of a 5 s window on an otherwise idle Linux machine
- **THEN** it reports CPU usage of at least 80%

#### Scenario: Threads are reported separately
- **WHEN** the measurement tool samples a process with one spinning thread and one sleeping thread over a 5 s window on Linux
- **THEN** it lists both threads with their names, the spinning thread at 80% or more and the sleeping thread below 1%

### Requirement: Baselines are recorded with the change
Before this change is archived, measurements of the legacy clock and of
`gameclockd` on the Pi Zero W SHALL be recorded with the change: process and
per-thread CPU %, resident memory, driver refresh rate, the driver flags used,
and the measurement window.

#### Scenario: Baseline table is complete (on-device)
- **WHEN** the change is reviewed for archive
- **THEN** its measurements record contains CPU %, RSS, and refresh Hz for both the legacy clock and `gameclockd`, each with the driver flags and window used
