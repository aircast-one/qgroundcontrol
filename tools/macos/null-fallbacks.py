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
from head_models import MODELS

ROOT = pathlib.Path(__file__).resolve().parents[2]
SOURCES = pathlib.Path(sys.argv[1]) if len(sys.argv) > 1 else ROOT / "macos/Sources"
PORT = 8777


# A fallback that is reachable but right, or unreachable. Each reason names the gate,
# and a gate that moves has to move this line with it. Without this table the steady
# state is four hits nobody reads, which is how an instrument stops being believed.
ACCEPTED = {
    ("MissionItem", "command"): "only the launch row sends null, and canChangeCommand -- "
        "isSimpleItem && sequence > 0 && !isLaunch -- keeps it out of chosenCommand, the "
        "one consumer. The mission probe's pickCommand does not check that gate, but the "
        "catalogue holds no command 0, so the probe's route highlights nothing either",
    ("MissionItem", "category"): "the same row behind the same gate: pickCommand is reached "
        "only through the button canChangeCommand draws",
    ("MissionItem", "specifiesAltitude"): "the launch row specifies no altitude, so false is "
        "what it means, and altitudeReading names settingsKind rather than leaning on the flag",
    ("MissionItem", "altitudeBandText"): "the settings kind returns altitudeText before the "
        "band is consulted, so the empty string is never the sentence drawn",
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
    try:
        payload = get(view)
    except Exception as problem:
        unchecked.append(f"{model}: {view} could not be read -- {problem}")
        continue
    if isinstance(payload, dict) and payload.get("kind") == "refused":
        unchecked.append(f"{model}: {view} refused in this state, so it sent no values to check")
        continue
    reached += 1
    sent = nulls(payload)
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
