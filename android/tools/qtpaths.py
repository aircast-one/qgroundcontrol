"""Count the Qt paths this head still asks for directly.

The port is finished for this head when the count is zero: every reading comes
from a `view.*` the core serves. Run it from anywhere:

    python3 android/tools/qtpaths.py             # the count, by root
    python3 android/tools/qtpaths.py --list      # every path and where it is read
    python3 android/tools/qtpaths.py --expect -2 # assert the count fell by two, then record it

Run `--expect` with every commit that converts or deletes a path. A count that
does not move by what you changed means the tool cannot see your change, which is
how a whole module of reads stayed invisible through two reconciliations.

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
BASELINE = ROOT / "android/tools/qtpaths.baseline"
WRAPPERS = re.compile(r"^(?:internal )?fun (qgc[A-Za-z]+|map[A-Za-z]+)\(\s*(?:group)?[Pp]ath: String", re.M)
DIRECT = ["setOk", "invokeOk", r"Qgc\.get", r"Qgc\.set", r"Qgc\.invoke", r"Qgc\.invokeResult",
          r"QGCBridge\.get", r"QGCBridge\.getFields", r"QGCBridge\.set", r"QGCBridge\.invoke"]
def call_pattern() -> str:
    """Every bridge wrapper, found wherever it is declared. Naming the two files that
    declare them today would stop counting the day a third module grows one."""
    declared: set[str] = set()
    for source in SOURCES:
        for path in source.rglob("*.kt"):
            declared.update(WRAPPERS.findall(path.read_text()))
    return "(?:%s)" % "|".join(sorted(declared) + DIRECT)


CALLS = call_pattern()
LITERAL = re.compile(CALLS + r'\(\s*"([^"$]+)"')
TEMPLATE = re.compile(CALLS + r'\(\s*"([^"]*\$[^"]*)"')
CONSTANT = re.compile(CALLS + r"\(\s*([A-Z][A-Z0-9_]{2,})\b")
PIECE = re.compile(r"\$\{?([A-Za-z_][A-Za-z0-9_]*)\}?")
DEFINE = re.compile(r'\b(?:const\s+val|val)\s+([A-Z][A-Z0-9_]{2,})\s*(?::\s*String\s*)?=\s*"([^"]+)"')


OWNED = re.compile(r'^const [A-Z_]+: &str = "([^"]+)";', re.M)


def core_owned() -> set[str]:
    source = (ROOT / "core-rs/src/actions.rs").read_text()
    listed = source.split("pub const OWNED", 1)[0]
    return set(OWNED.findall(listed))


OWNS_BODY = 'path.split([\'.\', \'[\']).next() == Some("view")'


def resolve(template: str, constants: dict[str, str]) -> str:
    """A path built by interpolation is still a path. Known constants are substituted;
    anything else - an index, a camera number - becomes * because the core has to cover
    the shape, not the instance."""
    return PIECE.sub(lambda hit: constants.get(hit.group(1), "*"), template)


def core_serves(path: str) -> bool:
    """The router's own test. `check()` asserts view.rs still spells it this way."""
    return re.split(r"[.\[]", path, maxsplit=1)[0] == "view"


def router_predicate() -> str:
    source = (ROOT / "core-rs/src/view.rs").read_text()
    body = source.split("pub fn owns(path: &str) -> bool {", 1)[1].split("}", 1)[0]
    return " ".join(body.split())


def defined_constants() -> dict[str, str]:
    """A constant may be built from another - FENCE_ROOT is "$PLAN_ROOT.geoFenceController" -
    so the table is resolved to a fixpoint before anything is looked up in it."""
    found: dict[str, str] = {}
    for source in SOURCES:
        for path in source.rglob("*.kt"):
            for name, value in DEFINE.findall(path.read_text()):
                found.setdefault(name, value)
    for _ in range(8):
        grown = {name: PIECE.sub(lambda hit: found.get(hit.group(1), hit.group(0)), value) for name, value in found.items()}
        if grown == found:
            break
        found = grown
    return {name: value for name, value in found.items() if "$" not in value}


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
            for template in TEMPLATE.findall(text):
                hits.append((resolve(template, constants), where + " [template]"))
    owned = core_owned()
    return [(value, where) for value, where in hits if not core_serves(value) and value not in owned]


def check() -> None:
    constants = defined_constants()
    assert "VEHICLE_LINKS" in constants, "the constant sweep found no VEHICLE_LINKS, so every constant path is invisible"
    assert constants["VEHICLE_LINKS"] == "view.vehicleLinks", constants["VEHICLE_LINKS"]
    assert constants.get("FENCE_ROOT", "").startswith("plan."), (
        "a constant built from another constant did not resolve, so every path under it counts as unknown: %r"
        % constants.get("FENCE_ROOT")
    )
    assert resolve("$GPS.count", {"GPS": "vehicle.gps"}) == "vehicle.gps.count"
    assert resolve("$A.$index.center", {"A": "plan.fence"}) == "plan.fence.*.center"
    assert resolve("view.$X", {"X": "flyState"}) == "view.flyState"
    for wrapper in ("qgcPath", "qgcString", "mapPath", "mapString", "mapInt"):
        assert wrapper in CALLS, "the wrapper sweep missed %s, so a module's reads are invisible" % wrapper
    sample = 'val x by qgcPath("vehicle.armed")\nval y by qgcPath("view.flyState")\nval z = someField("vehicle.nope")'
    assert LITERAL.findall(sample) == ["vehicle.armed", "view.flyState"], LITERAL.findall(sample)
    owned = core_owned()
    assert "mission.insert" in owned, "the core-owned sweep found nothing, so every core action counts as Qt work"
    assert "vehicle.armed" not in owned, sorted(owned)
    assert core_serves("view.flyState") and core_serves("view") and core_serves("view[0].x")
    assert not core_serves("viewfinder.zoom") and not core_serves("vehicle.armed")
    assert router_predicate() == OWNS_BODY, (
        "view::owns no longer reads as this tool assumes, so what counts as core-served has moved:\n"
        "  view.rs: %s\n  here:    %s" % (router_predicate(), OWNS_BODY)
    )


def main() -> None:
    check()
    hits = asked()
    roots: dict[str, int] = {}
    for value, _ in hits:
        roots[value.split(".")[0]] = roots.get(value.split(".")[0], 0) + 1
    unique = sorted({value for value, _ in hits})
    templates = len({value for value, where in hits if where.endswith(" [template]")})
    print("%d Qt paths asked for directly, %d distinct, %d of them template shapes" % (len(hits), len(unique), templates))
    for root, count in sorted(roots.items(), key=lambda pair: -pair[1]):
        print("  %-18s %d" % (root, count))
    if "--expect" in sys.argv:
        delta = int(sys.argv[sys.argv.index("--expect") + 1])
        was = int(BASELINE.read_text().split()[0]) if BASELINE.exists() else len(unique)
        wanted = was + delta
        if len(unique) != wanted:
            print("REFUSED: %d distinct, expected %d (%d %+d)." % (len(unique), wanted, was, delta))
            print("A change the tool cannot see is a blind instrument, not a green run.")
            raise SystemExit(1)
        BASELINE.write_text("%d\n" % len(unique))
        print("baseline now %d" % len(unique))
    if "--list" in sys.argv:
        print()
        for value in unique:
            for asked_value, where in hits:
                if asked_value == value:
                    shape = " [template]" if where.endswith(" [template]") else ""
                    print("  %-46s %s%s" % (value, where.removesuffix(" [template]"), shape))
                    break


if __name__ == "__main__":
    main()
