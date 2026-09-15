import pathlib, re, collections, sys
sys.path.insert(0, str(pathlib.Path('tools/macos')))
from importlib.machinery import SourceFileLoader
sweep = SourceFileLoader('sweep', 'tools/macos/orphan-sweep.py')

SRC = sorted(pathlib.Path('macos/Sources').glob('*.swift'))
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
dead = [r for r in rows if r[0] == 0 and r[1] <= 2]
for total, own, file, owner, name, hits in dead:
    print(f"  {owner+'.'+name:38s} {file.replace('.swift','')}")
print(f"\n{len(dead)} of {len(rows)} decoded fields are read by NOTHING: no qualified `.name`")
print("anywhere, and own=2 is the declaration plus the init line that assigns it. own>2 means a")
print("sibling in the same file reads it unqualified -- FleetVehicle.active sits at own=3 because")
print("listTitle reads it, which is the fix that put it there.")
