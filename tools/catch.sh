#!/bin/bash
# Catch a message that does not stay on screen.
#
#   catch.sh <pattern> [seconds]      poll the screen until the pattern appears
#
# A notice shown for a couple of seconds is indistinguishable from no notice if
# you read the screen once at an arbitrary moment. That mistake has been made
# three times on this rig: a fence refusal, a survey refusal, and an upload
# confirmation reported to a peer as missing. Each dump costs about a second, so
# this samples continuously rather than sleeping between reads.
#
# Exit 0 and print the line when seen; exit 1 having printed every distinct
# screen it looked at, so a miss says what WAS there rather than nothing.
set -u
cd "$(dirname "$0")/.."
PATTERN="${1:?usage: catch.sh <pattern> [seconds]}"
LIMIT="${2:-10}"
DEADLINE=$(( $(date +%s) + LIMIT ))
SEEN=""

while [ "$(date +%s)" -lt "$DEADLINE" ]; do
    SCREEN=$(./tools/ui.sh text 2>/dev/null | tr '|' '\n')
    HIT=$(printf '%s\n' "$SCREEN" | grep -iE "$PATTERN" | head -1)
    if [ -n "$HIT" ]; then
        echo "SEEN: $HIT"
        exit 0
    fi
    DIGEST=$(printf '%s' "$SCREEN" | cksum)
    case "$SEEN" in
        *"$DIGEST"*) ;;
        *) SEEN="$SEEN $DIGEST"
           printf '%s\n' "$SCREEN" | grep -vE '^\s*$' | head -3 | sed 's/^/  looked at: /' ;;
    esac
done

echo "NOT SEEN in ${LIMIT}s: $PATTERN" >&2
exit 1
