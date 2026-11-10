#!/bin/sh
# Tests scripts/check-pi-binary.sh against fixture binaries built with zig.
# Needs zig and readelf. Usage: scripts/test-check-pi-binary.sh
set -eu

here=$(cd "$(dirname "$0")" && pwd)
check="$here/check-pi-binary.sh"
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

cat >"$work/main.c" <<'C'
#include <stdio.h>
int main(void) { puts("hi"); return 0; }
C
: >"$work/empty.c"

pi="-target arm-linux-gnueabihf.2.28 -mcpu=arm1176jzf_s"
zig cc $pi -o "$work/good" "$work/main.c"
zig cc -target arm-linux-gnueabihf.2.28 -mcpu=cortex_a7 -o "$work/armv7" "$work/main.c"
zig cc -target arm-linux-gnueabi.2.28 -mcpu=arm1176jzf_s -o "$work/softfloat" "$work/main.c"
zig cc -target arm-linux-gnueabihf.2.34 -mcpu=arm1176jzf_s -o "$work/glibc234" "$work/main.c"
# A stand-in shared library whose soname is libstdc++. Linked by path:
# zig maps -lstdc++ to its own static libc++, which would hide the dependency.
zig cc $pi -shared -Wl,-soname,libstdc++.so.6 -o "$work/fakecxx.so" "$work/empty.c"
zig cc $pi -o "$work/cxxdep" "$work/main.c" -Wl,--no-as-needed "$work/fakecxx.so"
zig cc -o "$work/host" "$work/main.c"

failures=0
expect() { # expect pass|fail <binary> <reason-pattern>
    want=$1 bin=$2 pattern=${3:-}
    if out=$("$check" "$work/$bin" 2>&1); then got=pass; else got=fail; fi
    if [ "$got" != "$want" ]; then
        echo "FAIL: $bin: expected $want, got $got"; echo "$out" | sed 's/^/    /'
        failures=$((failures + 1)); return
    fi
    if [ -n "$pattern" ] && ! echo "$out" | grep -q "$pattern"; then
        echo "FAIL: $bin: output lacks '$pattern'"; echo "$out" | sed 's/^/    /'
        failures=$((failures + 1)); return
    fi
    echo "ok: $bin ($want)"
}

expect pass good
expect fail armv7 "not ARMv6"
expect fail softfloat "not hard-float"
expect fail glibc234 "GLIBC_2.34"
expect fail cxxdep "libstdc++"
expect fail host "not a 32-bit ARM ELF"

[ "$failures" -eq 0 ] || { echo "$failures failure(s)"; exit 1; }
echo "all check-pi-binary tests passed"
