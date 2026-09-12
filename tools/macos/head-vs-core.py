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
REGION = (-35.29, 149.32)
HOME = (-35.27, 149.36)
BEYOND = (-35.25, 149.40)
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
    # The land goes before the last waypoint on purpose: items after a return or a landing are
    # uploaded and never flown, so this is the case where the route must stop short of the list.
    for kind, (latitude, longitude) in (("takeoff", LAUNCH), ("waypoint", DISTANT),
                                        ("roi", REGION), ("survey", SURVEY),
                                        ("land", HOME), ("waypoint", BEYOND)):
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
    # Only the selected item has survey stats, and the items added after the survey took the
    # selection with them.
    survey = next((item["sequence"] for item in view("missionItems")["items"]
                   if item["kind"] == "survey"), None)
    if survey is not None:
        probe(f"&action=select&sequence={survey}")
    time.sleep(TERRAIN_SETTLE_SECONDS)


# The editor's selection is one index on the view, not a flag on each item. It was a per-item
# "current" until the core renamed it -- the rename broke this tool and the head together, and
# this guard refusing to run is what surfaced it, because it asks the core rather than the head.
def selected_survey(core):
    chosen = core.get("selected")
    if chosen is None:
        return None
    return next((item["index"] for item in core["items"]
                 if item["index"] == chosen and item.get("kind") == "survey"), None)


# Not every item has an altitude of its own. The plan's settings entry carries a planned home
# position altitude instead, and the head falls back to the coordinate's, so comparing its 585 m
# against a core answering null was this tool reporting a disagreement that was not one. Which
# source was used is in the label, so the fallback cannot quietly stand in for the real thing.
# The shown altitude is COMPOSED: a measure the core formatted, plus a frame word this head adds.
# Comparing that composed string against the core's bare altitudeText reported three of my own
# commits as defects -- "585 m AMSL" against "585 m", and a survey's band against an em dash,
# because altitudeText is null for a pattern while altitudeBandText holds the figure drawn.
#
# Decomposed rather than re-derived. Re-implementing the head's choice of source here would be a
# fixture agreeing with its implementation: it would pass whatever the head did. These two checks
# assert what the head cannot satisfy by being wrong -- the measure is a string the CORE produced
# rather than one this head invented, and the frame word answers to the frame the CORE reported.
#
# It now DOES decide which of the two core strings the head should have picked, because
# b5462e307 added altitudeSource for exactly this consumer: "text", "band", or null for a row
# stating no height at all. Before it, this could only ask whether the measure was one of the
# two the core offered, which passes a head drawing the band where the text belonged. The
# core's own note is why the field exists rather than an inference: the two are mutually
# exclusive in every plan measured so far, and that is an OBSERVATION, not a guarantee - a row
# carrying minAMSLAltitude and an altitude fact produces both, and the core's test builds one.
FRAME_WORDS = {"amsl": "AMSL", "terrain": "AGL", "launch": "", "": ""}


def altitude_checks(seq, mine, theirs):
    shown = mine["altitude"]
    frame = theirs.get("altitudeFrame") or ""
    word = FRAME_WORDS.get(frame, frame.upper())
    suffix = f" {word}" if word else ""
    measure = shown[: -len(suffix)] if suffix and shown.endswith(suffix) else shown
    named = {"text": theirs.get("altitudeText"), "band": theirs.get("altitudeBandText")}
    return [
        (f"item {seq} altitude measure is the string the core named",
         measure, named.get(theirs.get("altitudeSource")) or NO_ALTITUDE),
        (f"item {seq} altitude names its frame",
         shown[len(measure):],
         suffix if measure != NO_ALTITUDE else ""),
    ]


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
            *altitude_checks(seq, mine[seq], theirs[seq]),
        )
    ]


# Only the selected survey has stats in the head, so the comparison follows the selection rather
# than assuming an index.
def survey_comparisons(head, core):
    index = selected_survey(core)
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


