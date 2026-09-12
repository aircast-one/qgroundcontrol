#!/usr/bin/env python3
"""Every key this head reads out of a view must be one the core still emits.

bridge-paths.py pins PATHS. A field renamed or removed INSIDE a served view is
invisible to it: the path still resolves, the key is simply absent, and the head's
decoder turns that into a default. The failure is silent in both directions --
`json["gone"] as? NSNumber ?? false` reads false forever, and a fixture that sets
"gone" agrees with the misreading rather than with the producer.

That is not hypothetical. df4cffb57 replaced a per-item "current" flag with a
"selected" index; this head went on reading "current", every item came back
unselected, and the item facts panel, the altitude mode, the per-item speed, the
survey statistics and the map's .required row were all dead at once. No test
noticed, because the fixtures set "current" themselves.

It compares DECLARATIONS on both sides and needs no running app, which matters:
the alternative -- diffing against a live payload -- cries wolf on every field the
core emits only in a state the current plan is not in, blockedReason being one.

The view-to-module half is parsed out of core-rs/src/view.rs rather than written
here, so it cannot drift. The model-to-view half is written here and each entry is
checked against that table; a view path this file invents is reported, not ignored.

Usage: python3 tools/macos/view-fields.py
"""
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parents[2]
SOURCES = ROOT / "macos/Sources"
CORE = ROOT / "core-rs/src"

# Which Swift model decodes which view. The head builds these from a view payload and
# nothing else, so every key their initialisers look up has to be a key that view emits.
MODELS = {
    "MissionItem": "view.missionItems",
    "MissionSummary": "view.missionSummary",
    "MissionItemKind": "view.missionKinds",
    "TerrainProfile": "view.terrainProfile",
    "FenceShape": "view.fences",
    "SurveyStats": "view.surveyStats",
    "AltitudeModeOffer": "view.altitudeModes",
    "CalibrationState": "view.calibration",
    "CameraControl": "view.camera",
    "FlightModeChoice": "view.flightModes",
    "LinkConfig": "view.links",
    "LogEntry": "view.logs",
    "MavlinkMessage": "view.inspector",
    "PreflightCheck": "view.preflight",
    "RadioState": "view.radio",
    "SensorHealth": "view.sensors",
    "VehicleComponentInfo": "view.setup",
    "VibrationReading": "view.vibration",
    "VideoStatus": "view.video",
}

# A subscript on the decoded dictionary, or a literal handed to a helper closing over it.
# Not a bare quoted run: pairing the closing quote of one literal with the opening quote of
# the next reports the code between keys as a key, which is how an earlier attempt at this
# declared 247 fixtures broken on its first run.
LOOKUP = re.compile(r'\w+\??\[\s*"([^"]+)"\s*\]|\b\w+\(\s*"([^"]+)"\s*\)')

# Every quoted literal the module mentions, not just the ones written "key": inside a json!
# literal -- the core also assigns keys as json["enabled"] = ..., and matching only the first
# form reported two live fields as gone on this checker's first run.
#
# Comments are stripped first, and that is load-bearing rather than tidiness: missionitems.rs
# explains the rename in prose that contains the word "current" in quotes, so collecting
# comments would find the very key whose removal this exists to catch. The head's Swift is
# stripped for the same reason and the asymmetry was a real hole: a comment inside an
# initialiser mentioning json["gone"] reads exactly like a line that looks it up, and the
# checker would report a key the head does not read as one the core dropped.
# Escape-aware. A pattern like "([^"\\]*)" cannot match a Rust string containing \n or \",
# so it skips that literal and pairs the NEXT closing quote with the following opening one --
# the same mis-pairing that made the first version of this report the code between keys as a
# key, arriving a second time in a different disguise. It reported 25 live fields as gone.
LITERAL = re.compile(r'"((?:[^"\\]|\\.)*)"')
COMMENT = re.compile(r"//[^\n]*")

# Not every key is written as a string. A serde-derived struct serialises its FIELD NAMES, so
# preflight.rs never spells "prompt", "verdict" or "reason" and all three reach the head --
# reported as gone until this was added. Fourth blindness in this reference set, each one a
# different way the core can name a key without quoting it.
FIELD = re.compile(r"\bpub\s+(\w+)\s*:")


