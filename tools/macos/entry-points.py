#!/usr/bin/env python3
"""Declarations in macos/Sources that nothing in this head ever enters.

The weaker question is "does this name appear anywhere", and it is wrong in one
direction: a member asserted through its consumer looks unread, and a read that
sits inside a function nobody calls looks live. This asks the second half --
whether the function HOLDING the work is ever entered.

The blind spot runs both ways, which is what ALLOWED is for. A protocol witness
is entered by the framework through the protocol, never by a caller naming it,
so "no caller" says nothing about whether it runs. Every name below is on the
list because a mechanism outside this head enters it, and the mechanism is named
-- not because the sweep was noisy.
"""

import os
import re
import sys
from collections import defaultdict

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
SOURCES = os.path.join(ROOT, "macos", "Sources")
TESTS = os.path.join(ROOT, "macos", "Tests", "main.swift")

ALLOWED = {
    "makeNSView": "NSViewRepresentable requirement; SwiftUI calls it when it realises the view.",
    "updateNSView": "NSViewRepresentable requirement; SwiftUI calls it on every state change.",
    "dismantleNSView": "NSViewRepresentable requirement, and static -- SwiftUI calls it on teardown.",
    "makeCoordinator": "NSViewRepresentable requirement; SwiftUI calls it before makeNSView.",
    "controlTextDidChange": "NSTextFieldDelegate; AppKit calls it on every keystroke in the field.",
    "windowDidBecomeKey": "NSWindowDelegate; AppKit calls it when the window takes focus.",
    "windowWillClose": "NSWindowDelegate; AppKit calls it as the window closes.",
    "gestureRecognizer": "NSGestureRecognizerDelegate; AppKit calls it to arbitrate a gesture.",
    "qgcMacosMain": "@_cdecl entry point; the C++ side calls it by symbol, so no Swift caller exists.",
    "body": "View requirement; SwiftUI calls it to render.",
    "probeState": "Probeable requirement; the debug API calls it through the protocol.",
    "probeInvoke": "Probeable requirement; the debug API calls it through the protocol.",
}

DECL = re.compile(r'\b(?:private\s+|fileprivate\s+|static\s+|@ViewBuilder\s+)*func\s+([A-Za-z_]\w*)\s*[(<]')
COMPUTED = re.compile(r'\bvar\s+([A-Za-z_]\w*)\s*:\s*[^=\n]+\{\s*$', re.M)


def read(path):
    with open(path, encoding="utf-8") as handle:
        return handle.read()


def main():
    if not os.path.isdir(SOURCES):
        print(f"cannot read {SOURCES}: this script must run from inside the repository", file=sys.stderr)
        return 2
    files = sorted(
        os.path.join(SOURCES, name) for name in os.listdir(SOURCES) if name.endswith(".swift")
    )
    if not files:
        print(f"{SOURCES} holds no .swift files: measured nothing rather than found nothing", file=sys.stderr)
        return 2
    if not os.path.isfile(TESTS):
        print(f"cannot read {TESTS}: a declaration reached only by an assertion would read as dead", file=sys.stderr)
        return 2

    bodies = {path: read(path) for path in files}
    every = "\n".join(bodies.values())
    asserted = read(TESTS)

    declared = defaultdict(set)
    for path, body in bodies.items():
        for match in list(DECL.finditer(body)) + list(COMPUTED.finditer(body)):
            declared[match.group(1)].add(os.path.basename(path))

    def mentions(name, text):
        bare = len(re.findall(r'(?<![A-Za-z0-9_.])' + re.escape(name) + r'(?![A-Za-z0-9_])', text))
        dotted = len(re.findall(r'\.' + re.escape(name) + r'(?![A-Za-z0-9_])', text))
        return bare + dotted

    unentered = {
        name: places for name, places in declared.items()
        if name not in ALLOWED
        and mentions(name, every) <= len(places)
        and mentions(name, asserted) == 0
    }

    # An allowlist entry outlives what it excuses. A name that is no longer declared
    # here is a line nobody has read since the code went, and one that has grown a
    # caller is being excused for a reason that stopped applying.
    stale = [f"{name!r} is allowed for a mechanism that enters it, but nothing in macos/Sources declares it any more"
             for name in sorted(ALLOWED) if name not in declared]

    for name in sorted(unentered):
        print(f"  NOT ENTERED {name} <- {', '.join(sorted(unentered[name]))}")
    for line in stale:
        print(f"  STALE ALLOWANCE {line}")

    # What this cannot see, stated rather than implied:
    # - a name reached by #selector, key-value coding or any string-built dispatch
    # - a declaration whose name is shared with a member of another type, whose
    #   mentions inflate the count and make a dead one look entered (FALSE NEGATIVE)
    # - a function entered only from C++ or QML, which is the ALLOWED case and is why
    #   a new hit is a question rather than a verdict
    print(f"checked {len(declared)} declared names across {len(files)} files in macos/Sources against every "
          f"mention in Sources and Tests: {len(unentered)} are entered by nothing this sweep can see, "
          f"{len(ALLOWED)} allowed because a framework or the C entry point enters them, {len(stale)} stale")
    return 1 if unentered or stale else 0


if __name__ == "__main__":
    sys.exit(main())
