import json
import os
import re
import sys

CONTRACT = os.environ.get(
    "VIEW_SHAPES",
    os.path.expanduser("~/Code/aircast/qgroundcontrol/test/Bridge/fixtures/view-shapes.json"),
)
BASELINE = os.path.join(os.path.dirname(os.path.abspath(__file__)), "unread-baseline.txt")
READS = re.compile(r'\.opt(?:Text|Boolean|Int|Double|JSONObject|JSONArray|String)\(\s*"([A-Za-z][A-Za-z0-9]*)"')
NAMES_VIEW = re.compile(r'"(view\.[A-Za-z]+)')
NESTED = re.compile(r'\.optJSON(?:Object|Array)\(\s*"([A-Za-z][A-Za-z0-9]*)"')
HELPER_DEF = re.compile(r'fun JSON(?:Object|Array)\.([a-zA-Z][A-Za-z0-9]*)\(\s*[a-zA-Z]+: String')
TAKES_KEY = re.compile(
    r'fun ([a-zA-Z][A-Za-z0-9]*)\(\s*[a-zA-Z]+: JSON(?:Object|Array)\??\s*,\s*key: String'
)


def served(node, into):
    if isinstance(node, dict):
        for key, value in node.items():
            if "." not in key:
                into.add(key)
            served(value, into)
    elif isinstance(node, list):
        for item in node:
            served(item, into)
    return into


def groups(node, root, into):
    if isinstance(node, dict):
        names = {k for k in node if "." not in k}
        if len(names) > 1:
            into.append((root, names))
        for value in node.values():
            groups(value, root, into)
    elif isinstance(node, list):
        for item in node:
            groups(item, root, into)
    return into


def head_views(root):
    found = set()
    for base, _, names in os.walk(root):
        if "/build/" in base or "/test/" in base:
            continue
        for name in names:
            if name.endswith(".kt"):
                found.update(NAMES_VIEW.findall(open(os.path.join(base, name)).read()))
    return found


def recorded_null(node, into):
    if isinstance(node, dict):
        for key, value in node.items():
            if value == "null":
                into.add(key)
            else:
                recorded_null(value, into)
    elif isinstance(node, list):
        for item in node:
            recorded_null(item, into)
    return into


def opened_as_container(root):
    found = set()
    for base, _, names in os.walk(root):
        if "/build/" in base or "/test/" in base:
            continue
        for name in names:
            if name.endswith(".kt"):
                found |= set(NESTED.findall(open(os.path.join(base, name)).read()))
    return found


def recorded_empty(node, path, into):
    if isinstance(node, dict):
        for key, value in node.items():
            if value == ["empty"]:
                into.append(f"{path}/{key}")
            else:
                recorded_empty(value, f"{path}/{key}", into)
    elif isinstance(node, list):
        for item in node:
            recorded_empty(item, path, into)
    return into


def nullable_bools(node, path, into):
    if isinstance(node, dict):
        for key, value in node.items():
            if isinstance(value, str) and "null" in value.split("|") and "bool" in value.split("|"):
                into.setdefault(key, set()).add(path or key)
            else:
                nullable_bools(value, path or key, into)
    elif isinstance(node, list):
        for item in node:
            nullable_bools(item, path, into)
    return into


def flattened(root, nullable):
    # optBoolean returns false for JSON null, so a field the core declares bool|null and the
    # head reads this way cannot hold the absence - and false is the reassuring direction for
    # every one of them so far: "contact is fine", "checked and not ready", "nothing to hide".
    bare = re.compile(r'\.optBoolean\(\s*"([A-Za-z][A-Za-z0-9]*)"\s*\)')
    hits = {}
    for base, _, names in os.walk(root):
        if "/build/" in base or "/test/" in base:
            continue
        for name in names:
            if not name.endswith(".kt"):
                continue
            body = open(os.path.join(base, name)).read()
            for key in bare.findall(body):
                if key in nullable and key not in ACCEPTED_FLAT and f'isNull("{key}")' not in body:
                    hits.setdefault(key, set()).add(name)
    return hits


