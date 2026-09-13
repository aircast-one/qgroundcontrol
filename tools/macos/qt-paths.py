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
On the macOS head 59 raw paths are invoke and only 24 are read -- so "118 paths left"
reads as "118 views to serve" when the majority are COMMANDS. Those need a core ACTION
endpoint, not a served view, and many of them (plan.sendToVehicle, vehicle.forceArm,
logDownload.eraseAll, vehicle.motorTest) are actuator paths no head can exercise on a
grounded rig. Counting a read and a command as one unit overstates what porting views
achieves and hides the work that has no view-shaped answer. It also shows the two heads
are at different stages rather than merely different sizes: macOS is 59 actions against 24
reads, Android 38 against 51. Hence:

  READ         group/get, or a typed helper (qgcBool, qgcDouble, qgcString, qgcPath,
               SettingsGroup). A served view retires it; this is the migration's bulk.
  ACTION       invoke. Needs a core action, and cannot be verified on a rig that must not
               move the vehicle.
  WRITE        set, or a head helper that wraps it. THIS BUCKET HAS NO RETIREMENT PATH: a
               served view is REFUSED on write by router.set, and an actions::owns entry is
               consulted by invoke only. A write is not a read that goes the other way; it needs
               a mechanism that does not exist. Counting it as noise hid a third of one head's
               surface until the vocabulary was widened on 2026-09-14.
  UNCLASSIFIED the literal is not on a call line -- a multi-line call, or a path built into
               a variable first. Reported rather than folded into READ, because guessing
               here is how the 89-vs-129 gap happened in the first place.

The call vocabulary is matched by NAME, not by receiver, so one list covers `Bridge.group`
in Swift and a bare `get(` in Kotlin. Matching `Bridge.` instead reported Android as 0
reads and 0 actions with every path unclassified -- a measure that looks like an answer.

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

A PATH IS NOT ALWAYS WRITTEN AS A LITERAL. Kotlin assembles most of its writes from a
constant -- `Qgc.set("$RADIO_CAL.transmitterMode", mode)` with `const val RADIO_CAL =
"radioCal"` -- so the literal begins with `$RADIO_CAL`, fails the root test, and lands
outside the measured surface rather than in UNCLASSIFIED. Sixteen write call sites were
invisible and the head reported 0 writes, which reads as "nothing to migrate" instead of
"not measured". The constant's value is in the source, so this is resolved exactly and not
guessed. A path a FUNCTION builds from an argument still cannot be resolved and stays
unclassified, which is the honest answer for it.

THE USE BUCKETS COUNT PATHS TOUCHED, NOT PATHS USED EXCLUSIVELY, and so they overlap and
sum past the raw total. Counting exclusive use hid the thing the split was added to show:
Android has 15 paths that are both read and written, and its write count still read as 3
with every constant resolved.

A RAW COUNT CONFLATES THREE STATES AND CANNOT SEPARATE THEM. Auditing the macOS head's five
distinct `vehicle.*` group reads on 2026-09-14 found all three present at once:

  NO VIEW EXISTS              `vehicle.gps` and `vehicle.terrain`. There is no view.gps
                              (view.gpsRtkBase is the base station) and no view carrying the
                              terrain tile download queue (view.terrainTile is a file lookup).
                              These are core asks.
  A VIEW EXISTS AND IS IGNORED  the state everyone assumes when they read the number.
  A VIEW EXISTS AND DOES NOT CARRY WHAT THIS READER NEEDS
                              `vehicle.batteries` at Fly.swift:106 builds six detail rows and
                              view.battery's packs carries two of them; `vehicle.cameraManager`
                              needs exactly one more field (cameraLabels). Migrating the first
                              on the strength of the path count alone would have dropped four
                              rows with nothing going red.

So a bare raw total points effort at the wrong session in both directions: it reads as core
work when the answer is a head edit, and as a head edit when the answer is four more fields.
Quote it with that caveat, and audit a path before planning around it.

AND SOME READERS ARE PERMANENT, NOT UNMIGRATED. Instruments.swift enumerates `vehicle.children`
and reads each group; Parameters.swift enumerates every parameter name and reads each Fact.
Both are BROWSERS over the live object graph, and reflection over paths nobody enumerated in
advance is the one thing a fixed view cannot replace. They will still be here when the port is
done, and counting them as remaining work makes a finished migration look stalled.

