#!/bin/bash
# device-lock.sh take "<what>"  -> 0 acquired, 1 held by someone else
# device-lock.sh drop           -> 0 released or nothing to do, 1 held by someone else
# device-lock.sh show
LOCK=/tmp/aircast-device.lock
ME="Android rewrite [5ddfae]"
STALE=600

owner_of() { cut -d'|' -f1 "$LOCK" 2>/dev/null; }
age_of()   { echo $(( $(date +%s) - $(cut -d'|' -f2 "$LOCK" 2>/dev/null || echo 0) )); }

case "$1" in
take)
    if [ -f "$LOCK" ]; then
        o=$(owner_of); a=$(age_of)
        if [ "$o" != "$ME" ] && [ "$a" -lt "$STALE" ]; then
            echo "HELD by $o for ${a}s: $(cut -d'|' -f3 "$LOCK")"; exit 1
        fi
        [ "$o" != "$ME" ] && echo "stealing stale lock from $o (${a}s old)"
    fi
    printf '%s|%s|%s\n' "$ME" "$(date +%s)" "${2:-adb work}" > "$LOCK"
    adb shell dumpsys deviceidle disable >/dev/null 2>&1
    adb shell svc power stayon usb >/dev/null 2>&1
    w=""
    for _ in 1 2 3; do
        w=$(adb shell dumpsys power 2>/dev/null | grep -o 'mWakefulness=[A-Za-z]*' | head -1)
        [ "$w" != "mWakefulness=Dozing" ] && break
        adb shell input keyevent 82 >/dev/null 2>&1
        python3 -c 'import time; time.sleep(2)'
    done
    if [ "$w" = "mWakefulness=Dozing" ]; then
        rm -f "$LOCK"
        echo "REFUSED: the handset will not wake - every capture would be a black frame" >&2
        exit 1
    fi
    adb shell settings put system accelerometer_rotation 0 >/dev/null 2>&1
    adb shell settings put system user_rotation 0 >/dev/null 2>&1
    r=$(adb shell dumpsys display 2>/dev/null | grep -oE 'mCurrentOrientation=[0-9]+' | head -1)
    if [ "$r" != "mCurrentOrientation=0" ]; then
        rm -f "$LOCK"
        echo "REFUSED: the handset is not portrait ($r) - every fixed coordinate would miss" >&2
        exit 1
    fi
    echo "ACQUIRED ($w portrait)"
    ;;
drop)
    if [ ! -f "$LOCK" ]; then echo "no lock held"; exit 0; fi
    o=$(owner_of)
    if [ "$o" != "$ME" ]; then echo "NOT MINE (held by $o) - left alone"; exit 1; fi
    adb shell svc power stayon false >/dev/null 2>&1
    adb shell settings put system accelerometer_rotation 1 >/dev/null 2>&1
    adb shell dumpsys deviceidle enable >/dev/null 2>&1
    rm -f "$LOCK" && echo "RELEASED"
    ;;
show)
    [ -f "$LOCK" ] && echo "$(owner_of) | $(age_of)s | $(cut -d'|' -f3 "$LOCK")" || echo "free"
    ;;
esac
