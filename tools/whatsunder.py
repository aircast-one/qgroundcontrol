import re
import sys

FLIGHT_CONTROLS = {
    "arm", "disarm", "takeoff", "land", "return", "rtl",
    "emergency stop", "start mission", "continue mission", "pause",
    "abort landing", "grab", "release",
}

NODE = re.compile(
    r'(?:text|content-desc)=(["\'])(.*?)\1[^>]*bounds="\[(\d+),(\d+)\]\[(\d+),(\d+)\]"'
)


def nodes(xml):
    return [(label, left, top, right, bottom) for _, label, left, top, right, bottom in NODE.findall(xml)]

PLANNING_SCREENS = ({"Upload", "Download"}, {"Plan items"})


def is_planning(xml):
    labels = {label.strip() for label, *_ in nodes(xml)}
    return any(markers <= labels for markers in PLANNING_SCREENS)


def labels_under(xml, x, y):
    if is_planning(xml):
        return []
    return [
        label
        for label, left, top, right, bottom in nodes(xml)
        if label.strip().lower() in FLIGHT_CONTROLS
        and int(left) <= x <= int(right)
        and int(top) <= y <= int(bottom)
    ]


def main():
    x, y = int(sys.argv[1]), int(sys.argv[2])
    hits = labels_under(sys.stdin.read(), x, y)
    if hits:
        print(hits[0])


if __name__ == "__main__":
    main()
