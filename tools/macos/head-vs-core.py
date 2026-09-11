#!/usr/bin/env python3
"""Every fact the Plan window shows must equal the core's answer for it, on a live app.

Two defects this month were the same shape: the head read a view once, the value it read
was not final, and nothing re-read it. The plan summary sat one edit behind and showed a
distance belonging to the plan before the last waypoint. The terrain panel said the
mission cleared ground it flew into, because terrain heights land after the read that
asked for them. Neither is visible from a unit test -- both halves of the disagreement
are correct in isolation, and only a running app holds both at once.

A comparison of an empty plan agrees about everything, which is how the summary defect
survived: every row read "0 m" and so did the core. So this builds a plan with a real
distance over real terrain first, and REFUSES to report agreement until it has one.

Usage: QGC_PORT=8777 python3 tools/macos/head-vs-core.py
"""
import json
import os
import sys
import time
import urllib.error
import urllib.request

PORT = os.environ.get("QGC_PORT", "8777")
HEADERS = {"X-QGC-Debug-Api": "1"}

# Canberra, where terrain tiles are cached and the ground rises ~300 m across the leg.
LAUNCH = (-35.363, 149.165)
DISTANT = (-35.30, 149.30)
TERRAIN_SETTLE_SECONDS = 4


def ask(path):
    request = urllib.request.Request(f"http://127.0.0.1:{PORT}{path}", headers=HEADERS)
    return json.loads(urllib.request.urlopen(request, timeout=25).read())


def view(name):
    return ask(f"/bridge/get?path=view.{name}")


def probe(action=""):
    return ask(f"/native/probe?id=mission{action}")


def rows(listed):
    return {row["label"]: row["value"] for row in listed}


def build_plan():
    ask("/native/menu/invoke?path=Window/Plan")
    time.sleep(2.5)
    probe("&action=createPlan")
    time.sleep(1.0)
    for kind, (latitude, longitude) in (("takeoff", LAUNCH), ("waypoint", DISTANT)):
        probe(f"&action=arm&kind={kind}")
        time.sleep(0.3)
        probe(f"&action=addWaypoint&latitude={latitude}&longitude={longitude}")
        time.sleep(0.3)
    time.sleep(TERRAIN_SETTLE_SECONDS)


def comparisons():
    head = probe()["state"]
    core = {name: view(name) for name in ("plan", "missionSummary", "terrainProfile", "missionItems")}
    plan, summary, terrain, items = (core[name] for name in
                                     ("plan", "missionSummary", "terrainProfile", "missionItems"))
    return core, [
        ("plan is ready to save", head["readyToSave"], plan["readiness"]["ready"]),
        ("why it is not ready", head["notReadyReason"], plan["readiness"]["reason"]),
        ("plan is dirty", head["dirty"], plan["dirty"]),
        ("item count", head["count"], len(items["items"])),
        ("mission collides with terrain", head["terrain"]["collision"], terrain["hasCollision"]),
        ("points without terrain", head["terrain"]["unknown"], terrain["unknownTerrain"]),
        ("distance flown", rows(head["summary"]["rows"]).get("Distance"), rows(summary["rows"]).get("Distance")),
        ("how long it takes", rows(head["summary"]["rows"]).get("Time"), rows(summary["rows"]).get("Time")),
        ("altitude range", head["summary"]["altitudeRange"], summary["altitudeRange"]["text"]),
    ]


# A plan that agrees about nothing in particular is not evidence, so this refuses to report
# agreement until the scenario could actually have gone wrong.
#
# It asks the CORE, never the head. The first version asked the head, and when both watches
# were removed to prove this tool catches them, the head answered zero distance -- which IS
# the stale-summary defect -- and this read that as a trivial plan and refused to report it.
# A guard that consults the thing under test to decide whether to test it goes quiet exactly
# when it is needed.
def worth_comparing(core):
    items, summary, terrain = core["missionItems"], core["missionSummary"], core["terrainProfile"]
    missing = []
    if len(items["items"]) < 3:
        missing.append(f"the core holds {len(items['items'])} items, so there is no distance to get wrong")
    if rows(summary["rows"]).get("Distance", "0 m").startswith("0 "):
        missing.append("the core computes zero distance, so a stale summary would look correct")
    if not terrain["usable"]:
        missing.append("no terrain data arrived, so the terrain verdict cannot be wrong yet")
    return missing


def main():
    try:
        probe()
    except (urllib.error.URLError, OSError):
        sys.exit(f"no app answering on {PORT}; start one with tools/macos/build-run.sh")

    build_plan()
    core, checks = comparisons()

    if missing := worth_comparing(core):
        print("this run proves nothing:", file=sys.stderr)
        for line in missing:
            print(f"  {line}", file=sys.stderr)
        sys.exit(1)

    differ = [(what, mine, theirs) for what, mine, theirs in checks if str(mine) != str(theirs)]
    print(f"compared {len(checks)} facts against the core: "
          f"{len(checks) - len(differ)} agree, {len(differ)} DIFFER")
    for what, mine, theirs in differ:
        print(f"  DIFFER {what}\n    head: {mine!r}\n    core: {theirs!r}")
    sys.exit(1 if differ else 0)


main()
