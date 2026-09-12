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

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
from head_models import MODELS, views

ROOT = pathlib.Path(__file__).resolve().parents[2]
SOURCES = ROOT / "macos/Sources"
CORE = ROOT / "core-rs/src"


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


# A regex cannot strip Rust line comments, because "//" occurs INSIDE string literals -- an SSE
# URL, an rtsp:// fixture. Cutting there deletes the rest of the line INCLUDING the closing quote,
# and every later quote pairs off by one: one poisoned line in detections.rs swallowed 1861
# characters as a single "literal" and hid stale, error and confidence. The instrument then
# reported keys as GONE that the core still serves, and could not have seen a real removal after
# that line either. Both directions, from one stray "//".
def strip_line_comments(text):
    out, index, length = [], 0, len(text)
    while index < length:
        char = text[index]
        if char == "r" and text.startswith(('r"', 'r#'), index):
            hashes = len(text[index + 1:]) - len(text[index + 1:].lstrip("#"))
            close = '"' + "#" * hashes
            end = text.find(close, index + 2 + hashes)
            end = length if end < 0 else end + len(close)
            out.append(text[index:end]); index = end
        elif char == '"':
            end = index + 1
            while end < length and text[end] != '"':
                end += 2 if text[end] == "\\" else 1
            out.append(text[index:min(end + 1, length)]); index = end + 1
        elif text.startswith("//", index):
            end = text.find("\n", index)
            index = length if end < 0 else end
        else:
            out.append(char); index += 1
    return "".join(out)

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
    ("view.core", "A PREFIX, NOT A PATH -- it silences SEVEN registered views at once: "
                  "view.coreVehicle, coreGuided, coreParameter, coreParameters, coreMission, "
                  "coreRemoteId and coreCalibration. All sit behind QGC_CORE_LINKS, which this "
                  "head must not enable, so the suppression is right; the entry read as one view "
                  "and silenced seven, which is a partial enumeration wearing a singular noun"),
    ("view.control", "the SAME Control shape arrives NESTED inside the settings and parameter "
                     "page views, which this head does read and decode with SettingsControl. The "
                     "standalone path is for a head that renders one control at a time. Measured: "
                     "a control inside view.settings(General) carries every key view.control "
                     "serves, restartNotices included. The reason here used to read 'manual "
                     "control, which needs a joystick this machine does not have', which was "
                     "simply wrong -- it is a FACT control, and that false reason excused the "
                     "view from scrutiny for as long as it stood"),
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
    ("view.linkForm", "link creation, which is a transient-handle API and QML-only"),
    ("view.transports", "NOT link creation -- that reason belonged to view.linkForm above and was "
                        "copied here. transports_view returns a live INVENTORY: openCount, "
                        "qtOpenCount and a links array, measured at 141 entries with 1 open. It is "
                        "the core's own transport registry joined with the Qt links, and it sits "
                        "behind QGC_CORE_LINKS for the core half. This head draws its link list "
                        "from view.links (Links.swift), which is the view that answers the "
                        "question an operator asks"),
    ("view.vehicles", "built and unverifiable: no second vehicle has ever connected here"),
    ("view.landingPattern", "a fixed-wing or VTOL landing pattern: this SITL is a quadrotor and "
                            "arm(land) places a plain Return To Launch, so landing_view's own "
                            "isSimpleItem guard refuses every item this rig can produce. Changing "
                            "the offline vehicle type is a settings write, which is forbidden "
                            "here, so the state cannot be reached at all rather than merely being "
                            "unmeasured. PARITY GAP, recorded not built: QML carries an EDITOR and "
                            "a MAP VISUAL for each of fixed-wing and VTOL. Counted as six once and "
                            "re-measured to four -- the fifth QML hit is PlanViewSettings, which "
                            "only holds the allowMultipleLandingPatterns toggle and draws no "
                            "pattern. Named by their shape rather than their number because a "
                            "count in a reason rots the moment a file is added"),
    ("view.cameraProtocol", "MEASURED: a PROTOCOL VOCABULARY, not live state -- action names, "
                            "capabilities, retry attempts and delays, level types, refusal and "
                            "result strings, storage and video status enumerations, staleMs. It "
                            "describes what a camera command IS and what can come back, for a "
                            "head that dispatches them. This head draws live camera state from "
                            "view.camera, which it reads, and has no surface built on the "
                            "vocabulary. Every action it names -- takePhoto, startRecording, "
                            "setMode, zoom, formatStorage -- is one this session must not send, "
                            "so even the gating could not be exercised"),
    ("view.cameraDefinition", "a file parser: it refuses with 'this needs the path of a camera "
                              "definition file, and optionally a locale'. Same class as "
                              "view.kmlFile and view.shapeFile, which a store reaches through "
                              "its own action rather than drawing"),
    ("view.joystickMapping", "MEASURED: an axis and button MAPPING SCHEMA -- four required "
                             "functions, six additional axes with their extension bits and RC "
                             "channels, four transmitter modes, the RC-override channel layout, "
                             "and nineteen setting definitions with types and ranges. It is "
                             "reference data for a head that CONFIGURES a joystick. This head "
                             "has no joystick surface at all: zero files in macos/Sources "
                             "mention one. PARITY GAP against QGC's joystick setup page, "
                             "recorded not built"),
    ("view.gimbal", "MEASURED with no vehicle: available false, count 0, gimbals empty, "
                    "discovery idle, ready false. A gimbal is discovered from a connected "
                    "vehicle that has one, and this SITL is a bare quadrotor -- so every field "
                    "worth drawing is absent and a head built against this payload would be "
                    "built against nothing. There is also no gimbal surface in this head to "
                    "put it on. PARITY GAP against QGC's own gimbal controls, recorded not "
                    "built"),
    ("view.followMe", "MEASURED: reason 'noVehicles', enabled false, fixValid null. Follow-me "
                      "makes the AIRCRAFT FOLLOW THE GROUND STATION, so exercising it needs a "
                      "vehicle and then commands it to move -- which this session must not do. "
                      "The view is readable but every state that distinguishes it from silence "
                      "requires the one action that is forbidden. PARITY GAP, recorded not "
                      "built"),
    ("view.adsbTraffic", "MEASURED: available true but connected false, enabled false, count 0, "
                         "contacts empty. Traffic comes from an ADSB receiver or a MAVLink feed "
                         "and this rig has neither, and turning it on is a SETTINGS WRITE, "
                         "which is forbidden here. So the list can only ever be empty and a "
                         "contacts layer could not be seen to work. PARITY GAP, recorded not "
                         "built"),
    ("view.debugApi", "INTROSPECTION, and pointed the other way: it answers count 10 and a "
                      "hostRoutes list naming /native/windows, /native/menu, /native/probe and the "
                      "rest -- routes THIS HEAD ITSELF SERVES, each with the reason it exists. The "
                      "core is modelling the test surface the head provides, so drawing it would "
                      "be the app showing an operator its own debugging plumbing. Same class as "
                      "view.contract and view.dependencies above"),
    ("view.packetRadio", "MEASURED as a REFUSAL, which is the answer: it wants a status token "
                         "(disabled, noAdapter, adapterUnavailable, invalidKey, listening, "
                         "receiving), then the adapter name, per-antenna raw rssi, snr and link "
                         "score written a/b, packets lost in the last second, the video packet "
                         "count and the host's start error. A FORMATTER for a wfb-ng link, same "
                         "class as the file parsers -- it spells figures a caller already holds "
                         "rather than answering live state. Nothing in this head holds them: there "
                         "is no packet-radio surface and no adapter on this machine"),
    ("view.orbit", "MEASURED: available false, orbiting null, reason \"No vehicle is connected.\" "
                   "Vehicle-gated like the rest -- but this one is ALSO WITHHELD ON PURPOSE. "
                   "Starting an orbit is forbidden in this session, and the probe hook for it was "
                   "deliberately built incapable of the write, so a control could be drawn and "
                   "never once exercised to see it work. The reading half, whether the vehicle is "
                   "orbiting, needs a vehicle that is flying one. Recorded not built, and the "
                   "reason is the session's own safety rule rather than a gap in the core"),
    ("view.geoTag", "MEASURED as a REFUSAL, which is the answer: it asks for the path of a "
                    "telemetry log, a tolerance and one epoch timestamp per image. It is the "
                    "tagging COMPUTATION, not the controller state the Analyze page draws -- "
                    "logFile, imageDirectory, saveDirectory, progress, inProgress. raw-reads.py "
                    "flags GeoTag.swift for reading 'geoTag' straight from Qt while a view names "
                    "the same subject, and its own question -- ask what that view refuses -- has "
                    "the answer here: it refuses everything except the tagging run. Same class as "
                    "the file parsers above, and the head reaches it through its own action"),
    ("view.videoSource", "MEASURED as a REFUSAL: it asks for a source token, optionally a url and "
                         "an rtsp timeout. A PARSER, same class as the file parsers. The head's "
                         "VideoSource model is a settings-slot list -- slot, name, url, enabled, "
                         "configured -- and shares only the NAME, which is why this checker's "
                         "filename heuristic paired VideoSourceModel.swift with it under UNLISTED. "
                         "A shared word is not a shared subject"),
    ("view.gpsRtkBase", "MEASURED with no vehicle and it answers REAL STATE: driver ublox, mode "
                        "surveyIn, accuracy 2.0 m, minimum duration 180 s, fixedBase null. An RTK "
                        "base is ground equipment, so unlike the vehicle-gated entries this one "
                        "could be built and seen to work here. PARITY GAP, recorded not built -- "
                        "the core began serving it tonight"),
    ("view.operatorControl", "MEASURED now the binary carries it, replacing the placeholder that "
                             "said NOT YET MEASURABLE: available false, known false, inControl, "
                             "holderSystemId, takeoverAllowed and systemManager all null, reason "
                             "\"No vehicle is connected.\" It answers which ground station is "
                             "flying the vehicle and whether it will hand over, which needs a "
                             "vehicle AND A SECOND GROUND STATION -- this rig has neither, so "
                             "every field would stay null however long it ran. QML DOES have an "
                             "original, src/UI/toolbar/GCSControlIndicator.qml, so this is a "
                             "PARITY GAP rather than a core-only capability. Half of it is a "
                             "reading and drawable; the other half sends requestOperatorControl "
                             "to acquire control or allow takeover, which is a vehicle command "
                             "forbidden here -- so those buttons could be built and never once "
                             "pressed to see them work. Recorded not built"),
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
    defines = {match.group(1): source.stem
               for source in sorted(CORE.glob("*.rs"))
               for match in re.finditer(r"\bfn (\w+_view)\b", source.read_text())}
    found = {}
    for entry in re.finditer(
            r'View\s*\{\s*path:\s*"([^"]+)"[^}]*?compute:\s*([A-Za-z_]+)(?:::(\w+))?',
            registry, re.S):
        path, first, second = entry.groups()
        found[path] = first if second else defines.get(first, first)
    return found


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
    modules = [registry.get(one) or registry.get(one.split("(")[0]) for one in views(view)]
    if any(module is None for module in modules):
        unchecked.append(f"{model}: this file names {view}, which core-rs/src/view.rs does not serve")
        continue
    sources = [CORE / f"{module}.rs" for module in modules]
    if any(not source.exists() for source in sources):
        unchecked.append(f"{model}: {view} is built by {modules}, and one of those .rs files does not exist")
        continue
    read = keys_a_model_reads(model)
    if not read:
        unchecked.append(f"{model}: no initialiser keys found, which is a broken reader not a clean result")
        continue
    emitted = set()
    for source in sources:
        stripped = strip_line_comments(source.read_text())
        emitted |= set(LITERAL.findall(stripped)) | set(FIELD.findall(stripped))
    for key in sorted(read - emitted):
        gone.append((model, key, view, "/".join(modules)))

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

