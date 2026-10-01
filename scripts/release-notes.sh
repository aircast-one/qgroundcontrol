#!/usr/bin/env bash
set -euo pipefail

TAG="${1:?Usage: $0 <tag>}"
REPO="${GITHUB_REPOSITORY:-aircast-one/qgroundcontrol}"

if [[ "$TAG" =~ ^aircast-v[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    PREVIOUS=$(git tag -l 'aircast-v*' --merged "$TAG^" --sort=-v:refname | grep -E '^aircast-v[0-9]+\.[0-9]+\.[0-9]+$' | head -1 || true)
else
    PREVIOUS=$(git describe --tags --abbrev=0 --match 'aircast-v*' "$TAG^" 2>/dev/null || true)
fi
RANGE="${PREVIOUS:+$PREVIOUS..}$TAG"

section() {
    local lines
    lines=$(git log --no-merges --format='%s' "$RANGE" \
        | sed -nE "s/^($2)(\(([^)]*)\))?!?: (.*)$/- **\3:** \4/p" \
        | sed -E 's/^- \*\*:\*\* /- /')
    if [ -n "$lines" ]; then
        printf '## %s\n\n%s\n\n' "$1" "$lines"
    fi
}

section Features feat
section Fixes fix
section Performance perf

if [ -n "$PREVIOUS" ]; then
    echo "**Full Changelog**: https://github.com/$REPO/compare/$PREVIOUS...$TAG"
fi