# What the core's own fields say the route should be: the placed legs, stopping wherever the
# mission ends. Computed from the core's answers and compared against what the head drew.
# Counted as POINTS, not items, and the distinction was a false red for a whole cycle: the head
# reported 4 where this said 3, and neither number was the "legs" the label claimed. A pattern
# contributes TWO route points -- the corner the aircraft enters by and the one it leaves by --
# which is what 40decbaf7 fixed after the route was seen doubling back 411 m across a survey.
# So an item-count and a point-count shared one name while measuring from different references.
def flown_after_end(core_items):
    ends = next((n for n, item in enumerate(core_items) if item["endsRoute"]), len(core_items))
    return [item for item in core_items[ends:] if item["flownLeg"] and item["coordinate"]]


def flown_route_points(core_items):
    ends = next((n for n, item in enumerate(core_items) if item["endsRoute"]), len(core_items))
    flown = [item for item in core_items[:ends] if item["flownLeg"] and item["coordinate"]]
    return sum(2 if item.get("exitCoordinate") else 1 for item in flown)


# Everything after the item that ends the route is uploaded and never reached. Derived from the
# core's own flags so the head's routeEnd derivation is checked against them rather than against
# a fixture this stream wrote. The plan built above ends in a return to launch with a waypoint
# after it, which is the only shape where this is not zero.
def unreached_after_route(core_items):
    ends = next((n for n, item in enumerate(core_items) if item["endsRoute"]), None)
    return 0 if ends is None else len(core_items) - (ends + 1)


def comparisons():
    head = probe()["state"]
    core = {name: view(name) for name in ("plan", "missionSummary", "terrainProfile", "missionItems")}
    plan, summary, terrain, items = (core[name] for name in
                                     ("plan", "missionSummary", "terrainProfile", "missionItems"))
    survey_checks, core["surveyStats"] = survey_comparisons(head, items)
    core["fences"] = view("fences")
    fences = fence_comparisons(fence_probe()["state"], core["fences"])
    return core, fences + survey_checks + item_comparisons(head["items"], items["items"]) + [
        # The markers and the route are different questions and were once one list, so the route
        # detoured through a region of interest the aircraft never visits.
        ("items with a place on the map", head["map"]["placed"],
         sum(1 for item in items["items"] if item["coordinate"])),
        ("points the route passes through", head["map"]["routePoints"],
         flown_route_points(items["items"])),
        ("rows marked never flown to", head["unreached"], unreached_after_route(items["items"])),
        # The head reads the selection from the view's own index; this asks the OTHER field the
        # core serves for it, the per-item flag, so the two sides come from different places and
        # the comparison can actually differ. Both replaced a per-item "current" the head went on
        # reading after it was renamed, which left nothing selected and no test noticing.
        ("the selected row", head["selected"],
         next((item["sequence"] for item in items["items"] if item.get("selected")), -1)),
        ("plan is ready to save", head["readyToSave"], plan["readiness"]["ready"]),
        ("why it is not ready", head["notReadyReason"], plan["readiness"]["reason"]),
        ("plan is dirty", head["dirty"], plan["dirty"]),
        ("item count", head["count"], len(items["items"])),
        ("mission collides with terrain", head["terrain"]["collision"], terrain["hasCollision"]),
        ("points without terrain", head["terrain"]["unknown"], terrain["unknownTerrain"]),
        # The head writes this sentence and the core supplies its number, so the check is that the
        # figure an operator reads is the core's and not one the head worked out from the plot.
        ("clearance the panel states", head["terrain"]["clearance"],
         f"Mission is below terrain by up to {terrain['clearanceText']}" if terrain["hasCollision"]
         else f"Clears terrain by {terrain['clearanceText']}"),
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
    if not terrain.get("clearanceText"):
        missing.append("the core measured no clearance, so the depth the panel states cannot be wrong")
    survey = core.get("surveyStats")
    if survey is None:
        missing.append("no survey is selected, so its late-arriving numbers cannot be wrong yet")
    elif not survey.get("shotsText"):
        missing.append("the core computed no shot count, which is what the stale panel also showed")
    fences = core.get("fences") or {}
    if not any(not item["flownLeg"] and item["coordinate"] for item in items["items"]):
        missing.append("no item has a place without a leg, so the route cannot detour through one")
    if not any(item["endsRoute"] for item in items["items"]):
        missing.append("nothing ends the mission, so the route cannot be drawn past the end of it")
    elif not flown_after_end(items["items"]):
        missing.append("nothing sits after the end of the mission, so stopping there proves nothing")
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
