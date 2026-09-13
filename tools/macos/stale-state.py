#!/usr/bin/env python3
"""A field set for a selection and not cleared when the selection goes.

Found by hand on 2026-09-13 in MissionStore.loadSelectedFacts, which assigned eight per-item
fields and cleared three. It drew nothing wrong: every consumer of the five sat inside factCard,
which is driven by selectedFacts, and THAT was cleared. So the safety came from a different field
being cleared -- an implicit coupling that tells the next person nothing, and which I nearly broke
adding selectedLanding an hour earlier.

That is the whole reason this is a script rather than a sentence in a commit message. The rule
cannot be pinned by a test, because the stores it applies to are not among the files
swift-checks.sh compiles. A check nobody can rerun decays into a claim the moment the reader
cannot cheaply retake it.

WHAT IT LOOKS FOR: a `guard ... else { ... return }` block -- the shape a store uses to say
"there is no selection" -- and then every assignment after it in the same function. A name
assigned when something IS selected and never assigned when nothing is has outlived its subject.

The reset block is read WHOLE rather than as a list of assignment lines. The first version
required every line in it to be a bare `x = y`, so loadSelectedFacts -- the function this was
written for -- did not match at all, because two of its four lines are
`if x != y { x = y }`. It reported zero on the very defect it exists to find, and the control
against the pre-fix tree is what said so.

WHAT IT CANNOT SEE, and the exclusions are structural rather than todo items: a field cleared
INDIRECTLY through a helper the guard calls rather than by name, which under-reports and never
invents; whether a stale value is ever DRAWN, which is the difference between a latent coupling
and a visible defect; and -- the one that produces its false positives -- the difference between
a there-is-no-selection RESET and a VALIDATION early-return. Both are `guard ... else { x = y;
return }`, and only the meaning separates them. The two accepted entries below are that case,
kept rather than filtered because naming the confusion is more use to a reader than hiding it.

Usage: python3 tools/macos/stale-state.py
"""
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parents[2]
SOURCES = ROOT / "macos/Sources"

ASSIGNED = re.compile(r"^\s*(?:if \w+ != [^{]*\{ )?(\w+) = ", re.M)
GUARD = re.compile(r"guard [^\n]*else \{\n(.*?)\n\s*return\n\s*\}", re.S)

# A name assigned in the selected path that is deliberately not cleared, and why. A store-local
# variable is not state a reader sees, so it does not belong here -- only @Published fields do.
ACCEPTED = {
    ("SettingsStore.swift", "cache"): "the page cache is a private dictionary, not published "
        "state, and load() clears it on every SUCCESS. Leaving it after a failed read costs "
        "nothing: sections is emptied so the window draws nothing, and the next successful load "
        "removes it before reading anything back",
    ("SettingsStore.swift", "selected"): "which page is chosen outlives an unreachable bridge on "
        "purpose -- it is the operator's choice, not the tree's state, and reselecting their page "
        "for them when the bridge returns is better than dropping them on the first one",
    ("ConnectionsSection.swift", "newName"): "NOT THE SHAPE THIS LOOKS FOR, and the clearest "
        "statement of what it cannot tell apart. create()'s guard is a VALIDATION early-return -- "
        "an empty name sets addError and stops -- not a there-is-no-selection reset. newName is "
        "cleared on SUCCESS, which is a form emptying itself after it was used, the opposite of "
        "a value outliving its subject",
    ("PlanWindow.swift", "replacing"): "the same validation shape: startPlan() returns early when "
        "the plan already has items, and otherwise opens a confirmation by naming what would be "
        "replaced. A guard that refuses is not a guard that clears",
}


def functions(text):
    """Each func's body, found by matching braces rather than by guessing where it ends.

    The first version searched backwards for `func` and forwards for a four-space `}`, which
    walked straight out of the function and collected assignments from the ones after it: 25
    findings, almost all of them another function's business. A regex cannot find the end of a
    Swift body; counting braces can.
    """
    for opened in (m for m in re.finditer(r"\n    (?:@\w+ )?(?:private |static |func)[^\n]*func (\w+)", text)):
        name = opened.group(1)
        start = text.index("{", opened.end() - 1)
        depth, i = 0, start
        while i < len(text):
            if text[i] == "{":
                depth += 1
            elif text[i] == "}":
                depth -= 1
                if depth == 0:
                    break
            i += 1
        yield name, text[start : i + 1]


def blocks(text):
    for name, body in functions(text):
        guard = GUARD.search(body)
        if not guard:
            continue
        cleared = set(ASSIGNED.findall(guard.group(1)))
        after = set(ASSIGNED.findall(body[guard.end():]))
        yield name, cleared, after


findings = []
for source in sorted(SOURCES.glob("*.swift")):
    for func, cleared, after in blocks(source.read_text()):
        for name in sorted(after - cleared):
            if (source.name, name) in ACCEPTED:
                continue
            findings.append((source.name, func, name))

for where, func, name in findings:
    print(f"  OUTLIVES {where} {func}() assigns {name!r} when something is selected and never "
          f"when nothing is, so it keeps describing a subject that has gone. Clear it beside the "
          f"others, or accept it here with the reason it is safe to keep", file=sys.stderr)

print(f"checked every guard-else reset in {len(list(SOURCES.glob('*.swift')))} head files: "
      f"{len(findings)} field(s) outlive their selection, {len(ACCEPTED)} accepted with a reason")
sys.exit(1 if findings else 0)
