#!/bin/sh
# Tests scripts/deploy.sh with stub ssh/scp on PATH (openspec/specs/pi-deployment,
# "Deploy keeps device details out of the repo", docs/adr/0010). No network.
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

# run <expected-exit> [args...]: run deploy.sh with stubs. Output goes in $out.
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
# before <a> <b>: the first call with <a> comes before the first call with <b>.
before() {
    a=$(grep -nF -- "$1" "$log" | head -n 1 | cut -d: -f1)
    b=$(grep -nF -- "$2" "$log" | head -n 1 | cut -d: -f1)
    [ -n "$a" ] && [ -n "$b" ] && [ "$a" -lt "$b" ] ||
        fail "expected '$1' before '$2'; calls: $(cat "$log")"
}

# Scenario: Deploy without a host refuses.
unset GAMECLOCK_HOST
run nonzero
echo "$out" | grep -q GAMECLOCK_HOST || fail "no-host message doesn't name GAMECLOCK_HOST: $out"
[ ! -s "$log" ] || fail "made network calls without a host: $(cat "$log")"

export GAMECLOCK_HOST=pi@example.invalid

# Default: copy binary and unit, install, restart. Boot settings stay the same.
run 0
called "scp"
called "gameclockd.service"
called "systemctl daemon-reload"
called "systemctl restart gameclockd"
not_called "enable"
not_called "disable"

# The example config is copied. The remote command installs it only if no
# config exists, so an edited config stays (openspec/specs/pi-deployment).
called "deploy/config.toml"
called "[ -e /etc/gameclockd/config.toml ] || sudo install -D -m 644"

# Scenario: Deploy stops the legacy web stack. gameclockd then gets port 80
# (docs/adr/0020). Boot does not change.
called "systemctl stop nginx.service pm2-pi.service"
before "systemctl stop nginx.service pm2-pi.service" "systemctl restart gameclockd"

# --enable: also swap the boot service.
run 0 --enable
called "systemctl restart gameclockd"
called "systemctl enable gameclockd"
# Scenario: Enable disables the legacy web stack.
called "systemctl disable clock.svc.service nginx.service pm2-pi.service"
called "systemctl stop nginx.service pm2-pi.service"
before "systemctl stop nginx.service pm2-pi.service" "systemctl restart gameclockd"

# --rollback: no copy. The legacy clock runs again and is the boot service.
run 0 --rollback
not_called "scp"
called "systemctl disable --now gameclockd"
# Scenario: Rollback starts the legacy web stack, after gameclockd stops.
called "systemctl enable --now clock.svc.service nginx.service pm2-pi.service"
before "systemctl disable --now gameclockd" "systemctl enable --now clock.svc.service nginx.service pm2-pi.service"

# Unknown option.
run nonzero --frobnicate
[ ! -s "$log" ] || fail "made network calls for a bad option"

# A failing remote step fails the deploy.
STUB_EXIT=1 run nonzero
export STUB_EXIT=0

[ "$failures" -eq 0 ] || { echo "$failures failure(s)"; exit 1; }
echo "all deploy tests passed"