# A field name can be bool in the view a file reads and bool|null in another, so this check
# matches names rather than paths and needs the collisions named.
ACCEPTED_FLAT = {
    "ready": "PlanFileRules and VehicleSync read the plan's own ready, not view.setup.ready -"
        " SetupView.kt is the one that reads that, and it holds it as Boolean?",
    "stale": "view.detections.stale and an adsbTraffic contact's stale are both plain bool;"
        " only view.obstacle.stale is an Option, and ObstacleDistance.kt names its choice",
}

def head_keys(root):
    sources = {}
    for base, _, names in os.walk(root):
        if "/build/" in base or "/test/" in base:
            continue
        for name in names:
            if name.endswith(".kt"):
                sources[os.path.join(base, name)] = open(os.path.join(base, name)).read()

    # A read through a hand-written JSONObject extension is a read. bound("radiusMaximum") and
    # text("minString") were counted unread for as long as this check has existed, because the
    # pattern only knew the optX family. The helper names are derived from their own definitions
    # rather than listed, so a new one starts counting the day it is written.
    helpers = {h for src in sources.values() for h in HELPER_DEF.findall(src)}
    through = re.compile(
        r'\.(?:' + "|".join(sorted(helpers)) + r')\(\s*"([A-Za-z][A-Za-z0-9]*)"'
    ) if helpers else None

    # measureText(view, "gimbalPitch") is the same read with the object passed rather than
    # received. ItemCamera reads both gimbal measures this way and neither was counted.
    takers = {t for src in sources.values() for t in TAKES_KEY.findall(src)}
    passed = re.compile(
        r'\b(?:' + "|".join(sorted(takers)) + r')\(\s*[^,()]+,\s*"([A-Za-z][A-Za-z0-9]*)"'
    ) if takers else None

    found = {}
    for path, src in sources.items():
        keys = set(READS.findall(src))
        if through:
            keys |= set(through.findall(src))
        if passed:
            keys |= set(passed.findall(src))
        for key in keys:
            found.setdefault(key, set()).add(os.path.basename(path))
    return found


ONLY_IN = {
    "enumStrings": {"Qgc.kt"},
    "enumIndex": {"Qgc.kt"},
    "bitmaskStrings": {"Qgc.kt"},
    "bitmaskValues": {"Qgc.kt"},
    "typeIsBool": {"Qgc.kt"},
    "defaultValueString": {"Qgc.kt"},
    "maxString": {"Qgc.kt"},
    "minString": {"Qgc.kt"},
    "unknownEnumLabel": {"Qgc.kt"},
    "message": {"VehicleMessages.kt"},
}

