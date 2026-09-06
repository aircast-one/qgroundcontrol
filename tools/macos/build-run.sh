#!/bin/zsh
# Build, then (re)start the Debug app with the debug API on $QGC_PORT.
#
# Two traps this guards against, both of which silently leave you testing a stale
# binary: piping ninja into head sends SIGPIPE and kills the build mid-link, and a
# SIGTERM'd Qt app can outlive a short sleep and keep the debug port, so the new
# instance fails to bind and every request answers from the old one.
set -euo pipefail
port="${QGC_PORT:-8779}"
root="$(cd "$(dirname "$0")/../.." && pwd)"
app="$root/build-test/Debug/AircastQGC.app/Contents/MacOS/AircastQGC"
log=/tmp/qgc-build.log

cmake --build "$root/build-test" > "$log" 2>&1 || { grep -E 'error:|FAILED' "$log" | tail -20; exit 1; }
if grep -qE 'error:|FAILED' "$log"; then grep -E 'error:|FAILED' "$log" | tail -20; exit 1; fi

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
