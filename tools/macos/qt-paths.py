#!/usr/bin/env python3
"""How much Qt a native head still needs, counted in buckets rather than one number.

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
predicate beside every number.

WHAT A PATH COSTS DEPENDS ON WHAT IT DOES, which the first version of this script missed.
On the macOS head 59 raw paths are invoke and only 18 are group -- so "118 paths left"
reads as "118 views to serve" when the majority are COMMANDS. Those need a core ACTION
endpoint, not a served view, and many of them (plan.sendToVehicle, vehicle.forceArm,
logDownload.eraseAll, vehicle.motorTest) are actuator paths no head can exercise on a
grounded rig. Counting a read and a command as one unit overstates what porting views
achieves and hides the work that has no view-shaped answer. It also shows the two heads
are at different stages rather than merely different sizes: macOS is 59 actions against 18
reads, Android 23 against 28. Hence:

  READ         group/get, or a typed helper (qgcBool, qgcDouble, qgcString, qgcPath,
               SettingsGroup). A served view retires it; this is the migration's bulk.
  ACTION       invoke. Needs a core action, and cannot be verified on a rig that must not
               move the vehicle.
  WRITE        set.
  UNCLASSIFIED the literal is not on a call line -- a multi-line call, or a path built into
               a variable first. Reported rather than folded into READ, because guessing
               here is how the 89-vs-129 gap happened in the first place.

The call vocabulary is matched by NAME, not by receiver, so one list covers `Bridge.group`
in Swift and a bare `get(` in Kotlin. Matching `Bridge.` instead reported Android as 0
reads and 0 actions with all 76 paths unclassified -- a measure that looks like an answer.

And apart from that, by shape:

  LITERAL      a complete quoted path. Mechanical to move.
  INTERPOLATED assembled at runtime -- `"plan.missionController.visualItems.\\(i)"`. These
               resolve into a live QObject exactly like a literal, so excluding them
               understates the Qt surface. But each template expands to an unknown number
               of runtime paths: THERE IS NO STATIC ANSWER TO HOW MANY, which is why they
               are counted apart rather than folded in.
  CALL SITES   occurrences rather than distinct paths: effort rather than surface.

SF SYMBOLS COLLIDE WITH THE PATH SHAPE. `"camera.fill"`, `"video.fill"` and
`"camera.metering.none"` are SwiftUI icon names, and `camera`/`video` are bridge roots, so
five icons counted as Qt debt. `"camera.fill"` and `"vehicle.armed"` are textually the
same shape -- no regex separates them, so the separator is the CONTEXT a symbol is written
in: a systemName:/systemImage: argument, or a declaration named symbol/glyphs/icon. Every
one of the five sat in one of those and no real bridge path does. The argument is looked
for on the literal's own line and the one before it, because SwiftUI wraps arguments and
`Image(systemName:` sat a line above its own literal; a symbol written three lines below
its argument would still be miscounted.

SERVED paths (`view.*`) are counted separately as the numerator of the migration: the head
is done with a root when its raw count reaches zero.

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

PATH = re.compile(r'"((?:' + "|".join(ROOTS) + r')\.[^"]*)"')

INTERPOLATION = re.compile(r'\\\(|\$\{|\$[A-Za-z_]')

DECLARATION = re.compile(r'\b(?:var|let|func|val|fun)\s+([A-Za-z_][A-Za-z0-9_]*)')
SYMBOL_DECLARATION = re.compile(r'symbol|glyph|icon', re.IGNORECASE)
SYMBOL_ARGUMENT = re.compile(r'systemName:|systemImage:')

USE = [("action", re.compile(r'\binvoke\(')),
       ("read", re.compile(r'\b(?:group|get|qgcBool|qgcDouble|qgcInt|qgcString|qgcPath'
                           r'|SettingsGroup)\(')),
       ("write", re.compile(r'\bset\('))]

SUFFIXES = (".swift", ".kt")


def paths(root):
    for source in sorted(p for p in root.rglob("*") if p.suffix in SUFFIXES):
        declaration, previous = "", ""
        for line in source.read_text(errors="replace").splitlines():
            found = DECLARATION.search(line)
            declaration = found.group(1) if found else declaration
            symbolic = (SYMBOL_ARGUMENT.search(line + previous)
                        or SYMBOL_DECLARATION.search(declaration))
            previous = line
            use = next((name for name, call in USE if call.search(line)), "unclassified")
            for match in PATH.finditer(line):
                if not symbolic:
                    yield match.group(1), use


def main():
    root = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else "macos/Sources")
    if not root.is_dir():
        print(f"not a directory: {root}", file=sys.stderr)
        return 2

    served, literal, template = set(), set(), set()
    uses, sites = {}, 0
    for path, use in paths(root):
        sites += 1
        bucket = template if INTERPOLATION.search(path) else literal
        (served if path.startswith("view.") else bucket).add(path)
        if not path.startswith("view."):
            uses.setdefault(path, set()).add(use)

    def counted(name):
        return sum(1 for kinds in uses.values() if kinds == {name})

    by_root = {}
    for path in literal | template:
        by_root.setdefault(path.split(".")[0], []).append(path)

    print(f"head: {root}")
    print(f"predicate: a quoted literal whose text is one of {len(ROOTS)} bridge roots "
          f"followed by a dot; interpolation detected as \\( or ${{ or $name; literals in "
          f"a systemName:/systemImage: argument or a symbol/glyph/icon declaration are "
          f"SwiftUI icons, not paths, and are excluded")
    print()
    print(f"  served (view.*)      {len(served):4}   distinct, the migration's numerator")
    print(f"  literal Qt paths     {len(literal):4}   distinct, mechanical to move")
    print(f"  interpolated Qt      {len(template):4}   distinct TEMPLATES, each expanding to an "
          f"unknown number of runtime paths -- needs a parameterised view, not a substitution")
    print(f"  raw Qt total         {len(literal) + len(template):4}   distinct, literal + templates")
    print(f"  call sites           {sites:4}   occurrences, not distinct: effort rather than surface")
    print()
    print(f"  reads                {counted('read'):4}   group/get/qgc* -- a served view retires these")
    print(f"  actions              {counted('action'):4}   invoke -- needs a core action, not a view, "
          f"and a grounded rig cannot exercise most of them")
    print(f"  writes               {counted('write'):4}   set")
    print(f"  unclassified         {counted('unclassified'):4}   not on a call line: a multi-line call or a "
          f"path built up first. NOT counted as reads -- guessing here is the error this script exists to avoid")
    print(f"  mixed use            {sum(1 for k in uses.values() if len(k) > 1):4}")
    print()
    for name, found in sorted(by_root.items(), key=lambda kv: (-len(kv[1]), kv[0])):
        templates = sum(1 for p in found if INTERPOLATION.search(p))
        detail = f"  ({templates} interpolated)" if templates else ""
        print(f"  {name:22} {len(found):4}{detail}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
