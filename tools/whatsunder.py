import re
import sys

FLIGHT_CONTROLS = {
    "arm", "disarm", "takeoff", "land", "return", "rtl",
    "emergency stop", "start mission", "continue mission", "pause",
    "abort landing", "grab", "release",
}

NODE = re.compile(r'(?:text|content-desc)="([^"]*)"[^>]*bounds="\[(\d+),(\d+)\]\[(\d+),(\d+)\]"')

PLANNING_MARKERS = {"Upload", "Download"}


def is_planning(xml):
    labels = {label.strip() for label, *_ in NODE.findall(xml)}
    return PLANNING_MARKERS <= labels


def labels_under(xml, x, y):
    if is_planning(xml):
        return []
    return [
        label
        for label, left, top, right, bottom in NODE.findall(xml)
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