# A view the core serves that no model here decodes is invisible to the check above, because
# MODELS is written by hand: view.obstacle arrived in abeca9f79 and was noticed only because
# someone read the commit. These are the families this head deliberately does not decode, kept
# as patterns rather than as 26 names so the table does not rot on every new file parser.
UNDRAWN = [
    ("view.core", "the core's own link path, behind QGC_CORE_LINKS, which this head must not enable"),
    ("view.contract", "introspection, not a screen"),
    ("view.dependencies", "introspection, not a screen"),
    ("view.geoTo", "a coordinate conversion called as a function, not a view to draw"),
    ("view.nedTo", "a coordinate conversion called as a function, not a view to draw"),
    ("view.utmTo", "a coordinate conversion called as a function, not a view to draw"),
    ("view.kmlFile", "a file parser the plan store reaches through its own action"),
    ("view.shapeFile", "a file parser the plan store reaches through its own action"),
    ("view.missionFile", "a file parser the plan store reaches through its own action"),
    ("view.planFile", "a file parser the plan store reaches through its own action"),
    ("view.waypointsFile", "a file parser the plan store reaches through its own action"),
    ("view.planFromWaypoints", "a file parser the plan store reaches through its own action"),
    ("view.tlog", "a file parser the analyze window reaches through its own action"),
    ("view.terrainTile", "a tile fetch, not a screen"),
    ("view.control", "manual control, which needs a joystick this machine does not have"),
    ("view.linkForm", "link creation, which is a transient-handle API and QML-only"),
    ("view.transports", "link creation, which is a transient-handle API and QML-only"),
    ("view.vehicles", "built and unverifiable: no second vehicle has ever connected here"),
    ("view.landingPattern", "a fixed-wing or VTOL landing pattern: this SITL is a quadrotor and "
                            "arm(land) places a plain Return To Launch, so the view answers its "
                            "refusal at every index. Changing the offline vehicle type is a "
                            "settings write. PARITY GAP against six QML files, recorded not built"),
    ("view.obstacle", "a proximity ring: available is false with no vehicle, so there is nothing "
                      "to draw and nothing to check. PARITY GAP, recorded not built"),
]


def undrawn_reason(path):
    return next((why for prefix, why in UNDRAWN if path.startswith(prefix)), None)


def balanced(text, start):
    depth = 0
    for end in range(start, len(text)):
        depth += text[end] == "{"
        depth -= text[end] == "}"
        if not depth:
            return end
    return len(text) - 1


def views_to_modules():
    """view path -> core module, read from the registry rather than written here."""
    registry = (CORE / "view.rs").read_text()
    return {m.group(1): m.group(2)
            for m in re.finditer(r'View\s*\{\s*path:\s*"([^"]+)".*?compute:\s*(\w+)::',
                                 registry, re.S)}


def keys_a_model_reads(name):
    for path in sorted(SOURCES.glob("*.swift")):
        text = COMMENT.sub("", path.read_text())
        declaration = re.search(rf"\n(?:final )?(?:struct|class|enum) {name}\b", text)
        if not declaration:
            continue
        following = text[declaration.end():]
        stop = re.search(r"\n(?:final )?(?:struct|class|enum) \w+", following)
        body = following[: stop.start()] if stop else following
        keys = set()
        for init in re.finditer(r"\n    (?:public )?init\??\(", body):
            open_brace = body.find("{", init.end())
            if open_brace < 0:
                continue
            end = balanced(body, open_brace)
            keys |= {found for match in LOOKUP.finditer(body[open_brace:end])
                     for found in match.groups() if found}
        return keys
    return None


registry = views_to_modules()
gone, unchecked = [], []
for model, view in sorted(MODELS.items()):
    module = registry.get(view)
    if module is None:
        unchecked.append(f"{model}: this file names {view}, which core-rs/src/view.rs does not serve")
        continue
    source = CORE / f"{module}.rs"
    if not source.exists():
        unchecked.append(f"{model}: {view} is built by {module}, and {module}.rs does not exist")
        continue
    read = keys_a_model_reads(model)
    if not read:
        unchecked.append(f"{model}: no initialiser keys found, which is a broken reader not a clean result")
        continue
    stripped = COMMENT.sub("", source.read_text())
    emitted = set(LITERAL.findall(stripped)) | set(FIELD.findall(stripped))
    for key in sorted(read - emitted):
        gone.append((model, key, view, module))

for line in unchecked:
    print(f"  NOT CHECKED {line}", file=sys.stderr)
for model, key, view, module in gone:
    print(f"  GONE {model} reads {key!r}, which {module}.rs no longer emits for {view}",
          file=sys.stderr)

# Asking whether a view has an entry in MODELS is the wrong question and reported 17 views
# this head decodes inline in a store rather than through a listed model. What matters is
# whether the head names the path at all.
names = "\n".join(path.read_text() for path in sorted(SOURCES.glob("*.swift")))
unmodelled = [v for v in sorted(registry)
              if v not in names and undrawn_reason(v) is None]
for view in unmodelled:
    print(f"  UNREAD {view} is served by the core, is named nowhere in macos/Sources, and is "
          f"not in UNDRAWN either. Read it, or add it with the reason it stays undrawn.",
          file=sys.stderr)

checked = len(MODELS) - len(unchecked)
print(f"checked {checked} of {len(MODELS)} models against the views they decode: "
      f"{len(gone)} read a key the core no longer emits")
print(f"every one of the {len(registry)} views the core serves is either read here or listed "
      f"with the reason it is not: {len(unmodelled)} are neither")
sys.exit(1 if gone or unchecked or unmodelled else 0)
