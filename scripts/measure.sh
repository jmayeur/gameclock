#!/bin/sh
# Measures one process the same way for every program (openspec/specs/pi-deployment,
# docs/adr/0006): CPU as a percentage of one core over a window, in total and
# per thread, plus resident memory and voluntary context switches. Reads only
# /proc, so it runs on the Pi as-is.
#
# Usage: scripts/measure.sh <pid> [seconds, default 60]
#
# Output (one item per line, easy to paste into measurements.md):
#   pid: <pid> (<name>)
#   window_s: <seconds>
#   cpu: <percent>%
#   rss_kib: <KiB>
#   thread <tid> <name> cpu <percent>% vol_ctxt_switches +<delta>
set -eu

pid=${1:?usage: measure.sh <pid> [seconds]}
window=${2:-60}
[ -r "/proc/$pid/stat" ] || { echo "error: no process $pid" >&2; exit 2; }
hz=$(getconf CLK_TCK)

# utime + stime, in clock ticks, from a stat file. Fields are counted after
# the last ") " because the command name may contain spaces or parentheses.
ticks() { sed 's/.*) //' "$1" | awk '{ print $12 + $13 }'; }

vol_switches() { sed -n 's/^voluntary_ctxt_switches:[[:space:]]*//p' "$1"; }

# snapshot <file>: one "tid ticks voluntary_switches name" line per thread.
snapshot() {
    : >"$1"
    for task in /proc/"$pid"/task/*; do
        tid=${task##*/}
        t=$(ticks "$task/stat" 2>/dev/null) || continue
        v=$(vol_switches "$task/status" 2>/dev/null) || continue
        name=$(tr ' ' '_' <"$task/comm" 2>/dev/null) || continue
        echo "$tid $t $v $name" >>"$1"
    done
}

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

total_start=$(ticks "/proc/$pid/stat")
snapshot "$work/start"
sleep "$window"
[ -r "/proc/$pid/stat" ] || { echo "error: process $pid exited during the window" >&2; exit 1; }
total_end=$(ticks "/proc/$pid/stat")
snapshot "$work/end"

name=$(cat "/proc/$pid/comm")
rss=$(sed -n 's/^VmRSS:[[:space:]]*\([0-9]*\).*/\1/p' "/proc/$pid/status")

echo "pid: $pid ($name)"
echo "window_s: $window"
awk -v d="$((total_end - total_start))" -v hz="$hz" -v w="$window" \
    'BEGIN { printf "cpu: %.1f%%\n", 100 * d / hz / w }'
echo "rss_kib: ${rss:-0}"
# Threads present at both ends of the window.
awk -v hz="$hz" -v w="$window" '
    NR == FNR { start_ticks[$1] = $2; start_vol[$1] = $3; next }
    ($1 in start_ticks) {
        printf "thread %s %s cpu %.1f%% vol_ctxt_switches +%d\n",
            $1, $4, 100 * ($2 - start_ticks[$1]) / hz / w, $3 - start_vol[$1]
    }' "$work/start" "$work/end"
