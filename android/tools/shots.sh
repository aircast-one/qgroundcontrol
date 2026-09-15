#!/bin/bash
# Screenshots of the tabs, refusing to save a picture of the wrong screen.
#
#   shots.sh [DIR] [TAB...]      default /tmp/shots and all five tabs
#
# Two failures made every earlier capture untrustworthy, and both are guarded here.
# `ui.sh pick` exits 1 when it cannot find the node and every script silenced it with
# >/dev/null 2>&1, so a tap that missed became nothing at all. And a confirm dialog
# left open swallows later taps, so the screen never changes while every command
# still succeeds - which is how a file called Plan.png ended up holding Connections.
set -u
export PATH="$PATH:$HOME/Library/Android/sdk/platform-tools"
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
if [ -z "${ANDROID_SERIAL:-}" ]; then
    ANDROID_SERIAL="$(adb devices | awk '$2 == "device" && $1 !~ /_adb\._tcp\./ { print $1; exit }')"
    [ -n "$ANDROID_SERIAL" ] && export ANDROID_SERIAL
fi

OUT="${1:-/tmp/shots}"; shift 2>/dev/null || true
TABS=("$@"); [ ${#TABS[@]} -eq 0 ] && TABS=(Fly Plan Setup Analyze Settings)
mkdir -p "$OUT"

screen() { adb shell uiautomator dump /sdcard/shots.xml > /dev/null 2>&1; adb shell cat /sdcard/shots.xml 2>/dev/null | md5; }

"$HERE/ui.sh" front > /dev/null || { echo "the app is not in front" >&2; exit 1; }
failed=0
for tab in "${TABS[@]}"; do
    # Step off the target first, so "the screen did not change" can only mean the tap
    # reached nothing - tapping the tab you are already on legitimately changes nothing.
    away=Fly; [ "$tab" = "Fly" ] && away=Plan
    "$HERE/ui.sh" pick "text=$away" > /dev/null 2>&1
    sleep 3
    before="$(screen)"
    if ! "$HERE/ui.sh" pick "text=$tab"; then
        echo "REFUSED $tab: ui.sh could not find that tab - a dialog may be holding the screen" >&2
        failed=1
        continue
    fi
    sleep 6
    if [ "$(screen)" = "$before" ]; then
        echo "REFUSED $tab: the screen did not change, so the tap reached nothing" >&2
        failed=1
        continue
    fi
    adb exec-out screencap -p > "$OUT/$tab.png"
    echo "$tab -> $OUT/$tab.png ($(wc -c < "$OUT/$tab.png") bytes)"
done
exit $failed
