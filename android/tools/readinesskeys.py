#!/usr/bin/env python3
"""Flag readiness predicates the core computes that the head never reads.

The head keeps reaching for the flag whose name sounds like the question and
getting a weaker one. Four instances so far, one of them dangerous:

  blocked          vs ready           guided actions - arm stayed live on an unknown state
  setupComplete    vs ready           Setup said "Ready to fly" ignoring sensor health
  hasModes         vs canChangeMode   camera offered a mode change mid-recording
  connected+syncing vs canSend        Upload overwrote a mission a vehicle was flying

A capability flag says the thing exists. A readiness flag says it is allowed
right now. Gating on the first is how a head offers what the core refused.

Every hit is a question. Three known-good reasons a key lands here:
a rule the head reimplements correctly for the narrower case it uses (fences'
cornerRemovable), and the hub family, which is empty unless the core owns the
link and which no head consumes, and a threshold constant served so a head
could word a sentence, where the head shows the core's own reason instead.
"""
import re
import sys
from pathlib import Path

QGC = Path(__file__).resolve().parents[2]
HEAD = Path(__file__).resolve().parents[1]

READINESS = re.compile(r"^(can[A-Z]|ready$|ready[A-Z]|is[A-Z].*(Valid|Allowed|Permitted)$|allowed|permitted)")
KNOWN = {"canRemoveVertex": "fences: head's minimum of 3 matches on the handset - offered at 4 corners, withheld at 3",
         "canBeSet": "hub family - empty unless the core owns the link, no head consumes it",
         "allowedFixAgeMs": "a THRESHOLD, not a verdict: followme.rs serves the constant so a head "
                            "could word its own sentence, and serves fixFresh and wouldSend beside "
                            "it. This head reads wouldSend and the core's reason, and never touches "
                            "fixAgeMs, so it re-derives nothing"}


def head_reads():
    sources = [p for p in HEAD.rglob("src/main/java/**/*.kt") if "/build/" not in str(p)]
    return set(re.findall(r'"([A-Za-z_][A-Za-z0-9_]*)"', "\n".join(p.read_text() for p in sources)))


def flagged():
    read = head_reads()
    out = []
    for path in sorted((QGC / "core-rs/src").rglob("*.rs")):
        body = path.read_text(errors="ignore").split("#[cfg(test)]")[0]
        for key in sorted(set(re.findall(r'"([a-zA-Z_]\w*)"\s*:', body))):
            if READINESS.match(key) and key not in read:
                out.append((path.name, key))
    return out


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
    print("readinesskeys selftest OK")


def core_is_present():
    if (QGC / "core-rs/src").is_dir():
        return True
    print(f"skipped: no core-rs under {QGC} - this sweep checked nothing")
    return False


if __name__ == "__main__":
    if "--selftest" in sys.argv:
        selftest()
    elif not core_is_present():
        sys.exit(0)
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
