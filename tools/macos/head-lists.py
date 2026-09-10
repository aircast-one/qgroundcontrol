#!/usr/bin/env python3
"""Pin a hand-written list in the head against the code it has to agree with.

A list someone maintains by hand rots the moment the thing it mirrors changes, and nothing
says so. These are the ones where drift is silent rather than loud.

SetupPage.bespoke names every page VehicleSetupWindow's content switch has a case for. A name
in the switch but not the list is a page that is built and never offered; a name in the list
but not the switch is a page that is offered and opens on nothing.
"""
import re
import sys

ROOT = __file__.rsplit("/tools/", 1)[0]


def read(path):
    with open(f"{ROOT}/{path}") as handle:
        return handle.read()


def switch_cases(path, opening, closing):
    """The case labels of one switch, from its opening line to its default.

    Assumes the switch has no nested switch with its own default before it; one that did would
    truncate the scan. The failure is loud - names go missing and this exits non-zero.
    """
    text = read(path)
    start = text.index(opening)
    end = text.index(closing, start)
    return set(re.findall(r'case "([^"]+)"', text[start:end]))


def swift_set(path, anchor):
    text = read(path)
    start = text.index(anchor)
    end = text.index("]", text.index("[", start))
    return set(re.findall(r'"([^"]+)"', text[start:end]))


declared = swift_set("macos/Sources/SetupPageModel.swift", "static let bespoke")
built = switch_cases("macos/Sources/VehicleSetupWindow.swift",
                     "private var content: some View {", "default:")

if not declared or not built:
    print("found no names on one side, which is a broken reader rather than a clean result",
          file=sys.stderr)
    sys.exit(1)

offered_but_blank = sorted(declared - built)
built_but_hidden = sorted(built - declared)
for name in offered_but_blank:
    print(f"SetupPage.bespoke lists {name!r}, which the content switch has no case for: "
          f"choosing it opens on nothing", file=sys.stderr)
for name in built_but_hidden:
    print(f"the content switch draws {name!r}, which SetupPage.bespoke omits: it is built and "
          f"never offered", file=sys.stderr)

if offered_but_blank or built_but_hidden:
    sys.exit(1)
print(f"head lists pinned: SetupPage.bespoke matches {len(built)} content switch cases")
