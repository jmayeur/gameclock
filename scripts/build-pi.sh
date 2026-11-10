#!/bin/sh
# Builds the Pi Zero W release binary with the LED driver (docs/adr/0007,
# docs/adr/0009). The single entry point used by developers, deploy, and CI.
#
# Usage: scripts/build-pi.sh [extra cargo args]
# Output: target/arm-unknown-linux-gnueabihf/release/gameclockd
set -eu

cd "$(dirname "$0")/.."
exec cargo zigbuild --locked --release -p gameclockd --features hzeller \
    --target arm-unknown-linux-gnueabihf.2.28 "$@"
