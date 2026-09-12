#!/usr/bin/env bash
# Build the every-kind plan and SAY when a step is refused.
#
# This exists because of a method failure rather than a missing feature. The recipe for
# the every-kind plan was a row of curl calls with their output piped to /dev/null, and
# two of them -- addFence and addFence&circle=1 -- had been refused on every run all
# session with "This vehicle does not accept a geofence". Nothing said so, the saved plan
# quietly carried zero fences, and a standing note claimed fence creation was proven.
#
# A rig whose output nobody reads is an instrument that cannot fail. Required steps make
# this exit 1. Vehicle-gated steps report the refusal and do not, because a refusal with
# no vehicle attached is the correct answer rather than a broken rig.
#
# It ADDS to whatever plan is already open rather than starting one, so a second run on
# the same launch gives fourteen items and the printed counts double. Relaunch the app
# for a clean plan; createPlan resets but takes a kind and would drop the others.
set -u
port="${QGC_PORT:-8777}"
api=(-s -H "X-QGC-Debug-Api: 1")
failed=0

call() {
    local expectation=$1 what=$2 url=$3
    local answer; answer=$(curl "${api[@]}" "$url")
    if printf '%s' "$answer" | grep -q '"ok":true'; then
        printf '  ok       %s\n' "$what"
        return 0
    fi
    local why; why=$(printf '%s' "$answer" | sed -n 's/.*"error":"\([^"]*\)".*/\1/p')
    printf '  %-8s %s -- %s\n' "$expectation" "$what" "${why:-no ok in the answer}"
    [ "$expectation" = REFUSED ] || failed=1
}

probe="http://127.0.0.1:$port/native/probe"
call REQUIRED "open the Plan window" "$probe?id=plan.pages"

place() {
    call REQUIRED "arm $1" "$probe?id=mission&action=arm&kind=$1"
    call REQUIRED "place $1" "$probe?id=mission&action=addWaypoint&latitude=$2&longitude=$3"
}
place takeoff   47.3977 8.5456
place waypoint  47.3995 8.5480
place roi       47.4000 8.5490
place survey    47.4010 8.5500
place corridor  47.4040 8.5560
place structure 47.4070 8.5620
place land      47.4090 8.5660

call REQUIRED "select the waypoint" "$probe?id=mission&action=select&sequence=2"
call REQUIRED "command a speed"     "$probe?id=mission&action=itemSpeed&on=1&value=9"
call REQUIRED "set a hold"          "$probe?id=mission&action=setFact&name=Hold&value=20"

# Both are gated on a connected vehicle: view.fences answers fenceSupported false and
# rallySupported false with none attached, so these are expected to be refused here.
call REFUSED "add a polygon fence" "$probe?id=fenceRally&action=addFence"
call REFUSED "add a circular fence" "$probe?id=fenceRally&action=addFence&circle=1"

curl "${api[@]}" "$probe?id=mission" | python3 -c '
import json, sys
state = json.load(sys.stdin).get("state") or {}
named = ("count", "routePoints", "transects", "surveys", "corridors")
print("  plan: " + ", ".join(f"{name} {state.get(name)}" for name in named))
'

[ "$failed" = 0 ] && echo "every required step was accepted" || echo "a REQUIRED step was refused"
exit "$failed"
