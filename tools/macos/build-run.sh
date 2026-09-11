#!/bin/zsh
# Build, then (re)start the Debug app with the debug API on $QGC_PORT.
#
# Three traps this guards against, all of which silently leave you testing a stale
# binary: piping ninja into head sends SIGPIPE and kills the build mid-link; a
# SIGTERM'd Qt app can outlive a short sleep and keep the debug port, so the new
# instance fails to bind and every request answers from the old one; and a build can
# succeed without relinking the Rust archive into the dylib at all -- core-rs declared
# no output for a while, so ninja had no edge to rebuild across and a core-only change
# never reached the app. A green build was not evidence, which is why the check below
# compares the artefacts rather than the exit status.
set -euo pipefail
port="${QGC_PORT:-8779}"
root="$(cd "$(dirname "$0")/../.." && pwd)"
app="$root/build-test/Debug/AircastQGC.app/Contents/MacOS/AircastQGC"
log=/tmp/qgc-build.log

# Stop the old instance before building: the deploy step rewrites load commands with
# install_name_tool, which intermittently fails with "cannot rename ... (No such file
# or directory)" when the binary is still mapped by a running process.
# Only ever kill our own Debug build; other sessions run the Release build.
pkill -9 -f 'build-test/Debug/AircastQGC.app' 2>/dev/null || true
for _ in $(seq 1 30); do
    lsof -nP -iTCP:"$port" -sTCP:LISTEN >/dev/null 2>&1 || break
    sleep 1
done
if lsof -nP -iTCP:"$port" -sTCP:LISTEN >/dev/null 2>&1; then
    echo "port $port still held by: $(lsof -nP -iTCP:"$port" -sTCP:LISTEN | tail -1)" >&2
    exit 1
fi

cmake --build "$root/build-test" > "$log" 2>&1 || { grep -E 'error:|FAILED' "$log" | tail -20; exit 1; }
if grep -qE 'error:|FAILED' "$log"; then grep -E 'error:|FAILED' "$log" | tail -20; exit 1; fi

# The dylib has to be at least as new as the Rust archive linked into it. Nothing else
# here notices a core change that built but never got linked, and the comparison tool
# cannot either: it reads the head and the core through the same process, so a stale
# core makes both sides agree on the same stale answer.
dylib="$root/build-test/Debug/AircastQGC.app/Contents/Frameworks/libAircastQGC.dylib"
archive=$(ls -t "$root"/build-test/core-rs/*/libqgc_core.a 2>/dev/null | head -1)
if [[ -n "$archive" && -f "$dylib" && "$archive" -nt "$dylib" ]]; then
    echo "libqgc_core.a is newer than the dylib it should be inside: the Rust change did not reach the app" >&2
    exit 1
fi

QGC_DEBUG_API_PORT="$port" nohup "$app" --allow-multiple --native-window > /tmp/qgc-app.log 2>&1 &
pid=$!
echo "$pid" > /tmp/qgc-app.pid

for _ in $(seq 1 40); do
    if curl -s -m 1 -H 'X-QGC-Debug-Api: 1' "http://127.0.0.1:$port/status" >/dev/null 2>&1; then
        owner=$(lsof -nP -iTCP:"$port" -sTCP:LISTEN -t 2>/dev/null | head -1)
        [[ "$owner" == "$pid" ]] || { echo "port $port answered by pid $owner, not ours ($pid)" >&2; exit 1; }
        echo "app up on $port (pid $pid)"
        exit 0
    fi
    sleep 1
done
echo "app did not come up; see /tmp/qgc-app.log" >&2
exit 1
