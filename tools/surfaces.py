import re
import subprocess
import sys
import time

import rig

TABS = ["Fly", "Plan", "Setup", "Params", "Analyze", "Settings"]
CHROME = {"Fly", "Plan", "Setup", "Params", "Analyze", "Settings", "Aircast"}


def controls():
    dump = subprocess.run(
        "adb exec-out uiautomator dump /dev/tty", shell=True, capture_output=True, text=True
    ).stdout
    nodes = []
    for node in re.finditer(r"<node[^>]*>", dump):
        tag = node.group(0)
        bounds = re.search(r'bounds="\[(\d+),(\d+)\]\[(\d+),(\d+)\]"', tag)
        if bounds:
            nodes.append((tag, tuple(int(g) for g in bounds.groups())))
    taps = [box for tag, box in nodes if 'clickable="true"' in tag]

    screen = max((t[2] - t[0]) * (t[3] - t[1]) for t in taps) if taps else 1

    def reachable(box):
        holding = [
            t for t in taps
            if t[0] <= box[0] and t[1] <= box[1] and t[2] >= box[2] and t[3] >= box[3]
        ]
        if not holding:
            return False
        smallest = min((t[2] - t[0]) * (t[3] - t[1]) for t in holding)
        return smallest < screen * 0.2

    found = []
    for tag, box in nodes:
        label = re.search(r'text="([^"]+)"', tag) or re.search(r'content-desc="([^"]+)"', tag)
        if not label:
            continue
        name = label.group(1).strip()
        if not name or name in CHROME or len(name) > 48:
            continue
        found.append((box[1], name, reachable(box)))
    seen = {}
    for y, name, clickable in sorted(found):
        seen[name] = seen.get(name, False) or clickable
    return seen


def main():
    rig.wake()
    rig.ensure_app()
    print("What each tab actually offers, read off the handset. A document cannot drift from this.")
    print("(*) marks a control the operator can tap.\n")
    for tab in TABS:
        where = rig.find(f"text={tab}")
        if not where:
            print(f"{tab}: NOT REACHABLE - no tab with that label")
            continue
        rig.sh(f"{rig.TOOLS}/ui.sh tap {where}")
        time.sleep(3)
        shown = controls()
        print(f"{tab} — {len(shown)} labels")
        for name, clickable in sorted(shown.items()):
            print(f"    {'*' if clickable else ' '} {name}")
        print()
    return 0


sys.exit(main())
