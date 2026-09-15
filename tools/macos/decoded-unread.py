import pathlib, re, collections, sys
sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
from sweepguard import anchored, refuse
from importlib.machinery import SourceFileLoader
sweep = SourceFileLoader('sweep', str(anchored('tools/macos/orphan-sweep.py')))

SRC = sorted(anchored('macos/Sources').glob('*.swift'))
refuse(sources=SRC)
raw = {p.name: p.read_text() for p in SRC}

def code_only(text):
    out, i, n = [], 0, len(text)
    while i < n:
        c = text[i]
        if c == '/' and text[i:i+2] == '//':
            i = text.find('\n', i)
            if i < 0: break
        elif c == '/' and text[i:i+2] == '/*':
            end = text.find('*/', i + 2)
            i = n if end < 0 else end + 2
        elif c == '"':
            i += 1
            while i < n and text[i] != '"':
                if text[i] == '\\':
                    if text[i:i+2] == '\\(':
                        depth, i = 1, i + 2
                        while i < n and depth:
                            if text[i] == '(': depth += 1
                            elif text[i] == ')': depth -= 1
                            if depth: out.append(text[i])
                            i += 1
                        out.append(' ')
                        continue
                    i += 2
                    continue
                i += 1
            i += 1
        else:
            out.append(c)
            i += 1
    return ''.join(out)

code = {k: code_only(v) for k, v in raw.items()}

# A stored field DECODED FROM THE CORE: assigned in an init from a json subscript.
assign = re.compile(r'^\s*(?:self\.)?([a-z]\w*)\s*=\s*.*\bjson\[')
stored = re.compile(r'^\s*(?:private\(set\)\s+)?(?:let|var)\s+([a-z]\w*)\s*:')
type_re = re.compile(r'^\s*(?:public\s+|final\s+)*(?:struct|class|enum)\s+([A-Za-z_]\w*)')

fields = {}
for p in SRC:
    cur = '(file)'
    declared = {}
    for line in raw[p.name].splitlines():
        m = type_re.match(line)
        if m: cur = m.group(1); continue
        d = stored.match(line)
        if d: declared[d.group(1)] = cur
        a = assign.match(line)
        if a and a.group(1) in declared:
            fields[(p.name, declared[a.group(1)], a.group(1))] = 0

rows = []
for (file, owner, name) in fields:
    rx = re.compile(r'\.' + re.escape(name) + r'\b')
    hits = collections.Counter()
    for f, t in code.items():
        n = len(rx.findall(t))
        if n: hits[f] = n
    bare = re.compile(r'(?<![.\w])' + re.escape(name) + r'\b')
    # inside the declaring file a sibling property reads it unqualified
    own = len(bare.findall(code[file]))
    rows.append((sum(hits.values()), own, file, owner, name, hits))

# Every one of these was reported, read, and SETTLED. Before this table those decisions lived only
# in a loop prompt and in commit messages, so each run re-surfaced eight findings with no record of
# which were already answered -- and 1d73e4db0 is what happens when a reason to dismiss sits
# somewhere nothing checks it. The guard below fails if an entry stops matching a decoded-and-unread
# field, so the table cannot outlive the code it excuses.
ACCEPTED = {
    ("AdsbContact", "squawk"): "CONSUMED, not orphaned. adsb.rs:211 turns a squawk into the "
        "emergency token the panel already draws, so the meaningful part is extracted upstream. "
        "Four digits of raw transponder code on every contact row is jargon, not an answer.",
    ("ObstacleReading", "sector"): "A WIRE TOKEN. The core serves \"aheadRight\" here and the "
        "cooked words in sectorText, which the panel draws. A head that renders the raw id puts "
        "wire vocabulary in front of an operator -- the same call as the emergency token above.",
    ("ObstacleReading", "bearing"): "The degrees behind that sector. Drawing both is two ways to "
        "say one fact, which is how a header and a label end up disagreeing.",
    ("Kind", "at"): "HostNotice's arrival time, epoch ms from QGCHostNotices.cc:90. Drawing it "
        "needs either a clock time -- locale-dependent, and this head leaves locale spelling to "
        "the core -- or a relative age, which rides a poll that STOPS when the window closes, so "
        "\"2m ago\" would freeze. Both are design decisions, not a gap closed by wiring a field.",
    ("JoystickSetting", "enumValues"): "NO JOYSTICK PANEL BY DESIGN. JoystickModel.swift's header "
        "says the decoder exists to pin the catalogue's rules under swift-checks until the live "
        "half is served; a panel on the catalogue alone would draw a settings schema with no "
        "values beside it and no device to apply them to.",
    ("JoystickFunction", "rcChannel"): "Same: no joystick panel by design.",
    ("JoystickAction", "repeats"): "Same: no joystick panel by design.",
    ("JoystickMapping", "transmitterModes"): "Same: no joystick panel by design.",
}

rows.sort(key=lambda r: (r[1], r[0]))
print("DECODED FROM THE CORE, RANKED BY HOW LITTLE READS THEM.")
print()
print("THE COUNT BELOW IS A FLOOR, NEVER A COUNT. A field is cleared as read on a qualified")
print("`.name` match anywhere in Sources, and that match cannot tell one type's member from")
print("another's -- AdsbContact.alert read as used for months because SwiftUI has .alert().")
print("An audit asking how many cleared fields have NO reader file that even names the owning")
print("type returned 122 of 333. That number convicts nothing (a reader normally holds the type")
print("in a variable of some other name) and it exonerates nothing either. What it establishes")
print("is that this tool was validated on ONE case -- FleetVehicle.active -- and one case proves")
print("nothing about a sweep. Fields it lists are worth reading; fields it omits are not cleared.")
print("A COUNT IS NOT A VERDICT: `.name` collides across types -- AdsbContact.alert reads as")
print("used because SwiftUI's .alert() modifier exists, and that is why the orphan sweep never")
print("surfaced it. Low rows are CANDIDATES TO READ, never findings. The `own` column counts")
print("unqualified uses in the declaring file, which includes the init that assigns it.")
print(f"\n{len(rows)} decoded fields across {len({r[2] for r in rows})} files\n")
unread = {(owner, name) for total, own, file, owner, name, hits in rows if total == 0 and own <= 2}
dead = [r for r in rows if r[0] == 0 and r[1] <= 2 and (r[3], r[4]) not in ACCEPTED]
for total, own, file, owner, name, hits in dead:
    print(f"  {owner+'.'+name:38s} {file.replace('.swift','')}")

stale = [f"{owner}.{name} is accepted here and is no longer decoded-and-unread -- either it grew a "
         f"reader or it went away, and the reason has outlived the code"
         for owner, name in sorted(ACCEPTED) if (owner, name) not in unread]
for why in stale:
    print(f"  STALE ACCEPTANCE {why}")
print(f"\n{len(dead)} of {len(rows)} decoded fields are read by NOTHING and unaccounted for; "
      f"{len(ACCEPTED)} more are read by nothing and ACCEPTED with a reason above. No qualified `.name`")
print("anywhere, and own=2 is the declaration plus the init line that assigns it. own>2 means a")
print("sibling in the same file reads it unqualified -- FleetVehicle.active sits at own=3 because")
print("listTitle reads it, which is the fix that put it there.")
