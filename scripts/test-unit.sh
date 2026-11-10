#!/bin/sh
# Tests deploy/gameclockd.service (openspec/specs/pi-deployment, "Runs as a supervised
# service", docs/adr/0010). Linux only: needs systemd-analyze.
# Usage: scripts/test-unit.sh
set -eu

cd "$(dirname "$0")/.."
unit=deploy/gameclockd.service
failures=0
fail() { echo "FAIL: $*"; failures=$((failures + 1)); }
has() { grep -qx "$1" "$unit" || fail "$unit lacks the line '$1'"; }

if [ ! -f "$unit" ]; then
    echo "FAIL: $unit does not exist"
    exit 1
fi

has "Conflicts=clock.svc.service"
has "Restart=on-failure"
has "EnvironmentFile=-/etc/default/gameclockd"
# 8 PWM bits first. A later flag from GAMECLOCKD_ARGS wins (ADR-0015).
has "ExecStart=/usr/local/bin/gameclockd --led-pwm-bits=8 \$GAMECLOCKD_ARGS"
has "WantedBy=multi-user.target"
grep -q "^User=" "$unit" && fail "$unit sets User=; the driver needs root to start (ADR-0010)"

# systemd-analyze verify needs ExecStart to exist. So it reads a copy that
# points at /bin/true. The ExecStart line test above checks the real path.
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
sed 's|^ExecStart=/usr/local/bin/gameclockd|ExecStart=/bin/true|' "$unit" >"$work/gameclockd.service"
if ! out=$(systemd-analyze verify "$work/gameclockd.service" 2>&1); then
    fail "systemd-analyze verify failed"
fi
# verify exits 0 on some warnings. Any output about our unit is a failure.
if echo "$out" | grep -q "gameclockd.service"; then
    fail "systemd-analyze reported problems:"
    echo "$out" | sed 's/^/    /'
fi

[ "$failures" -eq 0 ] || { echo "$failures failure(s)"; exit 1; }
echo "ok: $unit"
