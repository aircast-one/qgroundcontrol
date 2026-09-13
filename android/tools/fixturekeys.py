#!/usr/bin/env python3
"""Flag JSON keys in head test fixtures that no producer actually emits.

Every hit is a question, not a defect. Known-legitimate reasons a key lands here:
a Compose item(key = ...) that happens to share the name, and a fixture that
deliberately feeds an invented name to prove the decoder ignores it. Validate a
hit against the producer before changing anything.

The failure this exists to catch shipped once: LogDownloadScreen read timeText,
core-rs emitted time and timeState, and the head's own fixture invented timeText
so both suites stayed green while every log row rendered a blank date.
"""
import re
import sys
from pathlib import Path

QGC = Path(__file__).resolve().parents[2] / "qgroundcontrol"
HEAD = Path(__file__).resolve().parents[1]


def camel(name):
    head, *rest = name.split("_")
    return head + "".join(w[:1].upper() + w[1:] for w in rest)


def sources(kind):
    return [p for p in HEAD.rglob(f"src/{kind}/java/**/*.kt") if "/build/" not in str(p)]


def produced():
    keys = set()
    for path in (QGC / "core-rs/src").rglob("*.rs"):
        body = path.read_text(errors="ignore").split("#[cfg(test)]")[0]
        keys |= set(re.findall(r'"([A-Za-z_][A-Za-z0-9_]*)"\s*:', body))
        for block in re.findall(r"struct\s+\w+\s*\{([^}]*)\}", body, re.S):
            fields = re.findall(r"(?:pub\s+)?([a-z_][a-z0-9_]*)\s*:", block)
            keys |= set(fields) | {camel(f) for f in fields}
    for path in (QGC / "src").rglob("*.h"):
        source = path.read_text(errors="ignore")
        keys |= set(re.findall(r"Q_PROPERTY\s*\(\s*[\w:<>\s\*]+?\s+(\w+)\s+(?:READ|MEMBER)", source))
        keys |= set(re.findall(r"Q_INVOKABLE[^;]*?\b(\w+)\s*\(", source))
    for path in (QGC / "src/Bridge").rglob("*.cc"):
        keys |= set(re.findall(r'QStringLiteral\("([A-Za-z_][A-Za-z0-9_]*)"\)', path.read_text(errors="ignore")))
    for path in sources("main"):
        keys |= set(re.findall(r'\.put\(\s*"([A-Za-z_][A-Za-z0-9_]*)"', path.read_text()))
    return keys


DECODES = r'\b(?:opt|get|has|isNull)[A-Za-z]*\(\s*"([A-Za-z_][A-Za-z0-9_]*)"'


def decoded():
    return set(re.findall(DECODES, "\n".join(p.read_text() for p in sources("main"))))


def report():
    emitted = produced()
    read = decoded()
    rows = {}
    for path in sources("test"):
        for match in re.finditer(r'"([A-Za-z_][A-Za-z0-9_]*)"\s*:', path.read_text()):
            rows.setdefault(match.group(1), set()).add(path.name)
    flagged = {k: v for k, v in rows.items() if k not in emitted}
    for key, files in sorted(flagged.items()):
        note = "  DECODED BY THE HEAD - nothing produces it" if key in read else "  no reader found"
        print(f"{key:24s} {', '.join(sorted(files))}{note}")
    live = sorted(k for k in flagged if k in read)
    if live:
        print(f"\nfixturekeys: {len(live)} key(s) the head decodes that no producer emits: {', '.join(live)}")
    return live


def selftest():
    emitted = produced()
    reads = decoded()
    assert "notice" not in reads, "item(key = \"notice\") is a Compose key, not a decode"
    assert "surveyAreaPolygon" in reads or "rows" in reads, "a real optJSONArray(\"...\") call is a decode"
    assert "timeText" not in emitted, "timeText was emitted by nothing; the sweep must flag it"
    assert "sizeText" in emitted, "sizeText is a real key beside it; the sweep must not flag it"
    assert "amslTerrainHeights" in emitted, "MEMBER properties count as emitted"
    assert "channel" in emitted, "a key the head itself writes into its own settings blob is produced"
    print("fixturekeys selftest OK")


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
        sys.exit(1 if report() else 0)
