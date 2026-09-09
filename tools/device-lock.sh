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
    adb shell svc power stayon true >/dev/null 2>&1
    w=$(adb shell dumpsys power 2>/dev/null | grep -o 'mWakefulness=[A-Za-z]*' | head -1)
    echo "ACQUIRED ($w)"
    [ "$w" = "mWakefulness=Dozing" ] && echo "WARNING: still dozing - screencap will be a black 14KB frame"
    ;;
drop)
    if [ ! -f "$LOCK" ]; then echo "no lock held"; exit 0; fi
    o=$(owner_of)
    if [ "$o" != "$ME" ]; then echo "NOT MINE (held by $o) - left alone"; exit 1; fi
    adb shell svc power stayon false >/dev/null 2>&1
    adb shell dumpsys deviceidle enable >/dev/null 2>&1
    rm -f "$LOCK" && echo "RELEASED"
    ;;
show)
    [ -f "$LOCK" ] && echo "$(owner_of) | $(age_of)s | $(cut -d'|' -f3 "$LOCK")" || echo "free"
    ;;
esac
