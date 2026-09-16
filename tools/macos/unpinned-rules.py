#!/usr/bin/env python3
"""A conditional that picks a SENTENCE or a SEVERITY lives where nothing compiles it.

swift-checks compiles 82 of this head's 127 Swift files. A rule written in one of the other
45 -- a plural, an empty-state sentence, a severity colour, a conditional button label -- has
no assertion that can reach it, so changing it is free and getting it wrong is silent. That
claim was made here a dozen times and never counted. It is 65.

The predicate has no judgement in it: a REAL ternary (`?` that is not `??`) with a string
literal branch, in a file swift-checks does not compile. What it cannot know is what the
string is FOR, and that is the whole triage:

  - a sentence, plural, severity or label an operator reads -- move it to a compiled file
  - a WIRE TOKEN or probe id, where the ternary picks a path or an argument rather than
    something a person sees -- legitimate (a field can be served for an instrument), ACCEPT
  - pure layout, a disclosure chevron -- ACCEPT

So ACCEPTED is keyed on the file and the first literal of the pair, which survives lines
moving, and every entry carries why. An entry that stops matching is reported: an acceptance
decays exactly like an assertion.

This does NOT floor the finding count. A floor on a number that should fall is an instrument
that weakens as what it measures improves. It floors NOTHING, in the end: both numbers
it was tempting to floor -- the hit count and the uncompiled file list -- fall as the work lands,
so a floor under either refuses on success. Both guards are controls against known samples
instead, which is a property of the instrument and cannot decay.

A SECOND predicate, added after the first went standing and I noticed what it could not see:
a conditional picking a SEVERITY COLOUR with no string literal on the line is invisible to a
string sweep, and that is not a small corner -- a red where an orange belongs says the wrong
thing about a vehicle just as loudly as a wrong sentence does.

It is deliberately narrow. Only .red/.orange/.green/.yellow count, because those four ARE the
severity vocabulary here; .secondary/.primary/.accentColor are EMPHASIS, and a line mixing
green with accentColor is a progress indicator rather than a ladder. A first attempt matched
an Overlay-dot-anything branch too and reported `CachedTileOverlay.missed += 1` as a colour -- 33 hits, of
which that one was visible only by reading the list rather than the count.

Prints one summary line so it can stand in the gate without burying it, and FAILS only on a
stale acceptance. The findings themselves are triage, not a gate: pass --list to read them.

Usage: python3 tools/macos/unpinned-rules.py [sources-dir] [--list]
"""
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent.parent