ACCEPTED = {
    "ok": "the invoke envelope, not a view field",
    "message": "a field of armingChecks, which the contract records as NULL - a null parent pins "
        "its name and never its shape, so none of its children are here. Read in VehicleMessages.kt",
    "simulated": "served per contact at adsb.rs:386 and invisible here because"
        "view.adsbTraffic.contacts records EMPTY - the same blind spot this check already"
        "prints above. Read in TrafficView.kt since 457dbdda7",
    "elements": "the bridge's list envelope",
    "shortDescription": "the vehicle's own fact group, read as a raw object because"
        "view.instrumentGroups enumerates the vehicle's CHILD groups and skips its own. That is"
        "where the four readings the flight screen starts with live, so without reading it the"
        "picker cannot draw them as chosen or let them be turned off. The macOS head builds the"
        "same group the same way",
    "defaultValueString": "Fact metadata read by path",
    "maxString": "Fact metadata read by path",
    "minString": "Fact metadata read by path",
    "unknownEnumLabel": "Fact metadata read by path",
    "alert": "per-contact and invisible here because view.adsbTraffic.contacts records EMPTY, "
        "the blind spot this check prints below. Read in TrafficView.kt",
    "bearingDegrees": "per-contact, see alert",
    "relativeAltitude": "per-contact, see alert",
    "enumStrings": "Fact metadata read by path",
    "enumIndex": "Fact metadata read by path",
    "bitmaskStrings": "Fact metadata read by path",
    "bitmaskValues": "Fact metadata read by path",
    "typeIsBool": "Fact metadata read by path",
    "typeIsString": "Fact metadata read by path",
    "minIsDefaultForType": "Fact metadata read by path",
    "maxIsDefaultForType": "Fact metadata read by path",
    "qgcRebootRequired": "Fact metadata read by path",
    "width": "the video surface, not a core view",
    "height": "the video surface, not a core view",
    "url": "the extra-video-sources file, written by this head",
    "received": "the log download screen reads its own progress shape",
    "sizeText": "the log download screen reads its own progress shape",
    "timeState": "the log download screen reads its own progress shape",
    "notices": "view.notices is not in the recorded contract at all",
    "close": "view.obstacle recorded without the nearest object present",
    "sectorText": "view.obstacle recorded without the nearest object present",
    "w": "view.detections recorded with no box, so boxes[] pins no element shape",
    "h": "view.detections recorded with no box, so boxes[] pins no element shape",
    "confidence": "view.detections recorded with no box, so boxes[] pins no element shape",
    "callsign": "view.adsbTraffic recorded with contacts[] empty, so no contact shape is pinned;"
        " adsb.rs serves it and a probe read SWR000 from the live head",
    "icaoAddress": "view.adsbTraffic recorded with contacts[] empty; adsb.rs serves it",
    "altitudeType": "view.adsbTraffic recorded with contacts[] empty; adsb.rs serves it",
    "live": "modeslots.rs:92 serves it inside slots[], which the recording did not populate",
    "triggerCount": "geotag.rs:757 serves it, on a structure the recording has no instance of",
}


