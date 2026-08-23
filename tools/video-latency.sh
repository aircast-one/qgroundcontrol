#!/usr/bin/env bash
# Per-element video latency for a running stream, ranked worst first.
#
#   tools/video-latency.sh [seconds] [path-to-AircastQGC]
#
# Runs the app under GStreamer's latency tracer, then reports mean/median/max
# time each element holds a buffer. The decoder and any queue that buffers for
# playback rather than for a live link show up at the top.
set -euo pipefail

SECONDS_TO_SAMPLE="${1:-40}"
APP="${2:-$(dirname "$0")/../build-aircast/Release/AircastQGC.app/Contents/MacOS/AircastQGC}"
TRACE="$(mktemp -t qgc-latency)"

if [ ! -x "$APP" ]; then
    echo "no executable at $APP" >&2
    exit 1
fi

pkill -x AircastQGC 2>/dev/null || true
sleep 3
for id in $(ipcs -m 2>/dev/null | awk '/^m /{print $2}'); do ipcrm -m "$id" 2>/dev/null || true; done

GST_TRACERS="latency(flags=pipeline+element)" GST_DEBUG="GST_TRACER:7" "$APP" > "$TRACE" 2>&1 &
APP_PID=$!
sleep "$SECONDS_TO_SAMPLE"
kill -9 "$APP_PID" 2>/dev/null || true

python3 - "$TRACE" <<'PY'
import collections, re, sys

samples = collections.defaultdict(list)
pattern = re.compile(r'element=\(string\)([^,]+), src=\(string\)([^,]+), time=\(guint64\)(\d+)')
for line in open(sys.argv[1], errors='ignore'):
    found = pattern.search(line)
    if found:
        samples[found.group(1)].append(int(found.group(3)))

if not samples:
    print("no tracer samples - was video actually playing?")
    raise SystemExit(1)

rows = []
for element, values in samples.items():
    values.sort()
    rows.append((sum(values) / len(values) / 1e6, values[len(values) // 2] / 1e6,
                 values[-1] / 1e6, len(values), element))
rows.sort(reverse=True)

print(f"{'element':<24}{'mean ms':>9}{'median':>9}{'max ms':>10}{'samples':>9}")
for mean, median, worst, count, element in rows[:15]:
    print(f"{element:<24}{mean:>9.2f}{median:>9.2f}{worst:>10.2f}{count:>9}")
print(f"\ntotal pipeline mean: {sum(r[0] for r in rows):.1f} ms")
PY

rm -f "$TRACE"
