#!/bin/sh
# Deploys gameclockd to the Pi (docs/adr/0010, specs/pi-deployment).
#
# Usage: GAMECLOCK_HOST=<ssh destination> scripts/deploy.sh [--enable | --rollback]
#
#   (default)   build, copy the binary and unit, install, and restart the
#               service. Starting it stops the legacy clock (Conflicts=).
#               What starts at boot is unchanged.
#   --enable    also make gameclockd the boot service instead of the legacy
#               clock.
#   --rollback  stop and disable gameclockd; start and enable the legacy
#               clock. Copies nothing.
#
# GAMECLOCK_HOST is any ssh destination (user@host or an ~/.ssh/config alias);
# the repo never names a device. The remote user needs passwordless sudo.
# GAMECLOCKD_BIN overrides the binary to copy (default: a fresh
# scripts/build-pi.sh build).
set -eu

cd "$(dirname "$0")/.."

usage() {
    echo "usage: GAMECLOCK_HOST=<ssh destination> $0 [--enable | --rollback]" >&2
    exit 2
}

mode=install
case "${1:-}" in
    "") ;;
    --enable) mode=enable ;;
    --rollback) mode=rollback ;;
    *) usage ;;
esac
[ "$#" -le 1 ] || usage

if [ -z "${GAMECLOCK_HOST:-}" ]; then
    echo "error: set GAMECLOCK_HOST to the Pi's ssh destination (e.g. pi@<host>)" >&2
    exit 2
fi
host=$GAMECLOCK_HOST

if [ "$mode" = rollback ]; then
    ssh "$host" 'set -e
        sudo systemctl disable --now gameclockd
        sudo systemctl enable --now clock.svc.service'
    echo "rolled back: legacy clock running and enabled at boot on $host"
    exit 0
fi

if [ -z "${GAMECLOCKD_BIN:-}" ]; then
    scripts/build-pi.sh
    GAMECLOCKD_BIN=target/arm-unknown-linux-gnueabihf/release/gameclockd
fi
bin=$GAMECLOCKD_BIN
[ -f "$bin" ] || { echo "error: no binary at $bin" >&2; exit 1; }

remote_dir=$(ssh "$host" 'mktemp -d /tmp/gameclockd-deploy.XXXXXX')
scp -q "$bin" "$host:$remote_dir/gameclockd"
scp -q deploy/gameclockd.service "$host:$remote_dir/gameclockd.service"

enable_cmds=""
if [ "$mode" = enable ]; then
    enable_cmds="sudo systemctl disable clock.svc.service
        sudo systemctl enable gameclockd"
fi

# Expanded locally on purpose: $remote_dir came from mktemp on the Pi.
# shellcheck disable=SC2029
ssh "$host" "set -e
    sudo install -m 755 $remote_dir/gameclockd /usr/local/bin/gameclockd
    sudo install -m 644 $remote_dir/gameclockd.service /etc/systemd/system/gameclockd.service
    rm -rf $remote_dir
    sudo systemctl daemon-reload
    $enable_cmds
    sudo systemctl restart gameclockd"

echo "deployed to $host ($mode)"
