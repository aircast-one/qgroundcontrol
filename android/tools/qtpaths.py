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
with `view.` and is not a write or invoke the core keeps. Field names that merely
look like paths are not counted, which is why this reads call sites rather than
grepping for quoted strings.

Which writes and invokes the core keeps is asked of the core itself, through the
ignored `claims_for_qtpaths` test in `core-rs/src/actions.rs`, which answers with the
same `owns` and `owns_write` the router consults. `mission.insert` looks exactly like
a Qt path and never reaches Qt, and neither does a write to any fact under
`settings.` - a claim made by pattern, which a list of names copied out of
actions.rs could not see, so every such write was counted as work still to do.
An index the head fills in at run time is asked about as 0; a path whose root is
itself a run-time value cannot be asked about and stays counted.
"""

import os
import re
import subprocess
import sys
import tempfile
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
WRITES = {"setOk", r"Qgc\.set", r"QGCBridge\.set"}
INVOKES = {"invokeOk", r"Qgc\.invoke", r"Qgc\.invokeResult", r"QGCBridge\.invoke"}


def call_pattern() -> str:
    """Every bridge wrapper, found wherever it is declared. Naming the two files that
    declare them today would stop counting the day a third module grows one."""
    declared: set[str] = set()
    for source in SOURCES:
        for path in source.rglob("*.kt"):
            declared.update(WRAPPERS.findall(path.read_text()))
    return "({})".format("|".join(sorted(declared) + DIRECT))


def kind(call: str) -> str:
    """What the router does with a call: a write and an invoke can be kept by the core, a read of
    anything but a view always reaches Qt."""
    spelled = call.replace(".", r"\.")
    return "set" if spelled in WRITES else "invoke" if spelled in INVOKES else "read"


CALLS = call_pattern()
LITERAL = re.compile(CALLS + r'\(\s*"([^"$]+)"')
TEMPLATE = re.compile(CALLS + r'\(\s*"([^"]*\$[^"]*)"')
CONSTANT = re.compile(CALLS + r"\(\s*([A-Z][A-Z0-9_]{2,})\b")
# ${row.index} is one piece. Reading only its first name left ".index}" behind as if it were
# part of the path, which made a shape no core pattern could ever match.
PIECE = re.compile(r"\$\{([^}]*)\}|\$([A-Za-z_][A-Za-z0-9_]*)")
DEFINE = re.compile(r'\b(?:const\s+val|val)\s+([A-Z][A-Z0-9_]{2,})\s*(?::\s*String\s*)?=\s*"([^"]+)"')


def instance(path: str) -> str | None:
    """A shape the core can be asked about: each run-time index becomes 0. A path whose root is
    run-time has no shape to ask about."""
    segments = path.split(".")
    if segments[0] == "*":
        return None
    return ".".join("0" if segment == "*" else segment for segment in segments)


def core_claims(asked_for: set[tuple[str, str]]) -> set[tuple[str, str]]:
    """The (kind, path) pairs the core keeps, answered by the core's own predicates."""
    queries = sorted({(k, instance(p)) for k, p in asked_for if k != "read" and instance(p)})
    with tempfile.NamedTemporaryFile("w", suffix=".tsv", delete=False) as query:
        query.write("".join(f"{k}\t{p}\n" for k, p in queries))
    try:
        run = subprocess.run(
            ["cargo", "test", "--locked", "-q", "--lib", "claims_for_qtpaths", "--", "--ignored", "--nocapture"],
            cwd=ROOT / "core-rs", capture_output=True, text=True, env={**os.environ, "QTPATHS_QUERY": query.name},
        )
    finally:
        os.unlink(query.name)
    answers = [line.split("\t")[1:] for line in run.stdout.splitlines() if line.startswith("CLAIM\t")]
    if run.returncode != 0 or len(answers) != len(queries):
        raise SystemExit(f"the core did not answer for its claims (cargo exited {run.returncode}):\n{run.stderr[-2000:]}")
    kept = {(k, p) for k, p, claimed in answers if claimed == "true"}
    return {(k, p) for k, p in asked_for if (k, instance(p)) in kept}


OWNS_BODY = 'path.split([\'.\', \'[\']).next() == Some("view")'


def resolve(template: str, constants: dict[str, str]) -> str:
    """A path built by interpolation is still a path. Known constants are substituted;
    anything else - an index, a camera number - becomes * because the core has to cover
    the shape, not the instance."""
    return PIECE.sub(lambda hit: constants.get(hit.group(1) or hit.group(2), "*"), template)


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
        grown = {name: PIECE.sub(lambda hit: found.get(hit.group(1) or hit.group(2), hit.group(0)), value) for name, value in found.items()}
        if grown == found:
            break
        found = grown
    return {name: value for name, value in found.items() if "$" not in value}


# A path built at run time - `vehicle.$method`, `${fact.path}.validate` - has no shape the core can
# be asked about. The call site can say what it becomes, on its own line or the one above:
#     // qtpaths: vehicle.guidedModeChangeGroundSpeedMetersSecond, vehicle.guidedModeChangeEquivalentAirspeedMetersSecond
# Each named path must fit the template, so a declaration left behind by a changed call is refused
# rather than counted, and each is asked about as if it were written out.
DECLARED = re.compile(r"//\s*qtpaths:\s*(.+)$")


def declared(lines: list[str], at: int) -> list[str]:
    for line in (lines[at], lines[at - 1] if at > 0 else ""):
        hit = DECLARED.search(line)
        if hit:
            return [path.strip() for path in hit.group(1).split(",") if path.strip()]
    return []