unlisted = []
for source in sorted(SOURCES.glob("*Model*.swift")):
    stem = source.name.replace("Model.swift", "").replace(".swift", "")
    served = "view." + stem[0].lower() + stem[1:]
    declared = set(re.findall(r"struct ([A-Z][A-Za-z]*)", source.read_text()))
    # The pairing is a FILENAME heuristic, so it asserts a shared subject from a shared word.
    # Measured twice and wrong both times: GeoTagModel holds the controller's job state while
    # view.geoTag is the tagging computation, and VideoSource is a settings-slot list while
    # view.videoSource is a parser. A view carrying an UNDRAWN reason is an explicit statement
    # that this head does not read it, and that outranks a guess made from a name.
    if served in registry and not declared & set(MODELS) and not undrawn_reason(served):
        unlisted.append((source.name, served))
for name, served in unlisted:
    print(f"  UNLISTED {name} decodes {served}, which the core serves, and no struct in it is in "
          f"MODELS - so neither this checker nor null-fallbacks.py has ever read it. MODELS is "
          f"written by hand and nothing else notices an omission: FlyState, GuidedOffer and "
          f"ModeSlot(s) sat outside it, and ModeSlot's keys were pinned by nothing at all, "
          f"because view.modeSlots.slots records as empty with no vehicle connected.",
          file=sys.stderr)

checked = len(MODELS) - len(unchecked)
print(f"checked {checked} of {len(MODELS)} models against the views they decode: "
      f"{len(gone)} read a key the core no longer emits")
print(f"every one of the {len(registry)} views the core serves is either read here or listed "
      f"with the reason it is not: {len(unmodelled)} are neither")
sys.exit(1 if gone or unchecked or unmodelled or unlisted else 0)
