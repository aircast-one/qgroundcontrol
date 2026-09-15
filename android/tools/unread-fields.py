import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parents[2]
TREES = [ROOT / "android/app/src/main", ROOT / "android/map-spike/src/main"]

KNOWN = ("canPhoto", 1)
KNOWN_CLASS = ("Instrument", ["label", "reading"])

ACCEPTED = {
    "InspectorMessage.compId": "MEASURED and already inside the title. inspector.rs folds it in as "
        "'HEARTBEAT (comp 1)' when a message name repeats across components, and the screen draws "
        "title. Reading the raw number as well would be this head re-deciding when the distinction "
        "matters, which is the judgement the core made",
    "CalibrationSide.key": "MEASURED and unused because the list is not keyed. SensorsScreen "
        "chunks the sides two to a row and draws them positionally, so Compose has no identity to "
        "preserve. It becomes the right thing the moment that list can reorder or animate",
    "GuidedOffer.carriesValue": "MEASURED and currently unreachable. It marks takeoff, altitude, "
        "speed and pause - the actions that need a number before they can be sent. This head never "
        "asks: the three value actions have their own dialogs on the flight row, and the one that "
        "reaches the more-actions sheet is pause, special-cased by id because the flag cannot say "
        "WHICH dialog an action wants. A guard for an action the core adds to the sheet that this "
        "head has no dialog for would be the use, and SHEET_ACTIONS is a head-side allowlist, so "
        "that case cannot arise until the allowlist grows",
    "TerrainProfile.highestText": "MEASURED and redundant here. bandText is range_text(min, max) - "
        "'474 m to 596 m' - and that is what profileLabel draws when the route is not flat, so the "
        "highest already reaches the operator inside it. lowestText is drawn alone for a flat "
        "route, where there is no band to state. A head that wanted the two ends separately would "
        "read both; this one states the range",
    "LogsView.eraseWarning": "MEASURED and deliberately not drawn. The core serves 'This erases "
        "every log on the vehicle.' and the dialog says 'This permanently deletes every log on the "
        "vehicle. It cannot be undone.' - the head's wording is the stronger one because it names "
        "the irreversibility, which is the fact an operator needs before a destructive action. "
        "Adopting the served sentence would lose that",
}


def corpus():
    return "\n".join(p.read_text() for tree in TREES for p in tree.rglob("*.kt"))


def parameters(text, at):
    depth = 0
    for index in range(at, len(text)):
        if text[index] == "(":
            depth += 1
        elif text[index] == ")":
            depth -= 1
            if depth == 0:
                return text[at + 1:index]
    return ""


def properties(text):
    for found in re.finditer(r"(?:internal )?data class (\w+)\s*\(", text):
        body = parameters(text, found.end() - 1)
        for name in re.findall(r"val (\w+)\s*:", body):
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
    every = [
        (cls, name)
        for tree in TREES
        for path in tree.rglob("*.kt")
        for cls, name in properties(path.read_text())
    ]
    sampled = [name for cls, name in every if cls == KNOWN_CLASS[0]]
    if sampled != KNOWN_CLASS[1]:
        raise SystemExit(f"instrument broken: {KNOWN_CLASS[0]} should have {KNOWN_CLASS[1]}, saw {sampled}")
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
    print("  Counted by field NAME across the whole corpus, so a field read on ANY class counts as")
    print("  read on every class that has one of that name. VehicleLinks.primary, .contactLost and")
    print("  .reason sat here unread and uncounted because GuidedOffer.reason and VehicleChoice")
    print("  .contactLost are read; one of them was pinned by a test and drawn nowhere. A zero above")
    print("  is not a clean head. Scoping to files naming the class was tried and flags 98, because")
    print("  Kotlin infers types and a use site need not spell the class - it needs real resolution.")
    return 1 if dead else 0


if __name__ == "__main__":
    sys.exit(main())