THE RAW COUNT RISES DURING PARITY WORK AND THAT IS NOT REGRESSION. Feature parity with
QGroundControl became required scope on 2026-09-14, and every feature built against a view
that covers only its READ side adds a raw Qt path for its writes. 96c8e8282 is the worked
example: the packet radio adapter picker had to write
`settings.packetRadioSettings.deviceName` because view.packetRadio reports radio state and
has nothing for choosing an adapter, so macOS went 118 -> 119 on a commit that closed a
parity gap. The count measures REMAINING QT DEPENDENCE, which is the right question at the
end and an actively misleading one in the middle. Read the direction against what landed,
never on its own.

Each such addition is also a core ask rather than an accident: a head writing a Qt setting
directly is how two heads come to disagree about validation, which is the same argument as
not converting units head-side. Batch them as they appear.

A LITERAL CAN RETIRE WHILE AN INTERPOLATED TEMPLATE STILL REACHES THE SAME OBJECT, and that
is the one way this number moves in the FLATTERING direction without the dependence changing.
Instruments.swift reads `"vehicle.batteries"` at :84 for a pack count and `"vehicle.\\(group)"`
at :91 for each pack's facts -- seven lines apart, same function, same Qt object. Sourcing the
count from `view.battery.packs` would delete the literal and drop the total by one while :91
went on reading vehicle.batteries.0 exactly as before. The count would report progress that
did not happen.

So a proposed migration has to be checked against what the head still REACHES, not against
what the head still SPELLS. Refused once on 2026-09-14 for this reason, which is the mirror of
the inflated-count problem in the header: an undercount is a number nobody re-derives, and a
deflated one is a number nobody questions at all.

SERVED paths (`view.*`) are counted separately as the numerator of the migration: the head
is done with a root when its raw count reaches zero.

A HEAD IS NOT ALWAYS ONE DIRECTORY, so this takes a list of them. Android is two modules
-- app/src/main plus map-spike/src/main, which app/build.gradle.kts:43 depends on and which
draws the Plan tab -- and measuring only the first left a third of the head out. Do not
reach for the parent directory instead: android/ sweeps app/src/test, whose paths are test
fixtures and not head debt at all. Name the source roots.

AND TWO DISTINCT-COUNTS CANNOT BE ADDED, which is why the list matters more than running it
twice. The two Android modules measured alone gave 76 and 13; their union is 84, because
five paths appear in both. Four of those also CHANGE CLASS on being merged -- a path read in
one module and invoked in the other is one path used two ways, and that is only visible when
they are counted together.

Usage: python3 tools/macos/qt-paths.py [head directory ...]
       default macos/Sources
       Android: android/app/src/main android/map-spike/src/main
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

USE = [("read", re.compile(r'\b(?:group|get|getFields|watch|invokeResult|qgcBool|qgcDouble'
                           r'|qgcInt|qgcString|qgcPath|SettingsGroup)\(')),
       ("action", re.compile(r'\binvoke\(')),
       ("write", re.compile(r'\b(?:set|write)\('))]

# Kotlin `const val NAME = "..."` and Swift `static let name = "..."`. Only values that already
# match PATH are kept, which is what makes this safe: `static let probeID = "geoTag"` is a bare
# root with no dot, so it never matched PATH and is never expanded. I refused this fix once on
# the grounds that expanding constants would manufacture paths -- the discriminator was there
# the whole time and I had not looked for it.
CONSTANT = re.compile(r'\bconst\s+val\s+([A-Z][A-Z0-9_]*)\s*=\s*"([^"]*)"')

# Swift binds paths to constants too -- `static let deviceNamePath = "settings.…deviceName"`,
# read and written only through the name. Unlike the Kotlin form the declaration line is NOT
# skipped: `let path = "plan.…\(index)"` is itself a path, and skipping it lost two real
# templates when I tried. Names must look like constants, because a bare `path` variable is
# common enough that expanding it would rewrite half the file.
SWIFT_CONSTANT = re.compile(r'\b(?:static\s+)?let\s+([A-Za-z_][A-Za-z0-9_]*[Pp]ath)\s*=\s*"([^"]*)"')

APP_STORAGE = re.compile(r'@AppStorage\(')

SUFFIXES = (".swift", ".kt")


def constants(sources):
    found = {}
    for source in sources:
        text = source.read_text(errors="replace")
        for pattern in (CONSTANT, SWIFT_CONSTANT):
            for match in pattern.finditer(text):
                found[match.group(1)] = match.group(2)
    whole = {n: v for n, v in found.items() if PATH.fullmatch(f'"{v}"')}
    root = {n: v for n, v in found.items() if v in ROOTS}
    return whole | root, whole


