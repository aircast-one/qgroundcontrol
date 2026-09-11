#!/usr/bin/env python3
"""Every literal bridge path in macos/Sources must resolve, and every watched dep must bind.

Two questions about the same subject, both of which fail silently. A path that does not
resolve reads back as a default. A path that resolves but never BINDS is worse: the watcher
falls back to a re-read that runs only while the event loop is idle and dedupes on the
serialised value, so the view goes stale exactly when the loop is busy and reports nothing.
The binding half needs a window open, because a head with no window registers no watches.

A path is a string the head hands to a reflection lookup. A typo does not raise:
it resolves to nothing and every decoder turns that into its default, which the
fail-open sweep showed is sometimes the permissive direction.

A misspelled property used to be undetectable from here, because it answered the
same bytes as a property that happens to be null. The bridge now sends found:false
for a name it cannot resolve, so both classes are caught and there is no longer an
unverifiable third answer. Measured on a running app:

    inventedRoot          -> {"kind":"null"}
    plan.dirtyy           -> {"found":false,"kind":"value","value":null}
    links.failedLink      -> {"kind":"value","value":null}      (real, and null)

vehicle.* is STILL EXCLUDED, and found:false does not help: with no vehicle the
root itself answers kind:null, so vehicle.armed and vehicle.inventedThing are
byte-identical. That one needs an aircraft.

Usage: QGC_PORT=8777 python3 tools/macos/bridge-paths.py
"""
import json, os, pathlib, re, sys, urllib.request

PORT = os.environ.get("QGC_PORT", "8777")
ROOT = pathlib.Path(__file__).resolve().parents[2]
CALL = re.compile(r'Bridge\.(group|json)\(\s*"([^"]+)"')
# A watched path is looked up the same way a read is, so a typo in one dies the same
# silent death -- except a watch cannot be caught by eye afterwards: it simply never
# fires, which is indistinguishable from a value that never changed.
WATCH = re.compile(r'BridgeWatch\.watch\([^,]+,\s*\[([^\]]*)\]')
WATCH_CALL = re.compile(r'BridgeWatch\.watch\(')
LITERAL = re.compile(r'"([^"]+)"')
RESOLVES = {"object", "fact", "coordinate", "list"}

def answer(path):
    req = urllib.request.Request(f"http://127.0.0.1:{PORT}/bridge/get?path={path}",
                                 headers={"X-QGC-Debug-Api": "1"})
    j = json.loads(urllib.request.urlopen(req, timeout=5).read())
    if j.get("found") is False:
        return "no such property"
    k = j.get("kind", "?")
    return "value" if k == "value" else k

reads = {}
unparsed = []
for p in sorted((ROOT / "macos/Sources").glob("*.swift")):
    for i, line in enumerate(p.read_text().splitlines(), 1):
        for m in CALL.finditer(line):
            path = m.group(2)
            if path.startswith("view.") or "\\(" in path or path == "vehicle" or path.startswith("vehicle."):
                continue
            reads.setdefault(path, f"{p.name}:{i}")
        if WATCH_CALL.search(line) and not WATCH.search(line):
            unparsed.append(f"{p.name}:{i}")
        for m in WATCH.finditer(line):
            for path in LITERAL.findall(m.group(1)):
                # A watch path may name a Qt signal as root.path@signalName; the signal
                # half is not a property and only the object half can be resolved here.
                path = path.split("@")[0]
                if "\\(" in path or path.startswith("vehicle."):
                    continue
                reads.setdefault(path, f"{p.name}:{i}")

try:
    answer("plan")
except Exception:
    sys.exit(f"no app answering on {PORT}; start one with tools/macos/build-run.sh")

bad = []
for path, where in sorted(reads.items()):
    k = answer(path)
    if k in RESOLVES or k == "value":
        continue
    bad.append((k, path, where))

print(f"checked {len(reads)} literal read paths: "
      f"{len(reads) - len(bad)} resolved, {len(bad)} BROKEN")
# A watch whose paths are built by a helper rather than an array literal cannot be read from
# here. Saying so is the point: a pinner that covers four of five call sites silently is worse
# than one that covers four and names the fifth. But say it accurately -- the one call site
# this misses builds plan.missionController.visualItems.<n>.<property>, and those property
# names ARE pinned, by interpolated-names.py against the two headers that declare them. Only
# the path prefix is unchecked. Reporting that as flatly "not pinned" sends the next reader
# looking for a hole that is mostly already plugged.
for w in unparsed:
    print(f"  PARTLY PINNED {w} - built by a helper; leaf names pinned by "
          f"interpolated-names.py, the path prefix by nothing")


# The core reports whether each resolved dep bound. It does not report WHY one did not, and
# the two reasons need opposite responses: a dep whose object does not exist yet rebinds by
# itself (Watcher::_poll calls _bind again every tick), while a dep that resolves and still
# does not bind has no notify signal and never will. Asking the bridge whether the path
# resolves is what separates them -- an earlier version exempted everything under vehicle.*,
# which would have explained away, forever, whatever unbindable dep happens to live under that
# prefix. geoFenceController.polygons is the shape to keep in mind: declared
# Q_PROPERTY(QmlObjectListModel* polygons READ polygons CONSTANT), so it resolves perfectly
# well and binds to nothing, because Qt gives a CONSTANT property no notify signal.
def watch_bindings():
    req = urllib.request.Request(f"http://127.0.0.1:{PORT}/native/probe?id=mission",
                                 headers={"X-QGC-Debug-Api": "1"})
    state = json.loads(urllib.request.urlopen(req, timeout=10).read())["state"]
    reported = state.get("watchBindings", {})
    return reported.get("bound", 0), reported.get("unbound", [])

unbound_broken = []
asked = True
bound, unbound = 0, []
try:
    bound, unbound = watch_bindings()
except Exception as error:
    print(f"  watch bindings NOT CHECKED - {error}")
    asked = False

if not asked:
    pass
elif bound == 0 and not unbound:
    # No window open means no watches at all, and an empty answer would read exactly like a
    # clean one.
    print("  watch bindings NOT CHECKED - no watches registered; open the Plan window first")
else:
    resolves = {p: answer(p) in RESOLVES or answer(p) == "value" for p in unbound}
    waiting = [p for p in unbound if not resolves[p]]
    unbound_broken = [p for p in unbound if resolves[p]]
    print(f"checked {bound + len(unbound)} watched deps: {bound} bound, "
          f"{len(waiting)} not resolvable yet, {len(unbound_broken)} BINDING TO NOTHING")
    for p in waiting:
        print(f"  waiting {p} - resolves to nothing yet; _poll rebinds when it appears")
    for p in unbound_broken:
        print(f"  UNBOUND {p} - resolves but binds to no signal; rides the idle-only poll")
for k, p, w in bad:
    why = "no such root" if k == "null" else k
    print(f"  BROKEN {p}  ({w}) - {why}")
sys.exit(1 if bad or unbound_broken else 0)