ACCEPTED = {
    ("Links.swift", "localPort"): "a BRIDGE PATH, not a sentence: UDP spells the field localPort "
        "and every other type spells it port, so the ternary picks which property to write. An "
        "operator never sees either word",
    ("Video.swift", "photo"): "the camera's WIRE TOKEN for setMode, handed straight to the core. "
        "Moving it to a compiled file would pin a spelling QGC owns, not a rule this head has",
    ("Mission.swift", "exact"): "the tile-source token this head's OWN probe exports so a rig can "
        "tell a served tile from one rebuilt out of a parent or children. An instrument, not a "
        "surface -- nothing on screen reads these three words",
    ("MissionMap.swift", "midpoint"): "a probe id naming which handle was read, for the same "
        "reason. The operator-facing half of that file IS pinned: MissionMap.title",
    ("Mission.swift", ""): "a view ARGUMENT -- view.polygon takes a trailing ,line for a polyline "
        "and nothing for a ring. The empty branch is the absence of an argument, not an absent "
        "sentence",
    ("FlyWindow.swift", "chevron.up"): "a disclosure chevron follows the disclosure state and says "
        "nothing about the vehicle. Pure layout may stay",
    ("PlanWindow.swift", "true"): "the two branches are the strings \"true\" and \"false\" going "
        "INTO setFact, which is how the bridge spells a boolean. A wire value",
    ("Mission.swift", "the plan is syncing"): "a PROBE REFUSAL, from probeInvoke's [\"ok\": false, "
        "\"error\": ...] -- three of them, and they are my own instrument telling me why it "
        "declined, never a sentence an operator sees. A field served for an instrument is "
        "legitimate, and so is a string returned to one. MEASURED: the mission probe's error key "
        "is read by my rig and by nothing on screen",
    ("VehicleSetupWindow.swift", ".green"): "MEASURED AND SPLIT: :167 pairs green with "
        "accentColor, which is PROGRESS rather than severity -- done/in-progress/not-yet, the same "
        "vocabulary as the current flight mode's accent. :76 and :484 key on store.failing.isEmpty "
        "and channelCount, both pinned in compiled models, so only the palette lookup floats and "
        "the rule underneath is asserted",
    ("AnalyzeWindow.swift", ".red"): "the CONDITIONS are pinned: :241 on GeoTagJob.failed, :67 on "
        "VibrationReading's band. .red is this head's ERROR palette -- the connection form and the "
        "vibration danger band spend it -- and is a different subject from FlyPanel.colour's "
        "VEHICLE-severity ladder, which has no red in it at all. Routing it there would be the "
        "wrong-accessor mistake",
    ("AnalyzeWindow.swift", ".green"): "the clip row, whose condition VibrationReading.clipHealthy "
        "is compiled and asserted at its OWN threshold -- zero, not one, separately from the "
        "plural beside it. Only the two-colour lookup floats",
    ("OverlayKit.swift", ".green"): "`good` comes from VehicleComponentInfo.severity, which is "
        "compiled and asserted; these two lines are the same weight drawn twice in one row",
    ("PacketRadioSection.swift", ".red"): "an ERROR line, not a ladder: primary or red on "
        "whether startError is empty, which is the .red palette again",
    ("ParameterRow.swift", ".orange"): "a non-default DOT, drawn or not drawn -- Color.clear "
        "is an absence rather than a second severity, so there is no ordering to get wrong",
    ("VehicleSetupWindow.swift", ".orange"): ":245 is the motors safety toggle, whose condition "
        "safetyOff is served; orange-or-secondary is emphasis on one served flag",
    ("FlyWindow.swift", ".orange"): ":488 keys on check.warns, a served preflight flag",
    ("FenceRally.swift", "addInclusionCircle"): "the controller METHOD NAME being invoked. Pinning "
        "it in a compiled file would assert a Qt method spelling this head does not own",
}

# Not a floor either, and for the same reason the hit count is not one. The uncompiled list
# SHRINKS as this port succeeds -- every file swift-checks learns to compile leaves it -- so a
# number under it would refuse on the migration working. The first version floored it at 40 and
# its own refusal message said to RAISE the floor when swift-checks compiles more, which is
# backwards: compiling more makes this smaller. What the guard is for is that the parse of
# swift-checks' file list worked at all, and that is checked against files whose side is known.
COMPILED_CONTROL = ("MeasureModel.swift", "SettingsPages.swift")
UNCOMPILED_CONTROL = ("PlanWindow.swift", "FlyWindow.swift")

# NOT a floor on the hit count. The first version of this floored it at 50 and refused at 49,
# because the number falls as the work lands -- flooring it asserts "the corpus still has enough
# defects in it", which is an instrument that blocks precisely when it is succeeding. What the
# guard is actually for is "the pattern still matches", and that is a property of the regex, not
# of the tree, so it is controlled against a sample here instead. This cannot decay: it holds
# whether the corpus has fifty hits or none.
CONTROL = '    Text(value.isEmpty ? "Not reported" : value)'
CONTROL_MISSES = '    Text(value ?? "Not reported")'

TERNARY = re.compile(r'(?<![?\w.])\?(?!\?)\s*("(?:[^"\\]|\\.)*")')
SEVERITY = re.compile(r'\.(red|orange|green|yellow)\b')
ANY_TERNARY = re.compile(r'(?<![?\w.])\?(?!\?)')

# The colour predicate needs its own controls: one line it must call a severity ladder, and the
# line that fooled the first version, which it must not.
COLOUR_CONTROL = '    .foregroundColor(store.failing.isEmpty ? .green : .orange)'
COLOUR_MISSES = '        data == nil ? (CachedTileOverlay.missed += 1) : (Overlay.fromChildren += 1)'



