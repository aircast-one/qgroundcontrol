import pathlib, re, collections

SRC = sorted(pathlib.Path('macos/Sources').glob('*.swift'))
TST = sorted(pathlib.Path('macos/Tests').glob('*.swift'))
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

src_text = {p.name: p.read_text() for p in SRC}
tst_text = {p.name: p.read_text() for p in TST}
src_code = {k: code_only(v) for k, v in src_text.items()}
tst_code = {k: code_only(v) for k, v in tst_text.items()}

decl_re = re.compile(
    r'^\s*(?:@\w+\s+)*(?:public\s+|private\s+|fileprivate\s+|internal\s+)?'
    r'(?:private\(set\)\s+)?(?:static\s+)?(?:final\s+)?'
    r'(func|var|let|case)\s+([A-Za-z_]\w*)')
type_re = re.compile(r'^\s*(?:public\s+|final\s+)*(struct|class|enum|extension|protocol)\s+([A-Za-z_]\w*)')

decls = collections.defaultdict(list)
for p in SRC:
    cur = '(file)'
    for i, line in enumerate(src_text[p.name].splitlines(), 1):
        m = type_re.match(line)
        if m: cur = m.group(2); continue
        d = decl_re.match(line)
        if d: decls[d.group(2)].append((p.name, i, cur, d.group(1)))

def total(name, texts):
    rx = re.compile(r'\b' + re.escape(name) + r'\b')
    return sum(len(rx.findall(t)) for t in texts.values())

rows = []
for name, ds in decls.items():
    uses = total(name, src_code) - len(ds)      # subtract the declarations themselves
    rows.append((name, ds, uses, total(name, tst_code)))

print("counts are REFERENCES IN CODE, never reachability: comments and string literals are")
print("stripped (interpolated \\(...) segments kept), but a symbol read only by another symbol")
print("nothing calls still counts as used -- velocityText read `velocity` for months that way.")
print("declared names:", len(rows))
print("declared in >1 type (the class my old sweep hid):", len([r for r in rows if len({d[2] for d in r[1]})>1]))
zero = [r for r in rows if r[2] == 0]
print("zero uses in Sources:", len(zero))
print()
for name, ds, s, t in sorted(zero, key=lambda r: (r[3], r[0])):
    where = "; ".join(f"{d[2]}.{name}({d[0]}:{d[1]})" for d in ds)
    print(f"  {name:26s} tests={t:<3d} {where[:96]}")
