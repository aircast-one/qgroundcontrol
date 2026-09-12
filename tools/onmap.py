import subprocess
import sys

from PIL import Image

KNOWN = {
    "keep-in": (0xFF, 0x95, 0x00),
    "keep-out": (0xFF, 0x3B, 0x30),
}

TOP = 380
BOTTOM = 1700
STEP = 2
TOLERANCE = 26


def parse(colour):
    if colour in KNOWN:
        return KNOWN[colour]
    text = colour.lstrip("#")
    if len(text) != 6:
        raise SystemExit(f"not a colour: {colour} (use #RRGGBB or one of {', '.join(KNOWN)})")
    return tuple(int(text[i:i + 2], 16) for i in (0, 2, 4))


def grab(serial):
    shot = subprocess.run(
        ["adb", "-s", serial, "exec-out", "screencap", "-p"],
        capture_output=True,
        check=True,
    ).stdout
    path = "/tmp/onmap.png"
    with open(path, "wb") as handle:
        handle.write(shot)
    return Image.open(path).convert("RGB")


def matching(image, target):
    return [
        (x, y)
        for y in range(TOP, min(BOTTOM, image.height), STEP)
        for x in range(0, image.width, STEP)
        if all(abs(a - b) <= TOLERANCE for a, b in zip(image.getpixel((x, y)), target))
    ]


BRIDGE = 12


def cluster(points):
    cells = {}
    for point in points:
        cells.setdefault((point[0] // BRIDGE, point[1] // BRIDGE), []).append(point)
    parent = {cell: cell for cell in cells}

    def root(cell):
        while parent[cell] != cell:
            parent[cell] = parent[parent[cell]]
            cell = parent[cell]
        return cell

    for cell in cells:
        for dx in (-1, 0, 1):
            for dy in (-1, 0, 1):
                other = (cell[0] + dx, cell[1] + dy)
                if other in parent:
                    a, b = root(cell), root(other)
                    if a != b:
                        parent[a] = b
    groups = {}
    for cell, members in cells.items():
        groups.setdefault(root(cell), []).extend(members)
    return sorted(groups.values(), key=len, reverse=True)


def describe(points):
    xs = [p[0] for p in points]
    ys = [p[1] for p in points]
    return (
        (min(xs) + max(xs)) // 2,
        (min(ys) + max(ys)) // 2,
        max(xs) - min(xs),
        max(ys) - min(ys),
        len(points),
    )


def report_absence(image):
    present = {name: len(matching(image, rgb)) for name, rgb in KNOWN.items()}
    seen = [f"{name}={count}" for name, count in present.items() if count]
    return "; ".join(seen) if seen else "no known map colour anywhere on this screen"


def main():
    raw = sys.argv[1:]
    tap = "--tap" in raw
    nth = None
    if "--nth" in raw:
        index = raw.index("--nth")
        nth = int(raw[index + 1])
        raw = raw[:index] + raw[index + 2:]
    args = [a for a in raw if a != "--tap"]
    if not args:
        raise SystemExit("usage: onmap.py <#RRGGBB|keep-in|keep-out> [--nth N] [--tap]")
    serial = subprocess.run(
        ["adb", "devices"], capture_output=True, text=True, check=True
    ).stdout.splitlines()
    serials = [line.split()[0] for line in serial[1:] if line.strip().endswith("device")]
    if not serials:
        raise SystemExit("no device")
    image = grab(serials[0])
    target = parse(args[0])
    points = matching(image, target)
    if not points:
        print(f"NOT ON SCREEN: {args[0]}. What is: {report_absence(image)}", file=sys.stderr)
        raise SystemExit(1)
    shapes = cluster(points)
    if len(shapes) > 1 and nth is None:
        print(
            f"{len(shapes)} separate shapes are this colour. The midpoint of all of them is "
            f"usually empty map, so a tap there would clear the selection rather than make one. "
            f"Say which with --nth:",
            file=sys.stderr,
        )
        for i, shape in enumerate(shapes, 1):
            cx, cy, w, h, n = describe(shape)
            print(f"  --nth {i}: {cx} {cy} span={w}x{h} pixels={n}", file=sys.stderr)
        raise SystemExit(3)
    chosen = shapes[(nth or 1) - 1] if shapes else points
    x, y, width, height, count = describe(chosen)
    points = chosen
    lopsided = min(width, height) < 0.85 * max(width, height)
    if lopsided:
        print(
            f"PARTLY HIDDEN: span {width}x{height} is not square, so part of this shape is off "
            f"screen or behind the controls and {x} {y} is the centre of what shows, not of the "
            f"shape. Still inside it for anything round; Fit first if you need the true centre.",
            file=sys.stderr,
        )
    if tap:
        subprocess.run(["adb", "-s", serials[0], "shell", "input", "tap", str(x), str(y)], check=True)
    print(f"{x} {y} span={width}x{height} pixels={len(points)}")


main()
