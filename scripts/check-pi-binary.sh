#!/bin/sh
# Checks that a binary will run on the Pi Zero W as-is (specs/pi-deployment,
# "Binary targets ARMv6 and glibc 2.28"; docs/adr/0007):
#   - 32-bit ARM ELF, hard-float ABI
#   - CPU architecture attribute ARMv6 (v6, v6K, or v6KZ), not v7+
#   - no versioned glibc symbol newer than GLIBC_2.28
#   - no dynamic dependency on a C++ standard library
#
# Usage: scripts/check-pi-binary.sh <binary>
# Uses $READELF, else readelf on PATH, else Homebrew's binutils readelf.
set -eu

MAX_GLIBC=2.28

bin=${1:?usage: check-pi-binary.sh <binary>}
[ -f "$bin" ] || { echo "error: no such file: $bin" >&2; exit 2; }

if [ -z "${READELF:-}" ]; then
    if command -v readelf >/dev/null 2>&1; then
        READELF=readelf
    elif [ -x /opt/homebrew/opt/binutils/bin/readelf ]; then
        READELF=/opt/homebrew/opt/binutils/bin/readelf
    else
        echo "error: readelf not found (install binutils or set READELF)" >&2
        exit 2
    fi
fi

problems=0
fail() { echo "FAIL: $*"; problems=$((problems + 1)); }

header=$("$READELF" -h "$bin" 2>/dev/null) || header=""
if ! echo "$header" | grep -q "Class:.*ELF32" || ! echo "$header" | grep -q "Machine:.*ARM$"; then
    echo "FAIL: $bin is not a 32-bit ARM ELF"
    exit 1
fi

echo "$header" | grep -q "Flags:.*hard-float ABI" || fail "not hard-float ABI"

arch=$("$READELF" -A "$bin" | sed -n 's/^ *Tag_CPU_arch: *//p' | head -n 1)
case "$arch" in
    v6 | v6K | v6KZ) ;;
    *) fail "not ARMv6: Tag_CPU_arch is '${arch:-missing}'" ;;
esac

newest=$("$READELF" -V "$bin" | grep -o 'GLIBC_[0-9][0-9.]*' | sed 's/GLIBC_//' | sort -u -t. -k1,1n -k2,2n -k3,3n | tail -n 1)
if [ -n "$newest" ]; then
    highest=$(printf '%s\n%s\n' "$newest" "$MAX_GLIBC" | sort -t. -k1,1n -k2,2n -k3,3n | tail -n 1)
    [ "$highest" = "$MAX_GLIBC" ] || fail "needs GLIBC_$newest, newer than GLIBC_$MAX_GLIBC"
fi

needed=$("$READELF" -d "$bin" | sed -n 's/.*(NEEDED).*\[\(.*\)\]/\1/p')
for lib in $needed; do
    case "$lib" in
        libstdc++* | libc++*) fail "depends on $lib" ;;
    esac
done

if [ "$problems" -gt 0 ]; then
    exit 1
fi
echo "ok: $bin is ARMv6 ($arch) hard-float, glibc <= $MAX_GLIBC (newest GLIBC_${newest:-none}), needs: $(echo $needed)"
