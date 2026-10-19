#!/usr/bin/env python3
import re
import sys
from pathlib import Path

from fixturekeys import without_tests

QGC = Path(__file__).resolve().parents[2]
HEAD = Path(__file__).resolve().parents[1]

READINESS = re.compile(
    r"^(can[A-Z]|ready$|ready[A-Z]|is[A-Z].*(Valid|Allowed|Permitted)$|allowed|permitted)"
)
KNOWN = {
    "isInsertROIValid": "the MissionController-shaped plan object mirrors QGC's property; heads offer a "
    "region of interest from view.missionKinds, which missionkinds.rs derives from the same insert_state.roi",
    "canRemoveVertex": "fences: head's minimum of 3 matches on the handset - offered at 4 corners, withheld at 3",
    "canBeSet": "hub family - empty unless the core owns the link, no head consumes it",
    "allowedFixAgeMs": "a THRESHOLD, not a verdict: followme.rs serves the constant so a head "
    "could word its own sentence, and serves fixFresh and wouldSend beside "
    "it. This head reads wouldSend and the core's reason, and never touches "
    "fixAgeMs, so it re-derives nothing",
}


def head_reads():
    sources = [p for p in HEAD.rglob("src/main/java/**/*.kt") if "/build/" not in str(p)]
    return set(re.findall(r'"([A-Za-z_][A-Za-z0-9_]*)"', "\n".join(p.read_text() for p in sources)))


def core_bodies():
    return {
        path.name: without_tests(path.read_text(errors="ignore"))
        for path in sorted((QGC / "groundstation/src").rglob("*.rs"))
    }


def core_reads(bodies):
    return {
        key for body in bodies.values() for key in re.findall(r'"([a-zA-Z_]\w*)"(?!\s*:)', body)
    }


def flagged():
    bodies = core_bodies()
    read = head_reads() | core_reads(bodies)
    return [
        (name, key)
        for name, body in bodies.items()
        for key in sorted(set(re.findall(r'"([a-zA-Z_]\w*)"\s*:', body)))
        if READINESS.match(key) and key not in read
    ]


def unexplained():
    return [(view, key) for view, key in flagged() if key not in KNOWN]


def selftest():
    keys = {k for _, k in flagged()}
    assert all(k in KNOWN for _, k in flagged()), (
        "every current hit must carry a reason; add one to KNOWN or fix the head: "
        + ", ".join(k for _, k in unexplained())
    )
    assert "canSend" not in keys, "the upload gate is read now; the sweep must not still flag it"
    assert READINESS.match("canChangeMode"), "the camera key is the shape this looks for"
    assert not READINESS.match("hasModes"), "capability flags are not readiness flags"
    assert not READINESS.match("cancelled"), "a word starting with can is not a can-flag"
    assert "canTakeoff" not in keys, (
        "guided.rs gates takeoff on it, so the core reads it for the head"
    )
    print("readinesskeys selftest OK")


def core_is_present():
    if (QGC / "groundstation/src").is_dir():
        return True
    print(f"REFUSING: no groundstation under {QGC} - this sweep checked nothing")
    return False


if __name__ == "__main__":
    if "--selftest" in sys.argv:
        selftest()
    elif not core_is_present():
        sys.exit(2)
    else:
        for view, key in flagged():
            note = KNOWN.get(key, "UNEXPLAINED - check what the head gates on instead")
            print(f"{view:18s} {key:20s} {note}")
        loose = unexplained()
        if loose:
            print(
                f"\nreadinesskeys: {len(loose)} readiness flag(s) the core computes and the head "
                f"never reads: {', '.join(k for _, k in loose)}"
            )
        sys.exit(1 if loose else 0)
