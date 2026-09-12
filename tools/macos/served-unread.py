#!/usr/bin/env python3
"""A field the core ADDS is invisible to every other instrument here.

view-fields.py asks whether a key this head DECODES still exists. That direction catches a
removal and cannot catch an addition, so every served-field change that mattered on 2026-09-12
was found by reading a peer's commit or by being told: flightLoop and layers (a structure scan
drew no route), confidence (every detection lost its percentage), rangeText, spinsPropeller.

An earlier attempt compared a model's keys against every quoted literal in the core module that
serves the view. It failed its control: "conf" is a literal in detections.rs because the core
READS that name from upstream on the very line it renames it, so the checker could not tell a key
the core SERVES from one it CONSUMES.

The contract has no such confusion. test/Bridge/fixtures/view-shapes.json records only what was
SERVED, and its _observed list carries every view.path.field ever seen. This reads that.

WHAT IT CANNOT SEE, and the exclusion is structural rather than a bug to fix later: a field inside
an ARRAY element. view.detections records boxes as ["empty"], so x, y, w, h, label, confidence and
target are pinned nowhere, which is exactly why the conf rename was invisible. The recorder cannot
synthesise a detection box because the feed is an external SSE stream. Control: of six fields found
the hard way, this reports five and misses confidence for that reason.

It matches a field by NAME across all of macos/Sources rather than per view, so a name this head
reads for one view counts as read for every view. That UNDER-reports and never invents.

Usage: python3 tools/macos/served-unread.py
"""
import json
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parents[2]
SOURCES = ROOT / "macos/Sources"
CONTRACT = ROOT / "test/Bridge/fixtures/view-shapes.json"

# The last contract this head has reconciled. Move it forward when the additions
# since it have each been read or accepted -- never to silence a hit.
SINCE = "11124b8e3"

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
from head_models import MODELS  # noqa: E402


# Every field the core serves that this head does not read, and why that is right. A view the head
# does not read AT ALL is not listed here -- view-fields.py already carries a reason for each of
# those, and repeating them would be two tables to keep true instead of one.
ACCEPTED = {
    "spinsPropeller": "MEASURED and deliberately not decoded. calibration.rs began serving it with "
        "CompassMot, the first routine that spins the propellers. The WARNING text beside it "
        "carries the same fact in words an operator reads, and this head has no structural use "
        "for the flag -- no confirmation step it would gate. Decoding it to store it would be the "
        "unread-field debt removed in c485b1aac. A confirmation before starting a routine is what "
        "would earn the decode",
    "layers": "the COUNT of a structure scan's stacked circuits, already on screen as the Layers "
        "fact in the item's own editor, and two stacked circuits are ONE SHAPE on a flat map -- "
        "drawing the ring twice puts identical points on identical points. Android reached the "
        "opposite decision correctly, because their head shows a structure scan's facts nowhere",
    "layerSpanText": "the band a structure scan sweeps, spelled RELATIVE to launch. The item row "
        "already draws the same band as AMSL from altitudeBandText -- 485 m launch, so "
        "\"62.5 m to 87.5 m\" and \"548 m to 572 m AMSL\" are one measurement in two frames. A "
        "SCREENSHOT proved it; the two strings never appear side by side in any payload",
    "rcSignalText": "the RC percentage, composed. MEASURED AGAINST THE HEAD'S OWN SPELLING AND "
        "REJECTED: flystate.rs formats every accepted value as \"{percent}%\", so a reading of 0 "
        "becomes \"0%\". This head says \"No signal\" for 0, which is a different statement -- a "
        "transmitter that is off is not a transmitter at 0 per cent -- and FlyDetailModel carries "
        "the comment explaining why. Adopting the composed text would lose that distinction, so "
        "the head keeps its own and the core has been told which case differs",
    "altitudeMetres": "the RAW quantity behind a mission item's altitude, served for a head that "
        "would rather convert and write metres through the fact's rawValue setter. This head does "
        "the opposite on purpose: 8110770d3 fixed an editable altitude wearing the wrong unit by "
        "drawing the fact's OWN value beside the fact's OWN unit, and writing through setFact so "
        "the conversion happens where the unit is defined. Decoding a metre quantity here would "
        "put a second conversion in the head, which is the defect that fix removed",
    "autoDisconnect": "whether the vehicle drops its link by itself after contact is lost. It is "
        "a SETTING, and this head neither draws it nor may write it -- FirmwareUpgrade.qml is the "
        "only place upstream that touches it, and it WRITES it as part of a flow this head does "
        "not have. Drawing a control's state with no control is the unread-field debt removed in "
        "c485b1aac",
    "rangeText": "the guided slider's two ends as one combined string. This head draws them as "
        "SEPARATE labels at opposite ends of the track, and the dragged value between them can "
        "only be spelled here, so adopting it would put two precision rules on one control. The "
        "core agreed and declined to serve separate ends",
}


