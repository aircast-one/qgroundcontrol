#!/usr/bin/env python3
"""A core field that can be absent must not reach the screen as a zero.

view-fields.py catches a key the core stopped emitting. This catches the opposite
and quieter failure: a key the core still emits, whose VALUE is null, which the
head's decoder turns into 0, "" or false. The operator then reads a figure the
vehicle never reported.

a9b4e2a53 made a link's port and baud nullable because port is a Q_PROPERTY on
TCPLink and baud on SerialLink alone, so a UDP link has neither. This head went on
reading `(json["baud"] as? NSNumber)?.intValue ?? 0` and drew a rate of 0 where the
honest answer is a blank. 8edb642a7 fixed it. Nothing would have found the next one.

It reads LIVE payloads rather than the core's source, which fixes the direction the
source cannot answer -- whether an Option is actually None in a state this machine
can reach -- and costs the other: a field null only in a state no plan here reaches
is invisible. It UNDER-reports and never invents. It also sees only the nineteen
models below; the seventeen views decoded inline in a store are out of its reach,
and a text window around a decode cannot follow one across a call boundary anyway.

Control: restore LinkConfigModel.swift from 8edb642a7^ into a copy of macos/Sources
and pass that copy as argv[1]. It must report baud. (port comes back non-null while
the live links are UDP, which reads its localPort -- one of the pair is enough.)

Usage: python3 tools/macos/null-fallbacks.py [sources-dir]
"""
import json
import pathlib
import re
import sys
import urllib.request

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
from head_models import MODELS, views

ROOT = pathlib.Path(__file__).resolve().parents[2]
SOURCES = pathlib.Path(sys.argv[1]) if len(sys.argv) > 1 else ROOT / "macos/Sources"
PORT = 8777


# A fallback that is reachable but right, or unreachable. Each reason names the gate,
# and a gate that moves has to move this line with it. Without this table the steady
# state is four hits nobody reads, which is how an instrument stops being believed.
ACCEPTED = {
    ("Detections", "error"): "MEASURED: the core sends null when there is no error and a "
        "sentence when there is -- detections.rs carries self.error straight through, and its "
        "own test pins \"connection refused\" against a null in the quiet case. Null and the "
        "empty string mean the SAME THING to every consumer here, so the fallback merges two "
        "spellings of one fact rather than inventing a value. The head exposes it only through "
        "the video probe's state; nothing draws it",
    ("GuidedRange", "label"): "UNREACHABLE BY CONSTRUCTION, and measured on all three views "
        "this struct decodes. speed.rs builds the label as range.as_ref().map(|r| r.label), so "
        "it is null exactly when there is no range -- which is exactly when available is false. "
        "altitude.rs and takeoff.rs emit the literal unconditionally and were measured sending "
        "\"Height above launch\" while unavailable, so guidedSpeed is the only one that answers "
        "null at all. GuidedValueModel's init is failable and returns nil unless available is "
        "true, so the fallback cannot be reached: a null label and a constructed range are "
        "mutually exclusive states",
    ("MissionItem", "command"): "MEASURED, and the reason I first wrote was wrong twice. It "
        "said ONLY the launch row sends null; the rule is a CLASS -- no COMPLEX item carries a "
        "command, so the settings row, the survey, the corridor and the structure scan all send "
        "null here. And it quoted canChangeCommand as isSimpleItem && sequence > 0 && !isLaunch, "
        "a rule 1942cdad0 deleted. The gate is now isSimpleItem && !isLaunch, and isSimpleItem "
        "alone excludes every item in that measured set, so the acceptance holds for a BROADER "
        "reason than the one written. The mission probe's pickCommand does not check the gate, "
        "but the catalogue holds no command 0, so a defaulted zero highlights nothing",
    ("MissionItem", "category"): "the same measured set behind the same gate, and the same two "
        "corrections: every complex item, not one row, and the gate no longer names a sequence",
    ("MissionItem", "specifiesAltitude"): "MEASURED, and my first reason for this entry was "
        "wrong: it named the launch row, where the rule is a whole class. The core withholds "
        "this for anything that is NOT a simple item -- the settings row and every pattern send "
        "null -- while a simple item answers true or FALSE, and the false is real: a command "
        "item measured false here. So null and false are DIFFERENT answers, not-applicable "
        "against measured-no, and decoding the null to false merges them. It is safe only "
        "because altitudeReading names settingsKind explicitly and every pattern wants the band "
        "the merge sends it to. A third state arriving in this flag would be silent",
    ("MissionItem", "altitudeBandText"): "MEASURED: null on EVERY SIMPLE item and on the "
        "settings row -- a band is the range a pattern sweeps, so an item that flies to one "
        "height has none. My first reason named the settings kind alone, which is a row where "
        "the rule is a class. Safe in both: the settings kind returns altitudeText before the "
        "band is consulted, and for a simple item that specifies no altitude the empty string "
        "falls to the em dash, which is the honest answer for a row with no height",
    ("MissionItem", "altitudeText"): "every pattern and the Return To Launch send it null, "
        "because altitudeText is built from height(read) and those items carry no height fact. "
        "altitudeReading only reaches altitudeText when specifiesAltitude is true or the kind is "
        "settings, and none of these are either, so the em dash it falls back to is never the "
        "string drawn -- the band or the em dash answers first",
    ("MissionItem", "altitudeUnits"): "the same items, and the fallback is UNREACHABLE in every "
        "state this rig produces: altitudeUnits is only read through altitudeFieldUnits, which "
        "PlanWindow reaches only inside `if item.specifiesAltitude`. The core derives units and "
        "text from one height(read), so a null unit means no height -- but specifiesAltitude is "
        "a separate flag, so an item claiming an altitude it has no height for would draw a "
        "defaulted metre. Nothing here produces one and I am not inventing it",
    ("MissionItem", "altitudeFrame"): "the Return To Launch alone: a command with no altitude "
        "has no reference to name one from, and frameSuffix maps the empty string to no suffix. "
        "A pattern is NOT in this set -- its frame is amsl, from the band",
}

