#!/usr/bin/env python3
"""A field named *Text exists only to be drawn, so one no head names is a gap by construction.

The Android head's predicate, and it is sharper than my unread-field sweep for a reason
worth keeping: that one asks "does this head name this key", which needs a judgement about
every field -- is it a raw number behind a text, an envelope tag, a term an instrument
diffs -- and produced 69 hits nobody would read. This one asks nothing. A key ending in
Text is a string the core formatted FOR A SCREEN. If no head names it, either a screen is
missing or the core is formatting for nobody, and both are worth a look.

It also reports a NON-defect honestly, which their run showed and mine repeats: the one
unnamed field here is bandText, and the reason it is unnamed is a display choice this head
owns. An instrument that can only ever say "gap" is one you stop believing.

Control: restore MissionItemModel.swift from 4e4fbdc1c^ into a copy of macos/Sources and
pass that copy as argv[1]. speedChangeText must appear, because that is the cycle before
this head drew it.

A key containing a dot is a PATH, not a field. view.contract keys its enumerations by
path, so view.camera.modeText arrived looking like an undrawn text when the head reads
modeText perfectly well -- the first run's only other hit, and self-inflicted.

Usage: python3 tools/macos/text-fields.py [sources-dir]
"""
import json
import pathlib
import re
import sys
import urllib.request

ROOT = pathlib.Path(__file__).resolve().parents[2]
SOURCES = pathlib.Path(sys.argv[1]) if len(sys.argv) > 1 else ROOT / "macos/Sources"
PORT = 8777

# A view that needs an argument answers nothing without one. These are the ones a plan on
# the ground can produce; the rest want a file, a vehicle or a coordinate and are named as
# unread rather than guessed at.
ARGUMENTS = {
    "view.surveyStats": "(4)",
    "view.missionItems": "(geometry)",
    "view.polygon": "(plan.missionController.visualItems.4.surveyAreaPolygon)",
    "view.setup": "(Motors)",
    "view.settings": "(General)",
    "view.mapScale": "(400)",
    "view.altitudeModes": "(item,4)",
}

ACCEPTED = {
    "bandText": "the terrain panel draws lowestText and highestText at the plot's two ends "
                "rather than one sentence between them, so the joined spelling is a display "
                "choice this head makes and not a figure it fails to show",
    "layerSpanText": "DUPLICATE, and a SCREENSHOT is what proved it. The core spells the band a "
                     "structure scan sweeps in metres RELATIVE to launch -- measured "
                     "\"62.5 m to 87.5 m\" -- and this head already draws the SAME BAND on the "
                     "item's own row as \"548 m to 572 m AMSL\", from altitudeBandText with "
                     "altitudeFrameText saying AMSL. Launch sits at 485 m: 485+62.5 is 547.5 and "
                     "485+87.5 is 572.5, so the two strings are one measurement in two reference "
                     "frames. I had this filed as a TRUE undrawn finding and left the instrument "
                     "red on purpose for a cycle; looking at the rendered window is what settled "
                     "it, because the row's band and the geometry view's span never appear side "
                     "by side in any payload. Drawing both would put the same figure on screen "
                     "twice in different units, which is worse than drawing neither",
    "rangeText": "DECLINED, and the core agreed and declined to replace it. The guided slider draws "
                 "its two ends as SEPARATE labels at opposite ends of the track, so a combined "
                 "\"2 m to 150 m\" has nowhere to go without splitting a sentence the core "
                 "assembled. The deeper reason is that the value above the slider is DRAGGED -- it "
                 "never reaches the core and can only be spelled here -- so any rule the core owns "
                 "on the ends is a SECOND precision rule on one control, visible at the same "
                 "moment: \"2.0 m\" in the header above \"2 m\" at the end label. I asked for "
                 "minimumText/maximumText instead and the core refused with my own argument, which "
                 "was right: three labels agree only if one rule spells all three, and only the "
                 "head can spell the dragged one. GuidedRange.text() is that rule",
}


def get(path):
    request = urllib.request.Request(f"http://127.0.0.1:{PORT}/bridge/get?path={path}",
                                     headers={"X-QGC-Debug-Api": "1"})
    return json.load(urllib.request.urlopen(request, timeout=8))


def texts(value):
    if isinstance(value, dict):
        return ({key for key in value if key.endswith("Text") and "." not in key}
                | set().union(*map(texts, value.values()), set()))
    if isinstance(value, list):
        return set().union(*map(texts, value), set())
    return set()


registry = re.findall(r'View\s*\{\s*path:\s*"([^"]+)"', (ROOT / "core-rs/src/view.rs").read_text())
named = "\n".join(path.read_text() for path in sorted(SOURCES.glob("*.swift")))

served, silent = set(), []
for path in registry:
    asked = path + ARGUMENTS.get(path, "")
    try:
        payload = get(asked)
    except Exception as problem:
        silent.append(f"{asked}: {problem}")
        continue
    if isinstance(payload, dict) and payload.get("kind") == "null":
        silent.append(f"{asked}: refused -- {str(payload.get('reason'))[:60]}")
        continue
    served |= texts(payload)

undrawn = sorted(name for name in served
                 if f'"{name}"' not in named and name not in ACCEPTED)
stale = sorted(name for name in ACCEPTED if name not in served)

for why in silent:
    print(f"  NOT READ {why}", file=sys.stderr)
for name in undrawn:
    print(f"  UNDRAWN {name!r} is a text the core formatted for a screen and this head "
          f"names nowhere", file=sys.stderr)
for name in stale:
    print(f"  NOT CHECKED {name!r} is accepted and the core no longer serves it",
          file=sys.stderr)

print(f"{len(served)} fields named *Text across {len(registry) - len(silent)} views this rig "
      f"can read: {len(served) - len(undrawn) - len(ACCEPTED)} drawn, {len(undrawn)} undrawn, "
      f"{len(ACCEPTED)} accepted with a reason")
sys.exit(1 if undrawn or stale else 0)