def main():
    if not os.path.exists(CONTRACT):
        print(f"no contract at {CONTRACT}; set VIEW_SHAPES", file=sys.stderr)
        return 2
    contract = served(json.load(open(CONTRACT)), set())
    # An absolute root rather than ".", because run from anywhere but android/ this walked
    # nothing and reported "0 read by the head" - a clean result from no data, which then
    # called every ACCEPTED entry stale.
    root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    reads = head_keys(root)
    if not reads:
        print(f"  REFUSING: no .kt reads found under {root}, so every count below would be meaningless")
        return 1
    missing = {k: v for k, v in sorted(reads.items()) if k not in contract}

    for control, expected in [("altitudeFrame", True), ("foldedCommands", True), ("notAKeyAnyViewServes", False)]:
        present = control in contract
        if present != expected:
            print(f"CONTROL FAILED: {control} in contract = {present}, expected {expected}")
            return 3

    # An acceptance keyed by NAME exempts every file that reads it. "Fact metadata read by path"
    # is true of Qgc.kt and was also covering ItemCamera.kt, which read enumStrings off a
    # view-served measure that has never carried one - the picker it fed drew nothing for as long
    # as it existed, and this check called it explained. Scoped acceptances are the fix.
    escaped = {
        k: sorted(v - ONLY_IN[k])
        for k, v in missing.items()
        if k in ONLY_IN and v - ONLY_IN[k]
    }
    unexplained = {k: v for k, v in missing.items() if k not in ACCEPTED}
    for key, files in sorted(escaped.items()):
        print(f"  ACCEPTED ELSEWHERE: {key} is exempt in {', '.join(sorted(ONLY_IN[key]))}, "
              f"but {', '.join(files)} reads it too - a different object with the same key name")
    stale = [k for k in ACCEPTED if k not in missing]
    print(f"{len(contract)} keys in the contract, {len(reads)} read by the head, "
          f"{len(missing)} not served, {len(unexplained)} unexplained")
    for key in stale:
        print(f"  STALE ACCEPTANCE: {key} is served now; delete its entry")
    for key, files in unexplained.items():
        print(f"  NOT SERVED: {key:<24} {', '.join(sorted(files))}")

    blind = recorded_empty(json.load(open(CONTRACT)), "", [])
    print(f"\n  {len(blind)} list(s) recorded empty. Their element fields are in no contract,")
    print("  so the core can rename or drop one and every check here stays green.")
    for where in sorted(blind):
        print(f"    EMPTY WHEN RECORDED: {where}")

    tree = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

    # An empty list hides its element fields; a key recorded NULL hides them just as completely,
    # and this check reported only the first. These are the ones the head opens and reads INTO,
    # so their children are decoded against a shape the contract does not carry.
    unpinned = sorted(recorded_null(json.load(open(CONTRACT)), set()) & opened_as_container(tree))
    if unpinned:
        print(f"\n  {len(unpinned)} key(s) recorded NULL that the head opens as an object or array:")
        print(f"    {', '.join(unpinned)}")
        print("  Their children are in no contract either, for the same reason as the empty lists.")

    drawn = head_views(tree)
    # A shape shares class and kind with every other view, so matching key NAMES alone counts a
    # view the head has no screen for as one it consumes. That folded 60 fields from packetRadio,
    # joystickMapping, gimbal and gpsRtkBase into a count of omissions beside fields we read.
    if not drawn:
        print("  REFUSING: no view path found in any .kt, so every shape would look unconsumed")
        return 1

    shapes = []
    for key, value in json.load(open(CONTRACT)).items():
        groups(value, key.split("(")[0], shapes)

    beside, absent = {}, {}
    for root, names in shapes:
        known = names & set(reads)
        if not known:
            continue
        target = beside if root in drawn else absent
        for name in sorted(names - set(reads) - ACCEPTED.keys()):
            target.setdefault(name, set()).add(root)
    absent = {k: v for k, v in absent.items() if k not in beside}

    seen = set()
    if os.path.exists(BASELINE):
        seen = {line.strip() for line in open(BASELINE) if line.strip()}
    fresh = {k: v for k, v in beside.items() if k not in seen}
    if "--baseline" in sys.argv:
        with open(BASELINE, "w") as handle:
            handle.write("\n".join(sorted(set(beside) | set(absent))) + "\n")
        print(f"\n  baseline written: {len(beside)} beside a read view, {len(absent)} in views with no screen")
        return 0
    if fresh:
        print(f"\n  {len(fresh)} field(s) NEWLY served beside ones this head already reads:")
        for name, near in sorted(fresh.items()):
            print(f"    UNREAD BESIDE {', '.join(sorted(near))}: {name}")
        print("  A field the core adds to a shape you consume is invisible to every other check here.")

    unbuilt = {k: v for k, v in absent.items() if k not in seen}
    if unbuilt:
        views = sorted({r for roots in unbuilt.values() for r in roots})
        print(f"\n  {len(unbuilt)} field(s) in {len(views)} view(s) this head has no screen for:")
        print(f"    {', '.join(views)}")
        print("  These are whole features, not fields missed beside ones we read - a different call.")
    nullable = nullable_bools(json.load(open(CONTRACT)), "", {})
    # A parse that finds nothing reports a clean head, so prove it can still see a known
    # bool|null before believing an empty result. view.flyState.contactLost has been one
    # since f97d66e13, and it is the field that motivated this check.
    if "contactLost" not in nullable:
        print("  REFUSING: the contract parse found no bool|null fields, so an empty result means nothing")
        return 1
    flat = flattened(os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))), nullable)
    if flat:
        print(f"\n  {len(flat)} field(s) the core can serve as null, read as a bare Boolean:")
        for name, where in sorted(flat.items()):
            print(f"    FLATTENED {name}: {', '.join(sorted(where))} - contract says {sorted(nullable[name])[0]} is bool|null")
        print("  optBoolean turns JSON null into false, and false is the reassuring answer every time.")

    return 1 if unexplained or stale or fresh or flat or escaped else 0


if __name__ == "__main__":
    sys.exit(main())
