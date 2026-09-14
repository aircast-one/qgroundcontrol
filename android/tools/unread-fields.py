import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parents[2]
TREES = [ROOT / "android/app/src/main", ROOT / "android/map-spike/src/main"]

KNOWN = ("canPhoto", 1)

ACCEPTED = {
    "LogsView.eraseWarning": "MEASURED and deliberately not drawn. The core serves 'This erases "
        "every log on the vehicle.' and the dialog says 'This permanently deletes every log on the "
        "vehicle. It cannot be undone.' - the head's wording is the stronger one because it names "
        "the irreversibility, which is the fact an operator needs before a destructive action. "
        "Adopting the served sentence would lose that",
}


def corpus():
    return "\n".join(p.read_text() for tree in TREES for p in tree.rglob("*.kt"))


def properties(text):
    for found in re.finditer(r"(?:internal )?data class (\w+)\((.*?)\n\)", text, re.S):
        for name in re.findall(r"val (\w+):", found.group(2)):
            yield found.group(1), name


def dereferenced(body, name):
    return len(re.findall(r"\." + name + r"\b", body)) > 0


def named_bare(text, cls, name):
    inside = re.search(r"data class " + cls + r"\(.*?\n\)(.*?)(?=\ninternal |\n@|\Z)", text, re.S)
    return bool(inside and re.search(r"\b" + name + r"\b", inside.group(1)))


def main():
    body = corpus()
    seen = len(re.findall(r"\." + KNOWN[0] + r"\b", body))
    if seen < KNOWN[1]:
        raise SystemExit(f"instrument broken: {KNOWN[0]} should appear at least {KNOWN[1]} time(s), saw {seen}")
    found = [
        (f"{cls}.{name}", f"{path.name} {cls}.{name}")
        for tree in TREES
        for path in sorted(tree.rglob("*.kt"))
        for cls, name in properties(path.read_text())
        if not dereferenced(body, name) and not named_bare(path.read_text(), cls, name)
    ]
    dead = [entry for key, entry in found if key not in ACCEPTED]
    for entry in dead:
        print(f"  {entry}")
    print(f"{len(dead)} decoded field(s) nothing reads, {len(found) - len(dead)} accepted with a reason")
    return 1 if dead else 0


if __name__ == "__main__":
    sys.exit(main())
