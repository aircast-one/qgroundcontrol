#!/usr/bin/env python3
"""Measure the running QGC UI through the debug API.

Screenshots come back downscaled while clicks take scene coordinates, so anything read off an
image has to be converted or it lands somewhere else. Every command here does that conversion,
and `assert` checks layout from reported geometry rather than from pixels.
"""

import argparse
import json
import subprocess
import sys
import time
import urllib.error
import urllib.request

HEADER = {"X-QGC-Debug-Api": "1"}


def call(port, path, **params):
    query = "&".join(f"{k}={v}" for k, v in params.items())
    url = f"http://127.0.0.1:{port}{path}" + (f"?{query}" if query else "")
    request = urllib.request.Request(url, headers=HEADER)
    try:
        with urllib.request.urlopen(request, timeout=20) as response:
            payload = json.loads(response.read().decode())
        # The API answers 200 with an {"error": ...} body, so a caller that ignores it sees a
        # rejected request as a successful one. Every wrong-parameter drag looked like this.
        if isinstance(payload, dict) and "error" in payload:
            sys.exit(f"{path} rejected: {payload['error']}")
        return payload
    except urllib.error.HTTPError as error:
        # The API is up and refused this request. Reporting it as "not listening" would send
        # the reader chasing the wrong problem entirely.
        body = error.read().decode(errors="replace").strip()
        sys.exit(f"{path} rejected with HTTP {error.code}: {body or error.reason}")
    except (urllib.error.URLError, ConnectionError, TimeoutError, OSError) as error:
        # A stack trace here reads as a bug in the harness; the actual problem is almost always
        # that nothing is listening, and saying so is what lets someone act.
        sys.exit(f"no debug api on port {port} ({error}). "
                 f"Start the app with QGC_DEBUG_API_PORT={port} and run `ui-probe.py ready`.")


def wait_ready(port, timeout=60):
    deadline = time.time() + timeout
    while time.time() < deadline:
        try:
            call(port, "/status")
            return True
        except SystemExit:
            time.sleep(1)
    return False


def scene_scale(port):
    shot = call(port, "/screenshot")
    return shot.get("imageToScene", 1.0), shot


def named_items(port):
    tree = call(port, "/ui/tree")
    return [i for i in tree.get("items", []) if i.get("objectName")]


def cmd_shot(args):
    _, shot = scene_scale(args.port)
    if args.crop:
        left, top, width, height = (int(v) for v in args.crop.split(","))
        subprocess.run(
            ["ffmpeg", "-y", "-loglevel", "error", "-i", shot["imageFile"],
             "-vf", f"crop={width}:{height}:{left}:{top},scale={width * 2}:{height * 2}",
             args.out],
            check=True)
    else:
        subprocess.run(["cp", shot["imageFile"], args.out], check=True)
    print(json.dumps({"out": args.out, "scene": f"{shot['sceneWidth']}x{shot['sceneHeight']}",
                      "image": f"{shot['imageWidth']}x{shot['imageHeight']}",
                      "imageToScene": shot["imageToScene"]}))


def cmd_click(args):
    scale, _ = scene_scale(args.port)
    x, y = (args.x * scale, args.y * scale) if args.image else (args.x, args.y)
    print(json.dumps(call(args.port, "/ui/click", x=round(x), y=round(y))))


def cmd_connect(args):
    print(json.dumps(call(args.port, "/links/mocklink", autopilot=args.autopilot)))
    deadline = time.time() + 30
    while time.time() < deadline:
        if call(args.port, "/vehicle").get("connected"):
            call(args.port, "/ui/dismiss")
            print(json.dumps({"vehicle": "connected"}))
            return
        time.sleep(1)
    sys.exit("vehicle did not connect")


def cmd_drag(args):
    scale, _ = scene_scale(args.port)
    factor = scale if args.image else 1.0
    print(json.dumps(call(args.port, "/ui/drag",
                          x=round(args.x * factor), y=round(args.y * factor),
                          toX=round(args.to_x * factor), toY=round(args.to_y * factor),
                          steps=args.steps)))


def cmd_items(args):
    for item in named_items(args.port):
        if args.visible_only and not item.get("visible"):
            continue
        print(f"{item['objectName']:24} x={round(item['x']):5} y={round(item['y']):5} "
              f"w={round(item['width']):4} h={round(item['height']):4} visible={item.get('visible')}")


def _rect(item):
    return (item["x"], item["y"], item["x"] + item["width"], item["y"] + item["height"])


