import subprocess
import sys
import time

from rig import TOOLS, find, new_plan, sh, tap_label, view

FAILURES = []


def locate(colour, nth):
    out = sh(f"python3 {TOOLS}/onmap.py '{colour}' --nth {nth}")
    if not out:
        raise SystemExit(f"PRECONDITION: nothing matching {colour} on screen")
    parts = out.split()
    return int(parts[0]), int(parts[1])


def drag(x1, y1, x2, y2):
    sh(f"adb shell input swipe {x1} {y1} {x2} {y2} 800")
    time.sleep(3)


def check(name, before, after, expectation):
    if before == after:
        FAILURES.append(f"{name}: nothing changed - {expectation}")
        print(f"  FAIL {name}: unchanged ({before})")
    else:
        print(f"  ok   {name}: {before} -> {after}")


def circles():
    return [
        (round(c["centre"]["latitude"], 6), round(c["centre"]["longitude"], 6))
        for c in (view("view.fences").get("circles") or [])
    ]


def polygon_vertices():
    polygons = view("view.fences").get("polygons") or []
    if not polygons:
        return []
    return [(round(v["latitude"], 6), round(v["longitude"], 6)) for v in polygons[0].get("vertices") or []]


def main():
    print("fence editing, end to end. Every step asserts a change in what the core serves,")
    print("not a change on screen - a tap that misses looks identical to a path that is broken.")

    new_plan()

    tap_label("Circle")
    if len(circles()) != 1:
        raise SystemExit(f"PRECONDITION: expected one circle after Circle, got {len(circles())}")
    tap_label("Fit")

    before = circles()
    x, y = locate("keep-in", 1)
    drag(x, y, x + 150, y)
    check("circle drag writes a new centre", before, circles(), "MapHit.Circle drag reached FenceBridge.moveCircle")

    new_plan()
    if circles():
        raise SystemExit("PRECONDITION: a circle survived New plan; its centre handle would be "
                         "indistinguishable from a polygon vertex handle")

    tap_label("Fence")
    tap_label("Fit")
    if len(polygon_vertices()) < 3:
        raise SystemExit("PRECONDITION: Fence did not add a polygon")

    before = polygon_vertices()
    listing = subprocess.run(
        f"python3 {TOOLS}/onmap.py '#1565C0'",
        shell=True, capture_output=True, text=True,
    )
    handles = [
        line.strip() for line in (listing.stdout + listing.stderr).splitlines()
        if line.strip().startswith("--nth")
    ]
    def span_of(line):
        for word in line.split():
            if word.startswith("span="):
                return int(word[len("span="):].split("x")[0])
        return 0

    big = [line for line in handles if span_of(line) >= 35]
    if not big:
        raise SystemExit("PRECONDITION: no vertex handles found on screen")
    parts = big[0].split()
    hx, hy = int(parts[2]), int(parts[3])
    drag(hx, hy, hx - 80, hy - 80)
    check("vertex drag writes a new corner", before, polygon_vertices(), "MapHit.FenceVertex drag reached the bridge")

    before = len(polygon_vertices())
    small = [line for line in handles if span_of(line) < 35]
    if small:
        parts = small[0].split()
        mx, my = int(parts[2]), int(parts[3])
        sh(f"adb shell input tap {mx} {my}")
        time.sleep(4)
        check("midpoint tap adds a corner", before, len(polygon_vertices()), "MapHit.Midpoint invoked its split")
    else:
        FAILURES.append(
            "midpoint tap: no midpoint marker located, so this path went untested. "
            "A skipped step is not a passing one."
        )
        print("  FAIL midpoint tap: no midpoint marker located, path untested")

    if FAILURES:
        print(f"\n{len(FAILURES)} FAILED:")
        for line in FAILURES:
            print("  " + line)
        sys.exit(1)
    print("\nall fence editing paths write")


main()
