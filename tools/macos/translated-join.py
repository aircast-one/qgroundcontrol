#!/usr/bin/env python3
"""A comparison against a string QGC wraps in tr() breaks in every non-English build.

Six instances appeared in one session, the third written an hour after the first was fixed, so
knowing the rule is demonstrably not enough. This is the check. The shape came from the core
session: intersect the literals this head COMPARES against with the literals QGC TRANSLATES.

WHAT IT CANNOT SEE, and the exclusion is the larger half: a join between two VARIABLES whose
origin is translated. The armed gate ($0.name == page) and createPlan ($0.command == complex)
are both that shape and neither carries a literal. Control: of six known instances this reports
one and misses four; the sixth is the core's. So a clean run is NOT evidence of no join.

NOT every tr() literal is translated, and the exception is load-bearing: a tr() in a STATIC
initialiser runs before installTranslator, so it is frozen at source text in every locale. The
Android session established this for CorridorScanComplexItem::name (fd277b067), which is why
comparing against it is safe. This check cannot tell the two apart and will flag such a
comparison; the answer is to record it here with where the tr() is evaluated, not to remove it.
Both entries below were checked that way -- linkTypeStrings fills a function-local static on
first call, and a component's _name is a member initialiser run when a vehicle connects.

Precision on this head is about half -- "true", "false", "Mission" and "Rally" all collide with
some tr() literal by accident. Those are listed with a reason. A hit that is REAL but not yet
fixable is listed separately and still printed, because the reason it stays is a missing field
somewhere else, not a decision that it is fine.

Usage: python3 tools/macos/translated-join.py
"""
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parents[2]

COINCIDENCE = {
    ("PlanWindow.swift", "Mission"): "the head's OWN page vocabulary, set by its tab control and "
        "never served by the core -- it collides with a tr() literal somewhere in QGC by accident",
    ("PlanWindow.swift", "Rally"): "the same head-owned page vocabulary as 'Mission'",
    ("PlanWindow.swift", "true"): "a Fact's raw boolean, which Qt spells 'true' in every locale",
    ("Links.swift", "false"): "a Fact's raw boolean, as above",
}

OUTSTANDING = {
    ("VehicleSetupWindow.swift", "Sensors"): "REAL, and the same missing join recorded in "
        "fb0c97a6c -- a setup page's name is tr()'d, so the failing-sensors badge appears on no "
        "row outside English. Waiting on the same core-side page id",
}


def translated():
    found = set()
    for path in list((ROOT / "src").rglob("*.cc")) + list((ROOT / "src").rglob("*.qml")):
        text = path.read_text(errors="ignore")
        found |= set(re.findall(r'\bq?[sS]?[tT]r\("([^"\\]{2,60})"\)', text))
    return found


def compared():
    for path in sorted((ROOT / "macos/Sources").glob("*.swift")):
        for match in re.finditer(r'[!=]=\s*"([^"\\]{2,60})"', path.read_text(errors="ignore")):
            yield path.name, match.group(1)


tr = translated()
hits = {(name, text) for name, text in compared() if text in tr}
unexplained = sorted(hits - set(COINCIDENCE) - set(OUTSTANDING))
gone = sorted(key for key in (COINCIDENCE | OUTSTANDING) if key not in hits)

for name, text in sorted(hits & set(OUTSTANDING)):
    print(f"  OUTSTANDING {name} compares against {text!r}: {OUTSTANDING[(name, text)]}",
          file=sys.stderr)
for name, text in gone:
    print(f"  STALE {name} no longer compares against {text!r}; drop the entry", file=sys.stderr)
for name, text in unexplained:
    print(f"  TRANSLATED JOIN {name} compares against {text!r}, which QGC wraps in tr(). Outside "
          f"English that comparison is never true. Key on an id, a class or a number, or add the "
          f"reason it is a coincidence", file=sys.stderr)

print(f"checked {len(tr)} translated literals against every string this head compares: "
      f"{len(hits)} hits, {len(unexplained)} unexplained, {len(hits & set(OUTSTANDING))} real and "
      f"waiting on a served id, {len(hits & set(COINCIDENCE))} coincidences")
sys.exit(1 if unexplained or gone else 0)
