import re
import sys

NODE = re.compile(r'<node[^>]*>')


def attribute(node, name):
    found = re.search(r'%s="([^"]*)"' % name, node)
    return found.group(1) if found else ""


def centre(node):
    bounds = re.search(r'bounds="\[(\d+),(\d+)\]\[(\d+),(\d+)\]"', node)
    if not bounds:
        return None
    left, top, right, bottom = (int(bounds.group(i)) for i in range(1, 5))
    return (left + right) // 2, (top + bottom) // 2


def matches(node, selectors):
    def ok(key, value):
        if key == "id":
            return attribute(node, "resource-id").endswith(value)
        if key == "class":
            return value in attribute(node, "class")
        if key == "below":
            spot = centre(node)
            return spot is not None and spot[1] >= int(value)
        if key == "above":
            spot = centre(node)
            return spot is not None and spot[1] <= int(value)
        return attribute(node, key) == value

    return all(ok(key, value) for key, value in selectors)


def find(xml, selectors, index):
    hits = [n for n in NODE.findall(xml) if matches(n, selectors) and centre(n)]
    return centre(hits[index]) if -len(hits) <= index < len(hits) else None


def main():
    selectors = [tuple(arg.split("=", 1)) for arg in sys.argv[1:] if "=" in arg]
    index = next((int(a) for a in sys.argv[1:] if a.lstrip("-").isdigit()), 0)
    spot = find(sys.stdin.read(), selectors, index)
    if spot is None:
        sys.exit(1)
    print(spot[0], spot[1])


if __name__ == "__main__":
    main()

def clickable_state(xml, label):
    """enabled= on a Compose control lives on the clickable ancestor, not the label node."""
    nodes = NODE.findall(xml)
    for index, node in enumerate(nodes):
        if 'clickable="true"' not in node:
            continue
        following = [attribute(n, "text") for n in nodes[index:index + 4]]
        named = next((t for t in following if t.strip()), "")
        if named == label:
            return attribute(node, "enabled") == "true"
    return None
