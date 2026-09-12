import re
import sys

NODE = re.compile(r'<node[^>]*>')
BOUNDS = re.compile(r'bounds="\[(\d+),(\d+)\]\[(\d+),(\d+)\]"')


def box(node):
    found = BOUNDS.search(node)
    return tuple(int(g) for g in found.groups()) if found else None


def attribute(node, name):
    found = re.search(r'%s="([^"]*)"' % name, node)
    return found.group(1) if found else ""


APP = "one.aircast.android"


# A dump taken while a file picker or another app is in front still contains
# labels - "Download" is a folder in documentsui - and reading enabled off one
# of those is a measurement of the wrong screen.
def labelled(tree, label):
    return [
        n for n in NODE.findall(tree)
        if re.search(r'text="%s"' % re.escape(label), n) and attribute(n, "package") == APP
    ]


def button_state(tree, label):
    targets = [box(n) for n in labelled(tree, label)]
    if not targets or targets[0] is None:
        return None
    target = targets[0]
    holders = [
        n for n in NODE.findall(tree)
        if (b := box(n)) and attribute(n, "clickable") == "true"
        and b[0] <= target[0] and b[1] <= target[1] and b[2] >= target[2] and b[3] >= target[3]
    ]
    if not holders:
        return None
    innermost = min(holders, key=lambda n: (lambda b: (b[2] - b[0]) * (b[3] - b[1]))(box(n)))
    return attribute(innermost, "enabled") == "true"


if __name__ == "__main__":
    tree = sys.stdin.read()
    if APP not in tree:
        print(f"REFUSED: this dump holds no {APP} nodes, so it is the wrong screen", file=sys.stderr)
        sys.exit(2)
    for label in sys.argv[1:]:
        state = button_state(tree, label)
        print(f"{label:12} " + ("NOT FOUND" if state is None else f"enabled={str(state).lower()}"))
