#!/bin/bash
# Read the bridge directly instead of inferring a value off a screen.
#
#   probe.sh on                  enable the debug API and forward the port
#   probe.sh get <path>          read a bridge path or a view
#   probe.sh set <path> <json>   write a bridge path
#   probe.sh raw <route> [args]  any debug-api route, e.g. raw /status
#
# The deep link MUST name the activity: two installed apps claim aircast-qgc://
# (this head and the QML QGCActivity), so an untargeted intent opens a chooser
# and the link never arrives.
#
# A reinstall drops the server - it starts from a deep link, not a setting - and
# an empty response reads exactly like a refusal, so get/set re-enable first.
export PATH="$PATH:$HOME/Library/Android/sdk/platform-tools"
PORT="${QGC_DEBUG_PORT:-8790}"
APP="one.aircast.android"
HEADER="X-QGC-Debug-Api: 1"

case "${1:-}" in
on)
    adb shell am start -n "$APP/.MainActivity" -a android.intent.action.VIEW \
        -d "aircast-qgc://probe?debug=$PORT" > /dev/null 2>&1
    sleep 3
    adb forward "tcp:$PORT" "tcp:$PORT" > /dev/null 2>&1
    if curl -s --max-time 5 -H "$HEADER" "http://127.0.0.1:$PORT/status" > /dev/null; then
        echo "probe listening on $PORT"
    else
        echo "REFUSED: no answer on $PORT - is $APP running?" >&2
        exit 1
    fi
    ;;
get)
    [ -n "${2:-}" ] || { echo "usage: probe.sh get <path>" >&2; exit 2; }
    curl -s --max-time 3 -H "$HEADER" "http://127.0.0.1:$PORT/status" > /dev/null || "$0" on > /dev/null
    # curl --data-urlencode writes a space as "+", which QUrl::FullyDecoded leaves as a literal
    # plus: a path under "Aircast QGC Daily" came back as "Aircast+QGC+Daily" and read as
    # unreadable. percent-encode it ourselves so a space arrives as %20.
    curl -s --max-time 8 -H "$HEADER" \
        "http://127.0.0.1:$PORT/bridge/get?path=$(python3 -c 'import sys,urllib.parse; print(urllib.parse.quote(sys.argv[1], safe=""))' "$2")"
    echo ""
    ;;
set)
    [ -n "${3:-}" ] || { echo "usage: probe.sh set <path> <json-value>" >&2; exit 2; }
    curl -s --max-time 8 -H "$HEADER" -G "http://127.0.0.1:$PORT/bridge/set" \
        --data-urlencode "path=$2" --data-urlencode "value=$3"
    echo ""
    ;;
raw)
    [ -n "${2:-}" ] || { echo "usage: probe.sh raw <route>" >&2; exit 2; }
    curl -s --max-time 8 -H "$HEADER" "http://127.0.0.1:$PORT$2"
    echo ""
    ;;
*)
    sed -n '2,10p' "$0"
    exit 2
    ;;
esac
