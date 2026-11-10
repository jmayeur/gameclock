#!/bin/sh
# Tests scripts/lint-docs.sh with one compliant and one non-compliant file.
# Usage: scripts/test-lint-docs.sh
set -eu

here=$(cd "$(dirname "$0")" && pwd)
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

printf '# Good\n\nThe daemon shows the test pattern. It waits for a signal.\n' >"$work/good.md"
printf '# Bad\n\nThe daemon shows the pattern; then it waits.\n' >"$work/bad.md"

failures=0
if ! "$here/lint-docs.sh" "$work/good.md" >/dev/null 2>&1; then
    echo "FAIL: a compliant file failed the gate"; failures=$((failures + 1))
fi
if "$here/lint-docs.sh" "$work/bad.md" >/dev/null 2>&1; then
    echo "FAIL: a file with a semicolon passed the gate"; failures=$((failures + 1))
fi
if ! "$here/lint-docs.sh" --list | grep -q '^CLAUDE.md$'; then
    echo "FAIL: the default file set does not include CLAUDE.md"; failures=$((failures + 1))
fi
if "$here/lint-docs.sh" --list | grep -qE '^(vendor|\.claude|openspec/changes/archive)/'; then
    echo "FAIL: the default file set includes vendored, skill, or archived files"; failures=$((failures + 1))
fi

[ "$failures" -eq 0 ] || { echo "$failures failure(s)"; exit 1; }
echo "all lint-docs tests passed"