def fits(example: str, shape: str) -> bool:
    """A run-time piece may stand for several segments - a fact path is settings.appSettings.x - so *
    matches any run of characters, but everything the template spells out has to be there."""
    return re.fullmatch(".+".join(re.escape(piece) for piece in shape.split("*")), example) is not None


def asked() -> list[tuple[str, str]]:
    constants = defined_constants()
    hits: list[tuple[str, str, str]] = []
    for source in SOURCES:
        for path in sorted(source.rglob("*.kt")):
            text = path.read_text()
            where = str(path.relative_to(ROOT))
            for call, value in LITERAL.findall(text):
                hits.append((kind(call), value, where))
            for call, name in CONSTANT.findall(text):
                if name in constants:
                    hits.append((kind(call), constants[name], where))
            lines = text.splitlines()
            for found in TEMPLATE.finditer(text):
                call, template = found.groups()
                shape = resolve(template, constants)
                at = text.count("\n", 0, found.start())
                examples = declared(lines, at)
                for example in examples:
                    if not fits(example, shape):
                        raise SystemExit(f"{where}:{at + 1}: '// qtpaths: {example}' is not a path {shape} can become")
                for value in examples or [shape]:
                    hits.append((kind(call), value, where + ("" if examples else " [template]")))
    hits = [hit for hit in hits if not core_serves(hit[1])]
    kept = core_claims({(k, value) for k, value, _ in hits})
    return [(value, where) for k, value, where in hits if (k, value) not in kept]


def check() -> None:
    constants = defined_constants()
    assert "VEHICLE_LINKS" in constants, "the constant sweep found no VEHICLE_LINKS, so every constant path is invisible"
    assert constants["VEHICLE_LINKS"] == "view.vehicleLinks", constants["VEHICLE_LINKS"]
    assert constants.get("FENCE_ROOT", "").startswith("plan."), (
        "a constant built from another constant did not resolve, so every path under it counts as unknown: %r"
        % constants.get("FENCE_ROOT")
    )
    kept = sys.argv[1:]
    sys.argv[1:] = ["--expect=3"]
    assert expectation() == 3, "--expect=N is read as no expectation at all, so the check passes by not running"
    sys.argv[1:] = ["--expect", "-2"]
    assert expectation() == -2
    sys.argv[1:] = ["--list"]
    assert expectation() is None
    sys.argv[1:] = kept
    assert resolve("$GPS.count", {"GPS": "vehicle.gps"}) == "vehicle.gps.count"
    assert resolve("$A.$index.center", {"A": "plan.fence"}) == "plan.fence.*.center"
    assert resolve("view.$X", {"X": "flyState"}) == "view.flyState"
    assert resolve("$LINKS.${row.index}.link.disconnect", {"LINKS": "links.linkConfigurations"}) == "links.linkConfigurations.*.link.disconnect"
    assert instance("plan.fence.*.center") == "plan.fence.0.center" and instance("*.armed") is None
    assert fits("settings.appSettings.x.validate", "*.validate") and not fits("settings.x.enumIndex", "*.validate")
    assert declared(["// qtpaths: a.b, c.d", 'Qgc.invoke("$X.y")'], 1) == ["a.b", "c.d"] and declared(["x"], 0) == []
    for wrapper in ("qgcPath", "qgcString", "mapPath", "mapString", "mapInt"):
        assert wrapper in CALLS, "the wrapper sweep missed %s, so a module's reads are invisible" % wrapper
    sample = 'val x by qgcPath("vehicle.armed")\nval y by qgcPath("view.flyState")\nval z = someField("vehicle.nope")'
    assert LITERAL.findall(sample) == [("qgcPath", "vehicle.armed"), ("qgcPath", "view.flyState")], LITERAL.findall(sample)
    assert (kind("Qgc.set"), kind("setOk"), kind("Qgc.invokeResult"), kind("qgcPath")) == ("set", "set", "invoke", "read")
    probe = {("invoke", "mission.insert"), ("set", "settings.videoSettings.videoSource"), ("set", "vehicle.armed"), ("read", "settings.videoSettings.videoSource")}
    assert core_claims(probe) == {("invoke", "mission.insert"), ("set", "settings.videoSettings.videoSource")}, (
        f"the core's claims read wrong, so what counts as Qt work has moved: {core_claims(probe)!r}"
    )
    assert core_serves("view.flyState") and core_serves("view") and core_serves("view[0].x")
    assert not core_serves("viewfinder.zoom") and not core_serves("vehicle.armed")
    assert router_predicate() == OWNS_BODY, (
        "view::owns no longer reads as this tool assumes, so what counts as core-served has moved:\n"
        "  view.rs: %s\n  here:    %s" % (router_predicate(), OWNS_BODY)
    )


def expectation() -> int | None:
    """Accepts --expect N and --expect=N. A flag spelled one way and read the other
    is a check that passes by not running, which is the failure it exists to catch."""
    for index, argument in enumerate(sys.argv):
        if argument == "--expect":
            return int(sys.argv[index + 1])
        if argument.startswith("--expect="):
            return int(argument.split("=", 1)[1])
    return None


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
    expected = expectation()
    if expected is not None:
        delta = expected
        was = int(BASELINE.read_text().split()[0]) if BASELINE.exists() else len(unique)
        wanted = was + delta
        if len(unique) != wanted:
            print("REFUSED: %d distinct, expected %d (%d %+d)." % (len(unique), wanted, was, delta))
            print("A change the tool cannot see is a blind instrument, not a green run.")
            raise SystemExit(1)
        BASELINE.write_text("%d\n" % len(unique))
        print("count moved %+d as claimed (%d -> %d)." % (delta, was, len(unique)))
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