def cmd_overlaps(args):
    all_items = named_items(args.port)
    window = next((i for i in all_items if i["objectName"] == "MainWindow"), None)
    window_area = (window["width"] * window["height"]) if window else 0

    def is_full_surface(item):
        # Layers and gesture surfaces are meant to span the view; they cover every control by
        # design, so counting them as collisions only produces noise. Judged by size rather
        # than by a list of names, which would need editing every time one is added.
        return window_area and (item["width"] * item["height"]) >= window_area * 0.95

    # Popups and drawers likewise cover the view on purpose.
    items = [i for i in all_items
             if i.get("visible") and i["objectName"] not in args.ignore
             and "Popup" not in i.get("class", "") and "Drawer" not in i.get("class", "")
             and not is_full_surface(i)
             and i["width"] > 0 and i["height"] > 0]
    failures = []

    for item in items:
        left, top, right, bottom = _rect(item)
        if window and (left < -1 or top < -1 or right > window["width"] + 1
                       or bottom > window["height"] + 1):
            failures.append(f"{item['objectName']} extends outside the window")

    def contains(outer, inner):
        return outer["objectName"] in inner.get("namedAncestors", [])

    for i, first in enumerate(items):
        for second in items[i + 1:]:
            if contains(first, second) or contains(second, first):
                continue
            a, b = _rect(first), _rect(second)
            overlap_w = min(a[2], b[2]) - max(a[0], b[0])
            overlap_h = min(a[3], b[3]) - max(a[1], b[1])
            if overlap_w > args.tolerance and overlap_h > args.tolerance:
                failures.append(f"{first['objectName']} overlaps {second['objectName']} "
                                f"by {overlap_w:.0f}x{overlap_h:.0f}px")

    for failure in failures:
        print(f"FAIL {failure}")
    if failures:
        sys.exit(1)
    print(f"OK {len(items)} visible controls: none overlapping, none outside the window")


def cmd_assert(args):
    items = {i["objectName"]: i for i in named_items(args.port) if i.get("visible")}
    failures = []

    def require(name):
        if name not in items:
            failures.append(f"{name}: not visible")
            return None
        return items[name]

    cluster = [require(n) for n in args.column] if args.column else []
    cluster = [c for c in cluster if c]
    if len(cluster) > 1:
        centres = [c["x"] + c["width"] / 2 for c in cluster]
        spread = max(centres) - min(centres)
        if spread > args.tolerance:
            failures.append(f"column not aligned: centres span {spread:.0f}px "
                            f"(tolerance {args.tolerance})")
        ordered = sorted(cluster, key=lambda c: c["y"])
        if [c["objectName"] for c in ordered] != [c["objectName"] for c in cluster]:
            failures.append("column order on screen differs from the order given")

    for name in args.absent:
        if name in items:
            failures.append(f"{name}: visible but should not be")

    for failure in failures:
        print(f"FAIL {failure}")
    if failures:
        sys.exit(1)
    print(f"OK {len(items)} named items visible; layout assertions passed")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--port", type=int, default=8976)
    sub = parser.add_subparsers(dest="command", required=True)

    ready = sub.add_parser("ready", help="wait for the debug api")
    ready.set_defaults(func=lambda a: sys.exit(0 if wait_ready(a.port) else "debug api never came up"))

    connect = sub.add_parser("connect", help="start a mock vehicle and clear any dialog")
    connect.add_argument("--autopilot", default="apm")
    connect.set_defaults(func=cmd_connect)

    shot = sub.add_parser("shot", help="capture a screenshot")
    shot.add_argument("out")
    shot.add_argument("--crop", help="image-space left,top,width,height")
    shot.set_defaults(func=cmd_shot)

    click = sub.add_parser("click", help="click, converting image coordinates when asked")
    click.add_argument("x", type=float)
    click.add_argument("y", type=float)
    click.add_argument("--image", action="store_true", help="coordinates read off a screenshot")
    click.set_defaults(func=cmd_click)

    drag = sub.add_parser("drag", help="drag between two points")
    drag.add_argument("x", type=float)
    drag.add_argument("y", type=float)
    drag.add_argument("to_x", type=float)
    drag.add_argument("to_y", type=float)
    drag.add_argument("--steps", type=int, default=16)
    drag.add_argument("--image", action="store_true", help="coordinates read off a screenshot")
    drag.set_defaults(func=cmd_drag)

    items = sub.add_parser("items", help="list named items with geometry")
    items.add_argument("--visible-only", action="store_true")
    items.set_defaults(func=cmd_items)

    overlaps = sub.add_parser("overlaps", help="assert no visible control overlaps another or leaves the window")
    overlaps.add_argument("--ignore", nargs="*", default=["MainWindow"],
                          help="extra item names to skip")
    overlaps.add_argument("--tolerance", type=float, default=2)
    overlaps.set_defaults(func=cmd_overlaps)

    check = sub.add_parser("assert", help="check layout invariants")
    check.add_argument("--column", nargs="*", default=[], help="names that must share an x centre, top to bottom")
    check.add_argument("--absent", nargs="*", default=[], help="names that must not be visible")
    check.add_argument("--tolerance", type=float, default=12)
    check.set_defaults(func=cmd_assert)

    args = parser.parse_args()
    args.func(args)


if __name__ == "__main__":
    main()
