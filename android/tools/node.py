import re
import sys

NODE = re.compile(r'<node[^>]*>')


# uiautomator quotes an attribute with ' when its value contains a ", so a
# pattern that only accepts " silently cannot see any label holding a quote.
def attribute(node, name):
    found = re.search(r"""%s=(?:"([^"]*)"|'([^']*)')""" % name, node)
    if not found:
        return ""
    return found.group(1) if found.group(1) is not None else found.group(2)


def centre(node):
    bounds = re.search(r'bounds="\[(-?\d+),(-?\d+)\]\[(-?\d+),(-?\d+)\]"', node)
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


def count(xml, selectors):
    return len([n for n in NODE.findall(xml) if matches(n, selectors) and centre(n)])


def main():
    selectors = [tuple(arg.split("=", 1)) for arg in sys.argv[1:] if "=" in arg]
    index = next((int(a) for a in sys.argv[1:] if a.lstrip("-").isdigit()), 0)
    if not selectors:
        named = " ".join(a for a in sys.argv[1:] if not a.lstrip("-").isdigit())
        print(f"no selector in {named!r} - a selector is key=value (text=Settings, id=..., "
              f"class=...). Without one every node matches and this returns the first, which is "
              f"a tap on whatever happens to be at the top of the tree.", file=sys.stderr)
        sys.exit(2)
    xml = sys.stdin.read()
    spot = find(xml, selectors, index)
    if spot is None:
        sys.exit(1)
    seen = count(xml, selectors)
    if seen > 1 and not any(a.lstrip("-").isdigit() for a in sys.argv[1:]):
        print(f"{seen} nodes match {' '.join(f'{k}={v}' for k, v in selectors)}; using 0", file=sys.stderr)
    print(spot[0], spot[1])


def clickable_state(xml, label):
    nodes = NODE.findall(xml)
    for index, node in enumerate(nodes):
        if 'clickable="true"' not in node:
            continue
        following = [attribute(n, "text") for n in nodes[index:index + 4]]
        named = next((t for t in following if t.strip()), "")
        if named == label:
            return attribute(node, "enabled") == "true"
    return None


def selftest():
    off = '<node text="Next" bounds="[36,-50][444,58]"/>'
    on = '<node text="Cancel" bounds="[480,1620][888,1748]"/>'
    assert find(off + on, [("text", "Next")], 0) == (240, 4), "a node laid out off the top of the screen still has a centre"
    assert find(off + on, [("text", "Cancel")], 0) == (684, 1684)
    assert find(on, [("text", "Missing")], 0) is None
    two = '<node text="Sensors" bounds="[0,600][100,660]"/><node text="Sensors" bounds="[0,860][100,920]"/>'
    assert count(two, [("text", "Sensors")]) == 2, "a label that appears twice is ambiguous, not a single answer"
    assert find(two, [("text", "Sensors")], 1) == (50, 890)
    assert clickable_state('<node clickable="true" enabled="false" text=""/><node text="Erase" enabled="true"/>', "Erase") is False
    quoted = '<node text=\'Says "yes" here\' resource-id="" class="x" bounds="[0,0][10,10]" />'
    assert attribute(quoted, "text") == 'Says "yes" here', attribute(quoted, "text")
    assert find(quoted, [("text", 'Says "yes" here')], 0) == (5, 5)
    print("node.py ok")


if __name__ == "__main__":
    if "--selftest" in sys.argv:
        selftest()
    else:
        main()
