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
SURVEY = (-35.28, 149.34)
RALLY = (-35.36, 149.17)
TERRAIN_SETTLE_SECONDS = 4
NO_ALTITUDE = "\u2014"


def ask(path):
    request = urllib.request.Request(f"http://127.0.0.1:{PORT}{path}", headers=HEADERS)
    return json.loads(urllib.request.urlopen(request, timeout=25).read())


def view(name):
    return ask(f"/bridge/get?path=view.{name}")


def probe(action=""):
    return ask(f"/native/probe?id=mission{action}")


def fence_probe(action=""):
    return ask(f"/native/probe?id=fenceRally{action}")


def rows(listed):
    return {row["label"]: row["value"] for row in listed}


# A survey is in the plan because its shot count and flown distance are computed after the read
# that draws them, which is the third thing this tool has had to be taught to look at.
def build_plan():
    ask("/native/menu/invoke?path=Window/Plan")
    time.sleep(2.5)
    probe("&action=createPlan")
    time.sleep(1.0)
    for kind, (latitude, longitude) in (("takeoff", LAUNCH), ("waypoint", DISTANT), ("survey", SURVEY)):
        probe(f"&action=arm&kind={kind}")
        time.sleep(0.5)
        probe(f"&action=addWaypoint&latitude={latitude}&longitude={longitude}")
        time.sleep(0.5)
    # A fence and a rally point, because the Fence and Rally tabs live in the same window that has
    # produced every staleness defect so far and are read the same way -- once, with no poll.
    # addFence takes no coordinates; it uses the map centre, which is the same map both sides read.
    for circle in ("0", "1"):
        fence_probe(f"&action=addFence&circle={circle}")
        time.sleep(0.6)
    fence_probe(f"&action=addRally&latitude={RALLY[0]}&longitude={RALLY[1]}")
    time.sleep(TERRAIN_SETTLE_SECONDS)


def selected_survey(core_items):
    return next((item["index"] for item in core_items
                 if item.get("kind") == "survey" and item.get("current")), None)


# Not every item has an altitude of its own. The plan's settings entry carries a planned home
# position altitude instead, and the head falls back to the coordinate's, so comparing its 585 m
# against a core answering null was this tool reporting a disagreement that was not one. Which
# source was used is in the label, so the fallback cannot quietly stand in for the real thing.
def altitude_check(seq, mine, theirs):
    # Both sides are compared as the operator sees them, now that the core formats the altitude
    # itself. That catches a divergence in the formatting as well as in the number, and it means
    # neither side has to parse the other's string back into metres to disagree with it.
    return (f"item {seq} altitude as shown", mine["altitude"], theirs["altitudeText"] or NO_ALTITUDE)


# Per item, and by sequence rather than by position, so a list that gained or lost one reports
# that rather than reporting every row after it as wrong.
def item_comparisons(head_items, core_items):
    mine = {item["seq"]: item for item in head_items}
    theirs = {item["sequence"]: item for item in core_items}
    # Keying by sequence drops a duplicate silently, and a list that quietly got shorter is how a
    # comparison agrees about items it never looked at. Both sides of these two are counts from
    # the same list, so they say so rather than borrowing the head-versus-core wording.
    collapsed = [
        (f"{side} list has one sequence number per item", f"{listed} items", f"{keyed} distinct")
        for side, listed, keyed in (("head", len(head_items), len(mine)),
                                    ("core", len(core_items), len(theirs)))
        if listed != keyed
    ]
    checks = collapsed + [(f"item {seq} exists in both", seq in mine, seq in theirs)
                          for seq in sorted(set(mine) | set(theirs))]
    return checks + [
        check
        for seq in sorted(set(mine) & set(theirs))
        for check in (
            (f"item {seq} name", mine[seq]["command"], theirs[seq]["name"]),
            (f"item {seq} has a position", mine[seq]["position"] != "\u2014",
             theirs[seq]["coordinate"] is not None),
            altitude_check(seq, mine[seq], theirs[seq]),
        )
    ]


# Only the selected survey has stats in the head, so the comparison follows the selection rather
# than assuming an index.
def survey_comparisons(head, core_items):
    index = selected_survey(core_items)
    if index is None:
        return [], None
    mine, theirs = head["surveyStats"], view(f"surveyStats({index})")
    return [
        (f"survey {index} photo count", mine["shots"], theirs["shotsText"]),
        (f"survey {index} distance flown", mine["distance"], theirs["distanceText"]),
        (f"survey {index} area covered", mine["area"], theirs["areaText"]),
        (f"survey {index} between shots", mine["interval"], theirs["intervalText"]),
        (f"survey {index} each photo covers", mine["footprint"], theirs["footprintText"]),
    ], theirs


# The head draws a fence by its kind, its one-line detail and its vertex count; the core computes
# all three. Compared as sorted sets rather than row by row: the head keeps one list and the core
# keeps polygons apart from circles, so pairing them by position would compare a polygon against a
# circle the moment a plan held two of one kind, and report a disagreement that was only an order.
def fence_comparisons(head, core):
    def described(shapes):
        return sorted(f"{kind} | {detail} | {vertices} vertices"
                      for kind, detail, vertices in shapes)

    drawn = described((row["kind"], row["detail"], row["vertices"]) for row in head["fence"])
    computed = described((shape["kindText"], shape["detailText"], len(shape.get("vertices") or []))
                         for shape in core["polygons"] + core["circles"])
    return [
        ("every fence the head draws is one the core computed", drawn, computed),
        ("rally count", len(head["rally"]), len(core["rallyPoints"])),
        ("rally positions", sorted(row["position"] for row in head["rally"]),
         sorted(f"{point['latitude']:.6f}, {point['longitude']:.6f}"
                for point in core["rallyPoints"])),
    ]


def comparisons():
    head = probe()["state"]
    core = {name: view(name) for name in ("plan", "missionSummary", "terrainProfile", "missionItems")}
    plan, summary, terrain, items = (core[name] for name in
                                     ("plan", "missionSummary", "terrainProfile", "missionItems"))
    survey_checks, core["surveyStats"] = survey_comparisons(head, items["items"])
    core["fences"] = view("fences")
    fences = fence_comparisons(fence_probe()["state"], core["fences"])
    return core, fences + survey_checks + item_comparisons(head["items"], items["items"]) + [
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
    survey = core.get("surveyStats")
    if survey is None:
        missing.append("no survey is selected, so its late-arriving numbers cannot be wrong yet")
    elif not survey.get("shotsText"):
        missing.append("the core computed no shot count, which is what the stale panel also showed")
    fences = core.get("fences") or {}
    if not (fences.get("polygons") or fences.get("circles")):
        missing.append("no fence was drawn, so the fence panel cannot be wrong about one")
    if not fences.get("rallyPoints"):
        missing.append("no rally point was placed, so that panel cannot be wrong either")
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
