import re, pathlib, collections, json

ROOT = pathlib.Path("src")
files = sorted(ROOT.rglob("*.qml"))
by_name = {}
for f in files:
    by_name.setdefault(f.stem, []).append(f)

lines = {f: sum(1 for _ in f.open(errors="ignore")) for f in files}

# component references: a bare Identifier used as a type or attached property
ident = re.compile(r"\b([A-Z][A-Za-z0-9_]*)\b")
refs = {}
for f in files:
    text = f.read_text(errors="ignore")
    text = re.sub(r"//[^\n]*", "", text)
    text = re.sub(r"/\*.*?\*/", "", text, flags=re.S)
    found = set()
    for name in set(ident.findall(text)):
        if name != f.stem and name in by_name:
            found.update(by_name[name])
    refs[f] = found

PHASES = [
    ("1 Settings", ["UI/AppSettings"]),
    ("2 Analyze",  ["AnalyzeView", "Viewer3D"]),
    ("3 Setup",    ["AutoPilotPlugins", "Vehicle/VehicleSetup"]),
    ("4 Plan",     ["QmlControls/PlanView.qml", "QmlControls/GeoFenceEditor.qml",
                    "QmlControls/MissionItemEditor.qml", "FlightMap"]),
    ("5 Fly",      ["FlightDisplay", "UI/toolbar", "UI/MainWindow.qml",
                    "UI/AndroidHost.qml", "UTMSP", "FirstRunPromptDialogs"]),
]

def seeds(pats):
    out = set()
    for f in files:
        s = str(f)
        for p in pats:
            if s.endswith(p) or f"/{p}/" in s or s.startswith(f"src/{p}"):
                out.add(f)
    return out

reach = {}
for label, pats in PHASES:
    frontier, seen = seeds(pats), set()
    while frontier:
        cur = frontier.pop()
        if cur in seen: continue
        seen.add(cur)
        frontier |= refs.get(cur, set()) - seen
    reach[label] = seen

labels = [l for l, _ in PHASES]
owner = {}
for f in files:
    users = [l for l in labels if f in reach[l]]
    owner[f] = users[-1] if users else "unreached"

tot = collections.Counter()
cnt = collections.Counter()
for f in files:
    tot[owner[f]] += lines[f]; cnt[owner[f]] += 1

print(f"{'phase it can be deleted in':<28} {'files':>6} {'lines':>8}")
for l in labels + ["unreached"]:
    print(f"{l:<28} {cnt[l]:>6} {tot[l]:>8}")
print(f"{'TOTAL':<28} {sum(cnt.values()):>6} {sum(tot.values()):>8}")

shared = [(l, [f for f in files if owner[f] == l and sum(f in reach[x] for x in labels) > 1]) for l in labels]
print("\nfiles held hostage to a later phase (used by >1 view group):")
for l, fs in shared:
    if fs:
        print(f"  {l}: {len(fs)} files, {sum(lines[f] for f in fs)} lines")

print("\nunreached (no view group references these):")
for f in files:
    if owner[f] == "unreached" and lines[f] > 100:
        print(f"   {lines[f]:>5}  {f}")
