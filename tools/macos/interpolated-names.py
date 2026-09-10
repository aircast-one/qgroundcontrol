#!/usr/bin/env python3
"""Pin the head's hand-written path segments against the Q_PROPERTY that answers them.

Mission.swift and GeoTag.swift interpolate these names into bridge paths. A misspelling
is invisible from the consumer side: a read resolves to null and draws a default, and a
write resolves to nothing and simply does not happen, with no error in between. The core
never names either list, so nothing but this checks them.

It reads DECLARATIONS, not the running app's reflection. A Q_PROPERTY that exists here
can still be unreachable at runtime for a reason this cannot see; a name absent here is
wrong for certain.
"""
import re
import sys

ROOT = __file__.rsplit("/tools/", 1)[0]


def read(path):
    with open(f"{ROOT}/{path}") as handle:
        return handle.read()


def properties(header):
    return {
        match.group(2): match.group(0)
        for match in re.finditer(r"Q_PROPERTY\(\s*([\w:*<> ]+?)\s+(\w+)\s+([^)]*)\)", read(header))
    }


def swift_names(path, anchor):
    """The names in a Swift literal: a dictionary's keys, or an array's elements."""
    text = read(path)
    start = text.index(anchor)
    depth = 0
    for end in range(text.index("[", start), len(text)):
        depth += text[end] == "["
        depth -= text[end] == "]"
        if not depth:
            break
    literal = text[start:end + 1]
    keyed = re.findall(r'"([^"]+)"\s*:', literal)
    return keyed or re.findall(r'"([^"]+)"', literal)


def check(what, names, header, needs_write):
    declared = properties(header)
    bad = []
    for name in names:
        if name not in declared:
            bad.append(f"{what}: {name!r} is not a Q_PROPERTY in {header}")
        elif needs_write and " WRITE " not in declared[name]:
            bad.append(f"{what}: {name!r} in {header} has no WRITE, so the head's write is discarded")
    for line in bad:
        print(line, file=sys.stderr)
    return not bad


lists = swift_names("macos/Sources/ItemFactModel.swift", "static let lists")
labels = swift_names("macos/Sources/GeoTag.swift", "static let labels")

if not lists or not labels:
    print("found no names to check, which is a broken reader rather than a clean result",
          file=sys.stderr)
    sys.exit(1)

ok = check("ItemFact.lists", lists, "src/MissionManager/SimpleMissionItem.h", False)
ok &= check("GeoTagStore.labels", labels, "src/AnalyzeView/GeoTagController.h", True)

if ok:
    print(f"interpolated names pinned: {', '.join(lists + labels)}")
sys.exit(0 if ok else 1)
