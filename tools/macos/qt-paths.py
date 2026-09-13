#!/usr/bin/env python3
"""How much Qt a native head still needs, counted in three buckets rather than one.

WHY THIS EXISTS. Two sessions hand-rolled this measure on 2026-09-13 and disagreed by
40 (89 against 129). Neither was lying and neither was careless: the gap was entirely in
the PREDICATE, and a bare number carries none of it. aircast-94's grep required the
remainder of a path to be `[a-zA-Z.]*`, which silently drops every interpolated path
because the character after the root's dot is a backslash. Mine had no boundary after the
root, so `"vehicleTypeString"` -- a Qt property name, not a path -- counted as a bridge
read. Both numbers looked plausible; only comparing them exposed either.

Their measured sensitivity is the argument for this file existing at all: the same tree
gives 89 with letters-and-dots, 94 allowing digits and parens, and 100 without requiring
the closing quote. A 12% spread from charset choices alone. So the output prints the
predicate beside every number, and the three buckets are reported separately because they
are different kinds of debt:

  LITERAL      a complete quoted path. Mechanical to move: the core serves it or it does
               not, and porting is a substitution.
  INTERPOLATED a path assembled at runtime -- `"plan.missionController.visualItems.\\(i)"`.
               These resolve into a live QObject exactly like a literal, so excluding them
               understates the Qt surface, which is the one thing the measure exists to
               track. But they need a PARAMETERISED view rather than a substitution, and
               each template expands to an unknown number of runtime paths. THERE IS NO
               STATIC ANSWER TO HOW MANY. That is why they are counted apart rather than
               folded in -- a single total would imply a precision that does not exist.
  CALL SITES   total occurrences rather than distinct paths. Tells you effort; the distinct
               counts tell you surface. Reported because they answer different questions.

SERVED paths (`view.*`) are counted separately as the numerator of the migration: the head
is done with a root when its raw count reaches zero, not when the served count reaches
anything in particular.

Usage: python3 tools/macos/qt-paths.py [head directory]
       default macos/Sources; pass android/app/src/main to measure the Android head.
"""
import pathlib
import re
import sys

ROOTS = [
    "view", "settings", "vehicle", "vehicles", "links", "plan", "host", "corePlugin",
    "logDownload", "video", "geoTag", "positionManager", "units", "missionCommandTree",
    "mavlinkConsole", "mavlinkInspector", "sensorsCal", "radioCal", "packetRadio",
    "joystick", "camera", "gimbal",
]

# A root must be followed by a dot, or the literal is an identifier that merely starts with
# a root word. `"vehicleTypeString"` is the case that taught me this.
PATH = re.compile(r'"((?:' + "|".join(ROOTS) + r')\.[^"]*)"')

# Swift writes `\(expr)`, Kotlin writes `$name` or `${expr}`. Either means the path is
# assembled at runtime, so the script works on both heads without being told which.
INTERPOLATION = re.compile(r'\\\(|\$\{|\$[A-Za-z_]')

SUFFIXES = (".swift", ".kt")


def paths(root):
    for source in sorted(p for p in root.rglob("*") if p.suffix in SUFFIXES):
        for found in PATH.finditer(source.read_text(errors="replace")):
            yield found.group(1)


def main():
    root = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else "macos/Sources")
    if not root.is_dir():
        print(f"not a directory: {root}", file=sys.stderr)
        return 2

    served, literal, template = set(), set(), set()
    sites = 0
    for path in paths(root):
        sites += 1
        bucket = template if INTERPOLATION.search(path) else literal
        (served if path.startswith("view.") else bucket).add(path)

    by_root = {}
    for path in literal | template:
        by_root.setdefault(path.split(".")[0], []).append(path)

    print(f"head: {root}")
    print(f"predicate: a quoted literal whose text is one of {len(ROOTS)} bridge roots "
          f"followed by a dot; interpolation detected as \\( or ${{ or $name")
    print()
    print(f"  served (view.*)      {len(served):4}   distinct, the migration's numerator")
    print(f"  literal Qt paths     {len(literal):4}   distinct, mechanical to move")
    print(f"  interpolated Qt      {len(template):4}   distinct TEMPLATES, each expanding to an "
          f"unknown number of runtime paths -- needs a parameterised view, not a substitution")
    print(f"  raw Qt total         {len(literal) + len(template):4}   distinct, literal + templates")
    print(f"  call sites           {sites:4}   occurrences, not distinct: effort rather than surface")
    print()
    for name, found in sorted(by_root.items(), key=lambda kv: (-len(kv[1]), kv[0])):
        templates = sum(1 for p in found if INTERPOLATION.search(p))
        detail = f"  ({templates} interpolated)" if templates else ""
        print(f"  {name:22} {len(found):4}{detail}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
