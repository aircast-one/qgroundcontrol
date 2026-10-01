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

read -r -d '' SYSTEM_PROMPT <<'EOF' || true
You write GitHub release notes for Aircast QGC, a fork of the QGroundControl ground control station used by drone pilots and operators. You receive the git commit messages of one release.

Write the notes for pilots and operators, not developers:
- Lead with what changed for them, grouped under short "## " headings such as the device or area affected. Use one "## Changes" heading when the release is small.
- One bullet per change, starting with a bold short name, then one or two plain sentences on what it does or what was broken and is now fixed.
- Leave out commits that do not change the app for its users: CI, build, tests, formatting, refactoring, release tooling.
- Use only facts present in the commit messages. Do not guess versions, devices or behaviour.
- Refer to the SkyDroid H16 remote only by that name.
- No title, no introduction, no closing remarks, no commit hashes, no links. Output only the markdown.
- If no commit changes the app for its users, output exactly: Maintenance release with no user-facing changes.
EOF

commit_log() {
    git log --no-merges --format='--- %s%n%n%b' "$RANGE" | grep -vE '^(Claude-Session|Co-Authored-By):'
}

claude_notes() {
    command -v claude >/dev/null || return 1
    commit_log | claude -p "Write the release notes for these commits." \
        --system-prompt "$SYSTEM_PROMPT" \
        --tools "" \
        --strict-mcp-config \
        --no-session-persistence
}

section() {
    local lines
    lines=$(git log --no-merges --format='%s' "$RANGE" \
        | sed -nE "s/^($2)(\(([^)]*)\))?!?: (.*)$/- **\3:** \4/p" \
        | sed -E 's/^- \*\*:\*\* /- /')
    if [ -n "$lines" ]; then
        printf '## %s\n\n%s\n\n' "$1" "$lines"
    fi
}

if NOTES=$(claude_notes) && [ -n "$NOTES" ]; then
    printf '%s\n\n' "$NOTES"
else
    echo "Claude did not write the notes; listing commits instead." >&2
    section Features feat
    section Fixes fix
    section Performance perf
fi

if [ -n "$PREVIOUS" ]; then
    echo "**Full Changelog**: https://github.com/$REPO/compare/$PREVIOUS...$TAG"
fi
