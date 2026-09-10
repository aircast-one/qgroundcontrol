#!/usr/bin/env python3
"""Every literal bridge path in macos/Sources must resolve against a running app.

A path is a string the head hands to a reflection lookup. A typo does not raise:
it resolves to nothing and every decoder turns that into its default, which the
fail-open sweep showed is sometimes the permissive direction. This checks the
paths that can be checked.

vehicle.* is EXCLUDED and cannot be included: with no vehicle connected those
resolve to null, which is exactly what a typo returns. Distinguishing them needs
an aircraft. Run this again with one and drop the exclusion.

Usage: QGC_PORT=8777 python3 tools/macos/bridge-paths.py
"""
import json, os, pathlib, re, sys, urllib.request

PORT = os.environ.get("QGC_PORT", "8777")
ROOT = pathlib.Path(__file__).resolve().parents[2]
CALL = re.compile(r'Bridge\.(group|json)\(\s*"([^"]+)"')
# The bridge answers an invented ROOT with {"kind":"null"} - detectable. It answers a
# typo'd PROPERTY on a real object with {"kind":"value","value":null}, which is exactly
# what a real property that happens to be null returns. Those two cannot be told apart
# here, so they are reported as unverified rather than counted as passing.
RESOLVES = {"object", "fact", "coordinate", "list"}

def answer(path):
    req = urllib.request.Request(f"http://127.0.0.1:{PORT}/bridge/get?path={path}",
                                 headers={"X-QGC-Debug-Api": "1"})
    j = json.loads(urllib.request.urlopen(req, timeout=5).read())
    k = j.get("kind", "?")
    if k == "value":
        return "value" if j.get("value") is not None else "value:null"
    return k

reads = {}
for p in sorted((ROOT / "macos/Sources").glob("*.swift")):
    for i, line in enumerate(p.read_text().splitlines(), 1):
        for m in CALL.finditer(line):
            path = m.group(2)
            if path.startswith("view.") or "\\(" in path or path.startswith("vehicle"):
                continue
            reads.setdefault(path, f"{p.name}:{i}")

try:
    answer("plan")
except Exception:
    sys.exit(f"no app answering on {PORT}; start one with tools/macos/build-run.sh")

bad, unsure = [], []
for path, where in sorted(reads.items()):
    k = answer(path)
    if k in RESOLVES or k == "value":
        continue
    (unsure if k == "value:null" else bad).append((k, path, where))

print(f"checked {len(reads)} literal read paths: "
      f"{len(reads) - len(bad) - len(unsure)} resolved, {len(bad)} BROKEN, "
      f"{len(unsure)} unverifiable")
for k, p, w in bad:
    print(f"  BROKEN       {p}  ({w}) - no such root")
for k, p, w in unsure:
    print(f"  unverifiable {p}  ({w}) - null property; a typo looks identical")
sys.exit(1 if bad else 0)
