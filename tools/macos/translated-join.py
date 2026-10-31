#!/usr/bin/env python3
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parents[2]

COINCIDENCE = {
    ("PlanWindow.swift", "Mission"): "a PLAN TAB name, which is the head's own page vocabulary "
        "set by its tab control. Audited 2026-09-13 and the sentence was sloppier than the "
        "measurement: it said 'never served by the core', and the core DOES serve this literal "
        "-- as a FLIGHT MODE in modes.rs and flightmodes.rs, and as a preflight check name. "
        "Different subject, same nine letters, which is the whole reason this checker cannot "
        "read a collision on its own. What holds is the narrow claim: no view serves the plan's "
        "tab names. Retake it with: grep -rn '\"Mission\"' groundstation/src/*.rs and read which "
        "subject each hit belongs to",
    ("PlanWindow.swift", "Rally"): "the same head-owned PLAN TAB vocabulary as 'Mission', with "
        "the same narrow claim: no view serves the plan's tab names",
    ("PlanWindow.swift", "true"): "a Fact's raw boolean, which Qt spells 'true' in every locale",
    ("Links.swift", "false"): "a Fact's raw boolean, as above",
    ("VehicleSetupWindow.swift", "Sensors"): "a page name from the core's own PAGES table, which "
        "is a list of English constants it owns and does NOT pass through tr(). I first recorded "
        "this as real and it is not: the join in fb0c97a6c is a COMPONENT's tr()'d name against "
        "one of these page ids, which is the variable-to-variable shape this check cannot see at "
        "all. Reading the hit without reading which vocabulary each side came from is how it got "
        "filed wrong",
    ("SettingsWindow.swift", "General"): "a settings page title from the core's own PAGES table "
        "in settings.rs, the same English constants as 'Sensors' that never pass through tr(). "
        "The Updates section is drawn on that page",
}

OUTSTANDING = {}


def translated():
    found = set()
    for path in list((ROOT / "src").rglob("*.cc")) + list((ROOT / "src").rglob("*.qml")):
        text = path.read_text(errors="ignore")
        found |= set(re.findall(r'\bq?[sS]?[tT]r\("([^"\\]{2,60})"\)', text))
    return found


def compared():
    swift = sorted((ROOT / "macos/Sources").glob("*.swift"))
    if not swift:
        print(f"  REFUSING to judge: no *.swift under {ROOT / 'macos/Sources'}. Every accepted "
              f"entry would otherwise be reported stale with an instruction to delete it.",
              file=sys.stderr)
        sys.exit(2)
    for path in swift:
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