# A view this head does not read AT ALL already has a reason in view-fields.py's UNDRAWN table.
# Reporting its fields here would be the same decision written twice, and the first run did exactly
# that: 221 hits, almost all of them fields of adsbTraffic, followMe, packetRadio and the other
# views recorded as not built. One table stays true; two drift apart.
UNDRAWN_PREFIXES = [
    line.split('"')[1]
    for line in (ROOT / "tools/macos/view-fields.py").read_text().splitlines()
    if line.strip().startswith('("view.')
]


def read_here(view):
    base = view.split(".items.")[0].split(".contacts")[0]
    return not any(base.startswith(prefix) for prefix in UNDRAWN_PREFIXES)


def observed_fields(text=None):
    contract = json.loads(text) if text else json.loads(CONTRACT.read_text())
    seen = {}
    for path in contract.get("_observed", []):
        view, _, field = path.rpartition(".")
        if not (view and field.isidentifier()) or not read_here(view):
            continue
        seen.setdefault(field, set()).add(view)
    return seen


def contract_at(revision):
    import subprocess
    shown = subprocess.run(["git", "show", f"{revision}:test/Bridge/fixtures/view-shapes.json"],
                           cwd=ROOT, capture_output=True, text=True)
    return shown.stdout if shown.returncode == 0 else None


def names_the_head_uses():
    text = "\n".join(path.read_text() for path in sorted(SOURCES.glob("*.swift")))
    return set(re.findall(r'"([A-Za-z_][A-Za-z0-9_]*)"', text)) | set(
        re.findall(r"\b([a-z][A-Za-z0-9_]*)\b", text))


used = names_the_head_uses()
before = contract_at(SINCE)
if before is None:
    print(f"  CANNOT COMPARE the contract against {SINCE}, which git does not have",
          file=sys.stderr)
    sys.exit(1)

now, then = observed_fields(), observed_fields(before)
added = sorted((field, sorted(views)) for field, views in now.items()
               if field not in then and field not in used and field not in ACCEPTED)

stale = [f"{field!r} is accepted and the core no longer serves it"
         for field in sorted(ACCEPTED) if field not in now]

for why in stale:
    print(f"  NOT SERVED {why}", file=sys.stderr)
for field, views in added:
    where = ", ".join(views[:3]) + (" and more" if len(views) > 3 else "")
    print(f"  NEWLY SERVED {field!r} appeared in the contract for {where} since {SINCE}, and no "
          f"file under macos/Sources names it. Read it, or accept it with the reason it stays "
          f"undrawn and move SINCE forward", file=sys.stderr)

print(f"compared {len(now)} served field names against the contract at {SINCE}: "
      f"{len(now) - len(then)} added since, {len(added)} of them named nowhere in this head, "
      f"{len(ACCEPTED)} accepted with a reason")
sys.exit(1 if added or stale else 0)
