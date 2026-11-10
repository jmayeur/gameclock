#!/bin/sh
# Tests that gameclockd does not build with both the hzeller and simulator
# features (openspec/specs/daemon-lifecycle, scenario "Driver and simulator
# together do not build").
#
# The check is for the Pi target, through cargo-zigbuild. A host check fails
# earlier, in the C++ build, and never reaches the compile_error!. So this
# test needs zig and cargo-zigbuild, like scripts/build-pi.sh.
# Usage: scripts/test-feature-exclusive.sh
set -eu

cd "$(dirname "$0")/.."
expected="the hzeller and simulator features cannot be used together"

if out=$(cargo-zigbuild check --locked --target arm-unknown-linux-gnueabihf.2.28 \
    -p gameclockd --features hzeller,simulator 2>&1); then
    echo "FAIL: the build passed with both features"
    exit 1
fi
if ! printf '%s\n' "$out" | grep -qF "$expected"; then
    echo "FAIL: the build failed, but without the expected message:"
    printf '%s\n' "$out" | tail -n 20 | sed 's/^/    /'
    exit 1
fi
echo "ok: hzeller and simulator together do not build"
