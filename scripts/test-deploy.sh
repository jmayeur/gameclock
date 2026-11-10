#!/bin/sh
# Tests scripts/deploy.sh with stub ssh/scp on PATH (specs/pi-deployment,
# "Deploy keeps device details out of the repo"; docs/adr/0010). No network.
# Usage: scripts/test-deploy.sh
set -eu

here=$(cd "$(dirname "$0")" && pwd)
deploy="$here/deploy.sh"
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

mkdir "$work/bin"
log="$work/calls.log"
# Stubs record each call. ssh prints a temp dir when asked for one.
for tool in ssh scp; do
    cat >"$work/bin/$tool" <<STUB
#!/bin/sh
echo "$tool \$*" >>"$log"
case "\$*" in *mktemp*) echo /tmp/gameclockd-deploy.XYZ ;; esac
exit \${STUB_EXIT:-0}
STUB
    chmod +x "$work/bin/$tool"
done
: >"$work/gameclockd"   # stand-in for the release binary

failures=0
fail() { echo "FAIL: $*"; failures=$((failures + 1)); }

# run <expected-exit> [args...]: run deploy.sh with stubs; output in $out.
run() {
    want=$1; shift
    : >"$log"
    set +e
    out=$(PATH="$work/bin:$PATH" GAMECLOCKD_BIN="$work/gameclockd" "$deploy" "$@" 2>&1)
    got=$?
    set -e
    if [ "$want" = nonzero ]; then
        [ "$got" -ne 0 ] || fail "deploy.sh $*: expected failure, got success"
    else
        [ "$got" -eq "$want" ] || fail "deploy.sh $*: expected exit $want, got $got: $out"
    fi
}
called() { grep -qF -- "$1" "$log" || fail "expected a call containing '$1'; calls: $(cat "$log")"; }
not_called() { if grep -qF -- "$1" "$log"; then fail "unexpected call containing '$1'"; fi; }

# Scenario: Deploy without a host refuses.
unset GAMECLOCK_HOST
run nonzero
echo "$out" | grep -q GAMECLOCK_HOST || fail "no-host message doesn't name GAMECLOCK_HOST: $out"
[ ! -s "$log" ] || fail "made network calls without a host: $(cat "$log")"

export GAMECLOCK_HOST=pi@example.invalid

# Default: copy binary and unit, install, restart; boot settings untouched.
run 0
called "scp"
called "gameclockd.service"
called "systemctl daemon-reload"
called "systemctl restart gameclockd"
not_called "enable"
not_called "disable"

# --enable: also swap the boot service.
run 0 --enable
called "systemctl restart gameclockd"
called "systemctl disable clock.svc.service"
called "systemctl enable gameclockd"

# --rollback: no copy; legacy clock back as the running and boot service.
run 0 --rollback
not_called "scp"
called "systemctl disable --now gameclockd"
called "systemctl enable --now clock.svc.service"

# Unknown option.
run nonzero --frobnicate
[ ! -s "$log" ] || fail "made network calls for a bad option"

# A failing remote step fails the deploy.
STUB_EXIT=1 run nonzero
export STUB_EXIT=0

[ "$failures" -eq 0 ] || { echo "$failures failure(s)"; exit 1; }
echo "all deploy tests passed"