COMMENT = re.compile(r"//[^\n]*")
FALLBACK = re.compile(
    r'\w+\??\[\s*"([^"]+)"\s*\]\s*(?:as\?\s*\w+\s*)?(?:\)?\s*\??\.\w+Value\s*)?\)?\s*\?\?\s*(\S+)')


def get(path):
    request = urllib.request.Request(f"http://127.0.0.1:{PORT}/bridge/get?path={path}",
                                     headers={"X-QGC-Debug-Api": "1"})
    return json.load(urllib.request.urlopen(request, timeout=8))


def nulls(value):
    if isinstance(value, dict):
        return ({key for key, held in value.items() if held is None}
                | set().union(*map(nulls, value.values())))
    if isinstance(value, list):
        return set().union(*map(nulls, value))
    return set()


def balanced(text, start):
    depth = 0
    for end in range(start, len(text)):
        depth += text[end] == "{"
        depth -= text[end] == "}"
        if not depth:
            return end
    return len(text) - 1


def body_of(name):
    for swift in sorted(SOURCES.glob("*.swift")):
        text = COMMENT.sub("", swift.read_text())
        declaration = re.search(rf"\n(?:final )?(?:struct|class|enum) {name}\b", text)
        if not declaration:
            continue
        following = text[declaration.end():]
        stop = re.search(r"\n(?:final )?(?:struct|class|enum) \w+", following)
        return swift.name, (following[: stop.start()] if stop else following)
    return None, None


def fallbacks(body):
    bodies = (body[brace:balanced(body, brace)]
              for brace in (body.find("{", init.end())
                            for init in re.finditer(r"\n    (?:public )?init\??\(", body))
              if brace >= 0)
    return ((key, default, line.strip())
            for found in bodies for line in found.splitlines()
            for key, default in FALLBACK.findall(line))


reached, hits, unchecked = 0, [], []
for model, view in sorted(MODELS.items()):
    payloads = []
    for one in views(view):
        try:
            payloads.append(get(one))
        except Exception as problem:
            unchecked.append(f"{model}: {one} could not be read -- {problem}")
            payloads = None
            break
    if payloads is None:
        continue
    refused = [one for one, payload in zip(views(view), payloads)
               if isinstance(payload, dict) and payload.get("kind") == "refused"]
    if refused:
        unchecked.append(f"{model}: {refused} refused in this state, so it sent no values to check")
        continue
    reached += 1
    sent = set().union(*map(nulls, payloads))
    name, body = body_of(model)
    if body is None:
        unchecked.append(f"{model}: no declaration in {SOURCES}, which is a broken reader")
        continue
    hits += [(name, model, key, default, line)
             for key, default, line in fallbacks(body)
             if key in sent and (model, key) not in ACCEPTED]

# An accepted line outlives the code it excuses: the head stops writing the fallback, the
# reason stays, and the table quietly grows into a list of things nobody has read in months.
# Only the source half is checked here -- whether the model still falls back on that key at
# all. Whether the key still comes back null depends on the state this machine can reach, and
# reporting on that would cry wolf on every view a vehicle would have filled in.
stale = [f"{key!r} is accepted for {model}, which is not a model this file checks"
         for model, key in sorted(ACCEPTED) if model not in MODELS]
stale += [f"{key!r} is accepted for {model}, and {model} no longer falls back on it"
          for model, key in sorted(ACCEPTED) if model in MODELS
          and key not in {read for read, _, _ in fallbacks(body_of(model)[1] or "")}]

for why in unchecked + stale:
    print(f"  NOT CHECKED {why}", file=sys.stderr)
for name, model, key, default, line in hits:
    print(f"  NULL AS VALUE {name} {model} reads {key!r}, which {MODELS[model]} sent as null, "
          f"and falls back to {default}: {line[:90]}", file=sys.stderr)

print(f"read {reached} of {len(MODELS)} views live and checked what each model does with the "
      f"keys that came back null: {len(hits)} turn one into a value, "
      f"{len(ACCEPTED)} accepted with a reason")
sys.exit(1 if hits or unchecked or stale else 0)
