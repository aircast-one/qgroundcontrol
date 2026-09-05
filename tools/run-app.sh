#!/bin/zsh
set -euo pipefail

app="${QGC_APP:-$(dirname "$0")/../build-test/Debug/AircastQGC.app}"
stage="$(mktemp -d "${TMPDIR:-/tmp}/aircastqgc-stage.XXXXXX")"

cp -Rc "$app" "$stage/"
"$stage/AircastQGC.app/Contents/MacOS/AircastQGC" "$@" &
child=$!

trap 'kill "$child" 2>/dev/null; wait "$child" 2>/dev/null; rm -rf "$stage"; exit 143' INT TERM HUP
trap 'rm -rf "$stage"' EXIT
wait "$child"