# Only what swift-checks NAMES in its swiftc invocation counts as compiled. Treating every
# *Model*.swift as compiled was the first version, and it was worse than redundant: all 73 of them
# are named anyway, so it changed no outcome, while quietly asserting that a Model file is
# compiled BECAUSE OF ITS NAME. A new one nobody adds to that invocation is not compiled, and the
# glob would have classed it compiled and skipped its rules in silence. The naming convention is
# now checked rather than trusted -- see unnamed_models.
def compiled_files(checks: str) -> set[str]:
    return set(re.findall(r'macos/Sources/([A-Za-z0-9_]+\.swift)', checks))


def severity_line(text: str) -> bool:
    return ('"' not in text and not text.startswith('//')
            and bool(ANY_TERNARY.search(text)) and bool(SEVERITY.search(text)))


def main() -> int:
    given = [arg for arg in sys.argv[1:] if not arg.startswith('--')]
    src = pathlib.Path(given[0]) if given else ROOT / 'macos/Sources'
    checks = (ROOT / 'tools/macos/swift-checks.sh').read_text()
    every = sorted(path.name for path in src.glob('*.swift'))
    if not every:
        print(f"{src} holds no .swift file, so this has measured nothing rather than found "
              "nothing", file=sys.stderr)
        return 2
    compiled = compiled_files(checks)
    unnamed_models = [name for name in every if 'Model' in name and name not in compiled]
    if unnamed_models:
        print("swift-checks does not compile " + ", ".join(unnamed_models)
              + ", so a rule in one is unpinned while its NAME says otherwise. Add it to the "
              "swiftc invocation, or take the word Model out of the file name", file=sys.stderr)
        return 1
    missorted = ([name for name in COMPILED_CONTROL if name not in compiled]
                 + [name for name in UNCOMPILED_CONTROL if name in compiled])
    if missorted:
        print(f"the compiled/uncompiled split put {', '.join(missorted)} on the wrong side, so "
              "the parse of swift-checks' file list is broken and every finding below is about "
              "the wrong set of files", file=sys.stderr)
        return 2
    uncompiled = [name for name in every if name not in compiled]

    found = [(name, number, line.strip(), match.group(1).strip('"'))
             for name in uncompiled
             for number, line in enumerate((src / name).read_text().splitlines(), 1)
             if not line.strip().startswith('//')
             for match in [TERNARY.search(line)] if match]
    if not TERNARY.search(CONTROL) or TERNARY.search(CONTROL_MISSES):
        print("the pattern no longer separates a real ternary from a nil-coalesce, so a zero "
              "here would read as clean rather than as a broken regex", file=sys.stderr)
        return 2
    if not severity_line(COLOUR_CONTROL.strip()) or severity_line(COLOUR_MISSES.strip()):
        print("the colour pattern no longer separates a severity ladder from an arithmetic "
              "ternary, so a zero there would read as clean rather than as a broken regex",
              file=sys.stderr)
        return 2

    colours = [(name, number, line.strip(), SEVERITY.search(line).group(0))
               for name in uncompiled
               for number, line in enumerate((src / name).read_text().splitlines(), 1)
               if severity_line(line.strip())]
    unexplained = [hit for hit in found if (hit[0], hit[3]) not in ACCEPTED]
    unexplained += [hit for hit in colours if (hit[0], hit[3]) not in ACCEPTED]
    matched = {(name, literal) for name, _, _, literal in found + colours}
    stale = [key for key in sorted(ACCEPTED) if key not in matched]

    if '--list' in sys.argv:
        for name, number, text, _ in unexplained:
            print(f"{name}:{number}: {text[:110]}")
    for name, literal in stale:
        print(f"ACCEPTED {name} {literal!r} matches nothing now -- an acceptance decays like an "
              "assertion; re-derive it or delete it", file=sys.stderr)
    print(f"{len(found)} conditional string and {len(colours)} severity-colour rules in "
          f"{len(uncompiled)} uncompiled files: {len(unexplained)} unpinned, "
          f"{len(found) + len(colours) - len(unexplained)} accepted with a reason")
    return 1 if stale else 0


sys.exit(main())
