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


def groups(node, into):
    if isinstance(node, dict):
        names = {k for k in node if "." not in k}
        if len(names) > 1:
            into.append(names)
        for value in node.values():
            groups(value, into)
    elif isinstance(node, list):
        for item in node:
            groups(item, into)
    return into


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


def head_keys(root):
    found = {}
    for base, _, names in os.walk(root):
        if "/build/" in base or "/test/" in base:
            continue
        for name in names:
            if not name.endswith(".kt"):
                continue
            path = os.path.join(base, name)
            for key in READS.findall(open(path).read()):
                found.setdefault(key, set()).add(name)
    return found


ACCEPTED = {
    "ok": "the invoke envelope, not a view field",
    "elements": "the bridge's list envelope",
    "shortDescription": "the vehicle's own fact group, read as a raw object because"
        "view.instrumentGroups enumerates the vehicle's CHILD groups and skips its own. That is"
        "where the four readings the flight screen starts with live, so without reading it the"
        "picker cannot draw them as chosen or let them be turned off. The macOS head builds the"
        "same group the same way",
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
}


def main():
    if not os.path.exists(CONTRACT):
        print(f"no contract at {CONTRACT}; set VIEW_SHAPES", file=sys.stderr)
        return 2
    contract = served(json.load(open(CONTRACT)), set())
    reads = head_keys(".")
    missing = {k: v for k, v in sorted(reads.items()) if k not in contract}

    for control, expected in [("altitudeFrame", True), ("foldedCommands", True), ("notAKeyAnyViewServes", False)]:
        present = control in contract
        if present != expected:
            print(f"CONTROL FAILED: {control} in contract = {present}, expected {expected}")
            return 3

    unexplained = {k: v for k, v in missing.items() if k not in ACCEPTED}
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

    beside = {}
    for names in groups(json.load(open(CONTRACT)), []):
        known = names & set(reads)
        if not known:
            continue
        for name in sorted(names - set(reads) - ACCEPTED.keys()):
            beside.setdefault(name, set()).update(sorted(known)[:3])

    seen = set()
    if os.path.exists(BASELINE):
        seen = {line.strip() for line in open(BASELINE) if line.strip()}
    fresh = {k: v for k, v in beside.items() if k not in seen}
    if "--baseline" in sys.argv:
        with open(BASELINE, "w") as handle:
            handle.write("\n".join(sorted(beside)) + "\n")
        print(f"\n  baseline written: {len(beside)} fields unread beside ones this head reads")
        return 0
    if fresh:
        print(f"\n  {len(fresh)} field(s) NEWLY served beside ones this head already reads:")
        for name, near in sorted(fresh.items()):
            print(f"    UNREAD BESIDE {', '.join(sorted(near))}: {name}")
        print("  A field the core adds to a shape you consume is invisible to every other check here.")
    return 1 if unexplained or stale or fresh else 0


if __name__ == "__main__":
    sys.exit(main())
