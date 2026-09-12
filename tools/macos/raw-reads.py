#!/usr/bin/env python3
"""A head reading a raw Qt path is applying its own gate, or none.

view-fields.py asks whether the keys a model reads still exist. This asks the question one
level up: is the head reading Qt directly for something the core now serves as a view?

That is not hypothetical. This head read positionManager.gcsPosition raw and judged it with
MapCentre.usable -- valid coordinate, not null island. The core's view.gcsPosition also
refuses a fix coarser than 100 m, one older than 5 s, and an out-of-range latitude. So a
coarse or stale reading still centred My Location, up to two kilometres from where the
operator stands, with nothing on screen saying the fix was poor. bbf187780 fixed it.

A raw read is not wrong by itself: writes go to Qt paths by design, and plenty of state has
no view. What this reports is the narrow case where a SERVED VIEW NAMES THE SAME SUBJECT,
which is where the head is most likely to be re-deciding something the core already decided.

It matches the LAST dotted component against the view registry EXACTLY, so
positionManager.gcsPosition finds view.gcsPosition. It cannot see a view whose name differs
from the path's -- vehicle.terrain against view.terrainProfile is invisible to it, and that
is the direction it under-reports rather than invents. An earlier version folded a trailing
s and reported `vehicle` against view.vehicles, which are different subjects entirely: one
aircraft's live facts against the list of connected ones. Matching NAMES is not matching
subjects, and the fold bought one more name at the cost of a false report.

Control: run it against bbf187780^ and it must report positionManager.gcsPosition.

Usage: python3 tools/macos/raw-reads.py [sources-dir]
"""
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parents[2]
SOURCES = pathlib.Path(sys.argv[1]) if len(sys.argv) > 1 else ROOT / "macos/Sources"
CORE = ROOT / "core-rs/src"

# A raw read kept on purpose. Each reason names what the view does NOT cover, and a view that
# grows to cover it has to move this line with it.
ACCEPTED = {
    "plan": "MEASURED: the head takes syncInProgress and canUndo from here, and view.plan "
        "carries readiness and upload. The names collide; the fields do not. The core serves "
        "no undo state and no sync flag, so there is nothing here to defer to",
    "links": "MEASURED: the head takes connectingLinkName and serialPortStrings from here -- "
        "a transient connection attempt and the machine's serial ports. view.links is the "
        "list of CONFIGURED links, which Links.swift also reads and draws. Different subjects "
        "under one name",
    "vehicle.batteries": "MEASURED: Fly.swift reads this for the per-pack DETAIL ROWS and "
        "reads view.battery ten lines later for the line and the levels. The raw read takes "
        "each Fact's valueString, which is the same locale-aware string the core builds "
        "voltage_text from, so the two do NOT disagree in spelling -- this is a duplicated "
        "SOURCE, not a duplicated derivation. view.battery names six facts per pack and caps "
        "the count; the detail panel shows temperature, power and mAh consumed, which the view "
        "does not carry. Revisit if the view grows those",
}


def views():
    registry = (CORE / "view.rs").read_text()
    return {m.group(1) for m in re.finditer(r'View\s*\{\s*path:\s*"([^"]+)"', registry)}


def fold(name):
    return name.lower()


served = {fold(v.split(".", 1)[1].split("(")[0]): v for v in views() if "." in v}

reads = {}
for source in sorted(SOURCES.glob("*.swift")):
    for match in re.finditer(r'Bridge\.group\("([^"]+)"', source.read_text()):
        path = match.group(1)
        if path.startswith("view.") or "\\(" in path:
            continue
        reads.setdefault(path, set()).add(source.name)

hits = [(path, sorted(files), served[fold(path.rsplit(".", 1)[-1])])
        for path, files in sorted(reads.items())
        if fold(path.rsplit(".", 1)[-1]) in served and path not in ACCEPTED]

stale = [f"{path!r} is accepted and this head no longer reads it"
         for path in sorted(ACCEPTED) if path not in reads]

for why in stale:
    print(f"  NOT CHECKED {why}", file=sys.stderr)
for path, files, view in hits:
    print(f"  RAW READ {files} reads {path!r} straight from Qt, and the core serves {view} for "
          f"the same subject. Ask what that view refuses that this head does not.",
          file=sys.stderr)

print(f"{len(reads)} non-view paths read by this head; {len(hits)} have a view naming the same "
      f"subject, {len(ACCEPTED)} accepted with a reason")
sys.exit(1 if hits or stale else 0)
