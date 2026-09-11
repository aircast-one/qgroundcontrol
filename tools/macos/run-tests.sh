#!/bin/zsh
# Run the unit suite from a renamed clone of the Debug build.
#
# Four traps, all of which have cost real time:
#  - a parallel session's `pkill -9 -x AircastQGC` kills a run using the real name,
#    so the clone gets its own executable name;
#  - a crashed run spins in QGC's signal handler at >100% CPU and ignores SIGTERM,
#    so leftovers are killed with SIGKILL before starting. Two of these once ran for
#    over two hours and made timing-sensitive link tests fail;
#  - several tests are timing-sensitive (LinkStateTest sets a 50ms stall threshold),
#    so the run waits for the machine to settle first;
#  - two suites at once share one QSettings space and corrupt each other. A run that
#    overlapped another session's died on a SIGSEGV that would not reproduce alone, and
#    cost that session an afternoon proving the crash was real and separate. Checking by
#    eye does not work: I printed exactly this check once, watched it say another suite
#    was running, and let the next command go anyway.
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
stage="${TMPDIR:-/tmp}/qgc-testrun"
log="${1:-/tmp/qgc-unittest.log}"
suite="${2:-}"

pkill -9 -x QGCSuite 2>/dev/null || true

# A suite always runs from inside a bundle, which is what tells it apart from a shell that merely
# mentions the flag. pgrep -f was the first attempt and it matched the very command line asking
# the question -- the same trap that makes build-run.sh's pkill kill this session's own shells.
foreign_suites() {
    ps -axo command= | awk '$1 ~ /\.app\/Contents\/MacOS\// && $1 !~ /QGCSuite/ && /--unittest/' | wc -l
}

# Truncated before the gate, not after it. Every caller here parses this log after invoking the
# script, so a run that refuses to start would otherwise be read as whatever the last one said --
# reporting a green nobody saw, from the tool whose whole job is to stop exactly that.
: > "$log"

others=0
for _ in $(seq 1 60); do
    others=$(foreign_suites)
    (( others == 0 )) && break
    sleep 10
done
if (( others > 0 )); then
    print "REFUSED: another session's unit suite is still running after ten minutes" >> "$log"
    print -u2 "another session's unit suite is still running after ten minutes; not starting a second"
    exit 1
fi

# Waiting for a completely idle machine never converges here (Spotlight and parallel
# sessions keep it around 4-6), so settle for "not thrashing" and move on.
for _ in $(seq 1 12); do
    load=$(sysctl -n vm.loadavg | awk '{print int($2)}')
    [[ "$load" -lt 8 ]] && break
    sleep 10
done
echo "starting with load $(sysctl -n vm.loadavg | awk '{print $2}')"

# Refresh one clone in place rather than making a new one per run. `codesign --force`
# breaks APFS reflink dedup, so every clone is a full physical copy of a ~100MB bundle;
# recreating it each run filled the disk hard enough to block every tool.
app="$root/build-test/Debug/AircastQGC.app"
clone="$stage/QGCSuite.app"
if [[ ! -d "$clone" ]] || [[ "$app/Contents/MacOS/AircastQGC" -nt "$clone/Contents/MacOS/QGCSuite" ]]; then
    rm -rf "$stage"; mkdir -p "$stage"
    cp -Rc "$app" "$clone"
    mv "$clone/Contents/MacOS/AircastQGC" "$clone/Contents/MacOS/QGCSuite"
    /usr/libexec/PlistBuddy -c "Set :CFBundleExecutable QGCSuite" "$clone/Contents/Info.plist" >/dev/null
    codesign --force --sign - --timestamp=none "$clone" 2>/dev/null
fi

"$clone/Contents/MacOS/QGCSuite" --allow-multiple \
    ${suite:+--unittest:$suite} ${suite:---unittest} 2>&1 | head -c 200000000 > "$log" || true

pass=$(grep -c '^PASS' "$log" || true)
fail=$(grep -c '^FAIL' "$log" || true)
suites=$(grep -c 'Start testing' "$log" || true)
echo "PASS=$pass FAIL=$fail suites=$suites"
grep '^FAIL' "$log" || true
pkill -9 -x QGCSuite 2>/dev/null || true
[[ "$fail" -eq 0 ]]
