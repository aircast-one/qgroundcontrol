#!/bin/zsh
# Run the unit suite from a renamed clone of the Debug build.
#
# Three traps, all of which have cost real time:
#  - a parallel session's `pkill -9 -x AircastQGC` kills a run using the real name,
#    so the clone gets its own executable name;
#  - a crashed run spins in QGC's signal handler at >100% CPU and ignores SIGTERM,
#    so leftovers are killed with SIGKILL before starting. Two of these once ran for
#    over two hours and made timing-sensitive link tests fail;
#  - several tests are timing-sensitive (LinkStateTest sets a 50ms stall threshold),
#    so the run waits for the machine to settle first.
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
stage="${TMPDIR:-/tmp}/qgc-testrun"
log="${1:-/tmp/qgc-unittest.log}"
suite="${2:-}"

pkill -9 -x QGCSuite 2>/dev/null || true

for _ in $(seq 1 30); do
    load=$(sysctl -n vm.loadavg | awk '{print int($2)}')
    [[ "$load" -lt 4 ]] && break
    sleep 10
done
echo "starting with load $(sysctl -n vm.loadavg | awk '{print $2}')"

rm -rf "$stage"; mkdir -p "$stage"
cp -Rc "$root/build-test/Debug/AircastQGC.app" "$stage/QGCSuite.app"
mv "$stage/QGCSuite.app/Contents/MacOS/AircastQGC" "$stage/QGCSuite.app/Contents/MacOS/QGCSuite"
/usr/libexec/PlistBuddy -c "Set :CFBundleExecutable QGCSuite" "$stage/QGCSuite.app/Contents/Info.plist" >/dev/null
codesign --force --sign - --timestamp=none "$stage/QGCSuite.app" 2>/dev/null

"$stage/QGCSuite.app/Contents/MacOS/QGCSuite" --allow-multiple \
    ${suite:+--unittest:$suite} ${suite:---unittest} > "$log" 2>&1 || true

pass=$(grep -c '^PASS' "$log" || true)
fail=$(grep -c '^FAIL' "$log" || true)
suites=$(grep -c 'Start testing' "$log" || true)
echo "PASS=$pass FAIL=$fail suites=$suites"
grep '^FAIL' "$log" || true
pkill -9 -x QGCSuite 2>/dev/null || true
[[ "$fail" -eq 0 ]]
