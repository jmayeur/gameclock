#!/bin/sh
# Lints the feature-gated driver code for the Pi target (docs/adr/0009).
# Host clippy never compiles it. `cargo-zigbuild clippy` sets zig up as the
# target C/C++ compiler for build.rs.
#
# Usage: scripts/clippy-pi.sh
set -eu

cd "$(dirname "$0")/.."
exec cargo-zigbuild clippy --locked --target arm-unknown-linux-gnueabihf.2.28 --all-targets \
    -p rgbmatrix-sys --features rgbmatrix-sys/link \
    -p gameclockd --features gameclockd/hzeller \
    -- -D warnings
