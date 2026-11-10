#!/bin/sh
# Tests scripts/measure.sh (openspec/specs/pi-deployment, "Performance is measured the
# same way for every process"). Linux only. Needs a C compiler for the
# two-thread fixture. Usage: scripts/test-measure.sh
set -eu

here=$(cd "$(dirname "$0")" && pwd)
measure="$here/measure.sh"
work=$(mktemp -d)
pids=""
cleanup() {
    for p in $pids; do kill "$p" 2>/dev/null || true; done
    rm -rf "$work"
}
trap cleanup EXIT

failures=0
fail() { echo "FAIL: $*"; failures=$((failures + 1)); }

# value <key> <output>: the number after "<key>:" on a "<key>: N" line.
value() { printf '%s\n' "$2" | sed -n "s/^$1: *\([0-9.]*\).*/\1/p" | head -n 1; }
# below <a> <b>: a < b (decimals).
below() { awk -v a="$1" -v b="$2" 'BEGIN { exit !(a < b) }'; }

# Scenario: Idle process measures near zero.
sleep 60 &
idle=$!; pids="$pids $idle"
out=$("$measure" "$idle" 5) || fail "measure.sh failed on an idle process"
cpu=$(value cpu "$out"); rss=$(value rss_kib "$out")
{ [ -n "$cpu" ] && below "$cpu" 1; } || fail "idle cpu '$cpu' not below 1%: $out"
{ [ -n "$rss" ] && [ "$rss" -gt 0 ]; } || fail "idle rss '$rss' not positive: $out"

# Scenario: Busy process measures near one core.
sh -c 'while :; do :; done' &
busy=$!; pids="$pids $busy"
out=$("$measure" "$busy" 5) || fail "measure.sh failed on a busy process"
cpu=$(value cpu "$out")
{ [ -n "$cpu" ] && ! below "$cpu" 80; } || fail "busy cpu '$cpu' below 80%: $out"
kill "$busy"

# Scenario: Threads are reported separately.
cat >"$work/threads.c" <<'C'
#define _GNU_SOURCE
#include <pthread.h>
#include <unistd.h>
static void *spin(void *arg) {
    (void)arg;
    pthread_setname_np(pthread_self(), "spinner");
    for (volatile unsigned long i = 0;; i++) {}
    return 0;
}
int main(void) {
    pthread_t t;
    pthread_setname_np(pthread_self(), "sleeper");
    pthread_create(&t, 0, spin, 0);
    for (;;) pause();
}
C
cc -O0 -pthread -o "$work/threads" "$work/threads.c"
"$work/threads" &
two=$!; pids="$pids $two"
sleep 0.5
out=$("$measure" "$two" 5) || fail "measure.sh failed on a two-thread process"
spinner=$(printf '%s\n' "$out" | awk '$1 == "thread" && $3 == "spinner" { print $5 }' | tr -d '%')
sleeper=$(printf '%s\n' "$out" | awk '$1 == "thread" && $3 == "sleeper" { print $5 }' | tr -d '%')
{ [ -n "$spinner" ] && ! below "$spinner" 80; } || fail "spinner thread '$spinner' below 80%: $out"
{ [ -n "$sleeper" ] && below "$sleeper" 1; } || fail "sleeper thread '$sleeper' not below 1%: $out"

[ "$failures" -eq 0 ] || { echo "$failures failure(s)"; exit 1; }
echo "all measure tests passed"
