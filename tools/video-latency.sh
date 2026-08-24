#!/usr/bin/env bash
# Video latency for a running stream: source-to-sink total, then per element.
#
#   tools/video-latency.sh [seconds] [path-to-AircastQGC]
#
# Runs the app under GStreamer's latency tracer. The headline number is how long
# a buffer takes from the source pad to the sink - everything QGC controls once
# the packet arrives. Camera encode and radio time are upstream of this and are
# not included. The per-element table shows where that time goes.
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
import collections, re, statistics, sys

pipeline = collections.defaultdict(list)
elements = collections.defaultdict(list)

pipeline_pattern = re.compile(
    r'latency, src-element-id=\(string\)[^,]+, src-element=\(string\)([^,]+),'
    r'.*?sink-element=\(string\)([^,]+),.*?time=\(guint64\)(\d+)')
element_pattern = re.compile(
    r'latency, element-id=\(string\)[^,]+, element=\(string\)([^,]+),'
    r' src=\(string\)[^,]+, time=\(guint64\)(\d+)')

for line in open(sys.argv[1], errors='ignore'):
    found = pipeline_pattern.search(line)
    if found:
        pipeline[f"{found.group(1)} -> {found.group(2)}"].append(int(found.group(3)))
        continue
    found = element_pattern.search(line)
    if found:
        elements[found.group(1)].append(int(found.group(2)))

if not pipeline and not elements:
    print("no tracer samples - was video actually playing?")
    raise SystemExit(1)

def summarise(values):
    values = sorted(values)
    return (statistics.median(values) / 1e6,
            sum(values) / len(values) / 1e6,
            values[int(len(values) * 0.95)] / 1e6 if len(values) > 20 else values[-1] / 1e6,
            len(values))

if pipeline:
    print("SOURCE TO SINK (what QGC adds after the packet arrives)")
    print(f"{'path':<44}{'median':>9}{'mean':>9}{'p95':>9}{'samples':>9}")
    for path, values in sorted(pipeline.items(), key=lambda kv: -statistics.median(kv[1])):
        median, mean, p95, count = summarise(values)
        print(f"{path:<44}{median:>9.1f}{mean:>9.1f}{p95:>9.1f}{count:>9}")
    print()

print("PER ELEMENT")
print(f"{'element':<28}{'median':>9}{'mean':>9}{'p95':>9}{'samples':>9}")
ranked = sorted(elements.items(), key=lambda kv: -statistics.median(kv[1]))
for element, values in ranked[:12]:
    median, mean, p95, count = summarise(values)
    print(f"{element:<28}{median:>9.2f}{mean:>9.2f}{p95:>9.2f}{count:>9}")
PY

rm -f "$TRACE"
