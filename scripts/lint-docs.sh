#!/bin/sh
# Checks the project's Markdown against the structural STE rules (ADR-0011).
# Hard findings fail the run. Advisory findings (passive voice, compound
# tenses) print but do not fail it.
#
# Usage: scripts/lint-docs.sh [FILE...]   lint FILE, or the default set
#        scripts/lint-docs.sh --list      print the default set
#
# The default set is every tracked Markdown file, except vendored code, the
# third-party skill, and archived OpenSpec changes (they are history).
set -eu

cd "$(dirname "$0")/.."
lint=.claude/skills/asd-ste100/scripts/ste-lint.py

default_set() {
    git ls-files '*.md' |
        grep -vE '^(vendor|\.claude|openspec/changes/archive)/'
}

if [ "${1:-}" = --list ]; then
    default_set
    exit 0
fi

if [ "$#" -gt 0 ]; then
    exec python3 -I "$lint" "$@"
fi
# shellcheck disable=SC2046 # Split on newlines: the paths have no spaces.
exec python3 -I "$lint" $(default_set)
