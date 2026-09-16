"""Count the Qt paths this head still asks for directly.

The port is finished for this head when the count is zero: every reading comes
from a `view.*` the core serves. Run it from anywhere:

    python3 android/tools/qtpaths.py          # the count, by root
    python3 android/tools/qtpaths.py --list   # every path and where it is read

A path is counted when it is the first argument of a bridge call - qgcPath,
qgcString, qgcDouble, Qgc.get/set/invoke, setOk, invokeOk - and does not start
with `view.` and is not an action the core already owns. Field names that merely
look like paths are not counted, which is why this reads call sites rather than
grepping for quoted strings.

The core-owned list is read out of `core-rs/src/actions.rs` at scan time rather
than copied here. `mission.insert` looks exactly like a Qt path and never reaches
Qt: `router.invoke` hands it to `actions::run` before the backend sees it. Ten of
them were being counted as work to do.
"""

import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(
    subprocess.run(["git", "rev-parse", "--show-toplevel"], capture_output=True, text=True, cwd=Path(__file__).parent)
    .stdout.strip()
)
SOURCES = [ROOT / "android/app/src/main/java", ROOT / "android/map-spike/src/main/java"]
CALLS = r"(?:qgcPath|qgcString|qgcDouble|qgcJson|setOk|invokeOk|Qgc\.get|Qgc\.set|Qgc\.invoke|Qgc\.invokeResult|QGCBridge\.get|QGCBridge\.getFields|QGCBridge\.set|QGCBridge\.invoke)"
LITERAL = re.compile(CALLS + r'\(\s*"([^"$]+)"')
CONSTANT = re.compile(CALLS + r"\(\s*([A-Z][A-Z0-9_]{2,})\b")
DEFINE = re.compile(r'\b(?:const\s+val|val)\s+([A-Z][A-Z0-9_]{2,})\s*(?::\s*String\s*)?=\s*"([^"$]+)"')


OWNED = re.compile(r'^const [A-Z_]+: &str = "([^"]+)";', re.M)


def core_owned() -> set[str]:
    source = (ROOT / "core-rs/src/actions.rs").read_text()
    listed = source.split("pub const OWNED", 1)[0]
    return set(OWNED.findall(listed))


def defined_constants() -> dict[str, str]:
    found: dict[str, str] = {}
    for source in SOURCES:
        for path in source.rglob("*.kt"):
            for name, value in DEFINE.findall(path.read_text()):
                found.setdefault(name, value)
    return found


def asked() -> list[tuple[str, str]]:
    constants = defined_constants()
    hits: list[tuple[str, str]] = []
    for source in SOURCES:
        for path in sorted(source.rglob("*.kt")):
            text = path.read_text()
            where = str(path.relative_to(ROOT))
            for value in LITERAL.findall(text):
                hits.append((value, where))
            for name in CONSTANT.findall(text):
                if name in constants:
                    hits.append((constants[name], where))
    owned = core_owned()
    return [(value, where) for value, where in hits if not value.startswith("view.") and value not in owned]


def check() -> None:
    constants = defined_constants()
    assert "VEHICLE_LINKS" in constants, "the constant sweep found no VEHICLE_LINKS, so every constant path is invisible"
    assert constants["VEHICLE_LINKS"] == "view.vehicleLinks", constants["VEHICLE_LINKS"]
    sample = 'val x by qgcPath("vehicle.armed")\nval y by qgcPath("view.flyState")\nval z = someField("vehicle.nope")'
    assert LITERAL.findall(sample) == ["vehicle.armed", "view.flyState"], LITERAL.findall(sample)
    owned = core_owned()
    assert "mission.insert" in owned, "the core-owned sweep found nothing, so every core action counts as Qt work"
    assert "vehicle.armed" not in owned, sorted(owned)


def main() -> None:
    check()
    hits = asked()
    roots: dict[str, int] = {}
    for value, _ in hits:
        roots[value.split(".")[0]] = roots.get(value.split(".")[0], 0) + 1
    unique = sorted({value for value, _ in hits})
    print("%d Qt paths asked for directly, %d distinct" % (len(hits), len(unique)))
    for root, count in sorted(roots.items(), key=lambda pair: -pair[1]):
        print("  %-18s %d" % (root, count))
    if "--list" in sys.argv:
        print()
        for value in unique:
            for asked_value, where in hits:
                if asked_value == value:
                    print("  %-46s %s" % (value, where))
                    break


if __name__ == "__main__":
    main()