def expand(line, named):
    for name, value in named.items():
        line = line.replace("${" + name + "}", value).replace("$" + name, value)
    return line


def paths(roots):
    sources = sorted(p for root in roots for p in root.rglob("*") if p.suffix in SUFFIXES)
    named, whole = constants(sources)
    rooted = {n: v for n, v in named.items() if n not in whole}
    bare = re.compile(r'\b(' + "|".join(whole) + r')\b') if whole else None
    root_bare = re.compile(r'\b(' + "|".join(rooted) + r')\b') if rooted else None
    for source in sources:
        declaration, previous = "", ""
        for line in source.read_text(errors="replace").splitlines():
            found = DECLARATION.search(line)
            declaration = found.group(1) if found else declaration
            symbolic = (SYMBOL_ARGUMENT.search(line + previous)
                        or SYMBOL_DECLARATION.search(declaration))
            previous = line
            if CONSTANT.search(line) or APP_STORAGE.search(line):
                continue
            line = expand(line, named)
            use = next((name for name, call in USE if call.search(line)), "unclassified")
            if symbolic:
                continue
            for match in PATH.finditer(line):
                yield match.group(1), use
            for match in bare.finditer(line) if bare else ():
                yield whole[match.group(1)], use
            for match in root_bare.finditer(line) if root_bare and use != "unclassified" else ():
                yield rooted[match.group(1)], use


def main():
    roots = [pathlib.Path(a) for a in (sys.argv[1:] or ["macos/Sources"])]
    missing = [r for r in roots if not r.is_dir()]
    if missing:
        print("not a directory: " + ", ".join(map(str, missing)), file=sys.stderr)
        return 2

    served, literal, template = set(), set(), set()
    uses, sites = {}, 0
    for path, use in paths(roots):
        sites += 1
        bucket = template if INTERPOLATION.search(path) else literal
        (served if path.startswith("view.") else bucket).add(path)
        if not path.startswith("view."):
            uses.setdefault(path, set()).add(use)

    def counted(name):
        return sum(1 for kinds in uses.values() if name in kinds)

    by_root = {}
    for path in literal | template:
        by_root.setdefault(path.split(".")[0], []).append(path)

    print("head: " + ", ".join(map(str, roots)))
    print(f"predicate: a quoted literal whose text is one of {len(ROOTS)} bridge roots "
          f"followed by a dot; interpolation detected as \\( or ${{ or $name; literals in "
          f"a systemName:/systemImage: argument or a symbol/glyph/icon declaration are "
          f"SwiftUI icons, not paths, and are excluded; a leading $NAME or a bare NAME "
          f"argument is expanded when a const val NAME names a root path")
    print()
    print(f"  served (view.*)      {len(served):4}   distinct, the migration's numerator")
    print(f"  literal Qt paths     {len(literal):4}   distinct, mechanical to move")
    print(f"  interpolated Qt      {len(template):4}   distinct TEMPLATES, each expanding to an "
          f"unknown number of runtime paths -- needs a parameterised view, not a substitution")
    print(f"  raw Qt total         {len(literal) + len(template):4}   distinct, literal + templates")
    print(f"  call sites           {sites:4}   occurrences, not distinct: effort rather than surface")
    print()
    print(f"  reads                {counted('read'):4}   group/get/watch/qgc* -- a served view retires these")
    print(f"  actions              {counted('action'):4}   invoke -- needs a core action, not a view, "
          f"and a grounded rig cannot exercise most of them")
    print(f"  writes               {counted('write'):4}   set/write -- NOTHING RETIRES THESE YET: "
          f"router.set refuses view paths and otherwise passes straight to Qt, and actions::owns "
          f"is only consulted by invoke")
    print(f"  unclassified         {counted('unclassified'):4}   not on a call line: a multi-line call or a "
          f"path built up first. NOT counted as reads -- guessing here is the error this script exists to avoid")
    print(f"  used more than one way {sum(1 for k in uses.values() if len(k) > 1):3}   a path both read and "
          f"written is counted under EACH use above, so those four exceed the raw total")
    print()
    for name, found in sorted(by_root.items(), key=lambda kv: (-len(kv[1]), kv[0])):
        templates = sum(1 for p in found if INTERPOLATION.search(p))
        detail = f"  ({templates} interpolated)" if templates else ""
        print(f"  {name:22} {len(found):4}{detail}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
