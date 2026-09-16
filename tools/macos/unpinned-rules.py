#!/usr/bin/env python3
"""A conditional that picks a SENTENCE lives where nothing compiles it, so nothing can pin it.

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
that weakens as what it measures improves. It floors the SUBJECT instead -- the uncompiled
file list and the raw hit count -- because a sweep whose subject came back empty reports zero
findings and looks identical to a clean run.

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
    ("FenceRally.swift", "addInclusionCircle"): "the controller METHOD NAME being invoked. Pinning "
        "it in a compiled file would assert a Qt method spelling this head does not own",
}

FLOOR_UNCOMPILED = 40

# NOT a floor on the hit count. The first version of this floored it at 50 and refused at 49,
# because the number falls as the work lands -- flooring it asserts "the corpus still has enough
# defects in it", which is an instrument that blocks precisely when it is succeeding. What the
# guard is actually for is "the pattern still matches", and that is a property of the regex, not
# of the tree, so it is controlled against a sample here instead. This cannot decay: it holds
# whether the corpus has fifty hits or none.
CONTROL = '    Text(value.isEmpty ? "Not reported" : value)'
CONTROL_MISSES = '    Text(value ?? "Not reported")'

TERNARY = re.compile(r'(?<![?\w.])\?(?!\?)\s*("(?:[^"\\]|\\.)*")')


def compiled_files(checks: str, every: list[str]) -> set[str]:
    named = set(re.findall(r'macos/Sources/([A-Za-z0-9_]+\.swift)', checks))
    return named | {name for name in every if 'Model' in name}


def main() -> int:
    given = [arg for arg in sys.argv[1:] if not arg.startswith('--')]
    src = pathlib.Path(given[0]) if given else ROOT / 'macos/Sources'
    checks = (ROOT / 'tools/macos/swift-checks.sh').read_text()
    every = sorted(path.name for path in src.glob('*.swift'))
    if not every:
        print(f"{src} holds no .swift file, so this has measured nothing rather than found "
              "nothing", file=sys.stderr)
        return 2
    uncompiled = [name for name in every if name not in compiled_files(checks, every)]
    if len(uncompiled) < FLOOR_UNCOMPILED:
        print(f"only {len(uncompiled)} uncompiled files, floored at {FLOOR_UNCOMPILED}: either "
              "swift-checks now compiles far more (raise the floor in that commit) or the parse "
              "of its file list broke and this run is measuring almost nothing", file=sys.stderr)
        return 2

    found = [(name, number, line.strip(), match.group(1).strip('"'))
             for name in uncompiled
             for number, line in enumerate((src / name).read_text().splitlines(), 1)
             if not line.strip().startswith('//')
             for match in [TERNARY.search(line)] if match]
    if not TERNARY.search(CONTROL) or TERNARY.search(CONTROL_MISSES):
        print("the pattern no longer separates a real ternary from a nil-coalesce, so a zero "
              "here would read as clean rather than as a broken regex", file=sys.stderr)
        return 2

    unexplained = [hit for hit in found if (hit[0], hit[3]) not in ACCEPTED]
    matched = {(name, literal) for name, _, _, literal in found}
    stale = [key for key in sorted(ACCEPTED) if key not in matched]

    if '--list' in sys.argv:
        for name, number, text, _ in unexplained:
            print(f"{name}:{number}: {text[:110]}")
    for name, literal in stale:
        print(f"ACCEPTED {name} {literal!r} matches nothing now -- an acceptance decays like an "
              "assertion; re-derive it or delete it", file=sys.stderr)
    print(f"{len(found)} conditional string rules in {len(uncompiled)} uncompiled files: "
          f"{len(unexplained)} unpinned, {len(found) - len(unexplained)} accepted with a reason")
    return 1 if stale else 0


sys.exit(main())
