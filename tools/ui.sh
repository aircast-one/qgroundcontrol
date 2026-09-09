#!/bin/bash
# Input that refuses to go anywhere but the app.
#
# `adb shell am start` returns before the window is up, and a tap sent into that gap lands in
# whatever the user had open — twice, in one session, in their private messages. Every tap here
# checks topResumedActivity first and exits non-zero rather than guess.
#
#   ui.sh front            bring the app forward and wait for it
#   ui.sh tap X Y          tap, only if the app is in front
#   ui.sh swipe X1 Y1 X2 Y2 [MS]
#   ui.sh text "..."       type, only if the app is in front
#   ui.sh key KEYCODE
#   ui.sh shot FILE        screencap, and fail a capture too small to be a live screen
export PATH="$PATH:$HOME/Library/Android/sdk/platform-tools"
APP="${APP:-one.aircast.android}"
ACTIVITY="${ACTIVITY:-.MainActivity}"
set -u

in_front() {
    adb shell dumpsys activity activities 2>/dev/null |
        grep -q "topResumedActivity.*$APP/"
}

require_front() {
    if ! in_front; then
        echo "REFUSED: $APP is not in front - run 'ui.sh front' first" >&2
        exit 1
    fi
}

case "${1:-}" in
front)
    adb shell am start -W -n "$APP/$ACTIVITY" >/dev/null 2>&1
    for _ in $(seq 1 30); do
        if in_front; then echo "front: $APP"; exit 0; fi
        python3 -c "import time; time.sleep(1)"
    done
    echo "FAILED: $APP never came to the front" >&2
    exit 1
    ;;
tap)     require_front; adb shell input tap "$2" "$3" ;;
swipe)   require_front; adb shell input swipe "$2" "$3" "$4" "$5" "${6:-400}" ;;
text)    require_front; adb shell input text "$2" ;;
key)     require_front; adb shell input keyevent "$2" ;;
shot)
    adb exec-out screencap -p > "$2"
    bytes=$(stat -f%z "$2")
    if [ "$bytes" -lt 100000 ]; then
        echo "FAIL: $2 is ${bytes}B - screen asleep?" >&2
        exit 1
    fi
    echo "ok $2 (${bytes}B)"
    ;;
*)
    sed -n '2,14p' "$0"
    exit 2
    ;;
esac
