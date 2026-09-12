#!/usr/bin/env python3
"""Emit the set of contract field names this head references. Names only.

served-unread.py catches a field the core ADDS that this head never reads. Nothing on either
side catches the mirror: a field the core still serves that a head STOPPED reading. The core
cannot derive it, because only the head knows which names it references.

So this writes the set out on each commit that touches macos/, and the core diffs it across
commits. Names only, deliberately: no file structure, no line numbers, no reasons. The core
asked for it that way so the artefact stays cheap and does not become a fourth table to drift
out of date -- the reasons live in view-fields.py and the ACCEPTED tables, and belong in one
place.

A name is counted as referenced if it appears anywhere under macos/Sources, so a name read for
one view counts for every view. That matches served-unread.py's own matching, and it means a
disappearance from this list is a strong signal: the head stopped naming it ANYWHERE.

Usage: python3 tools/macos/head-reads.py   # rewrites tools/macos/head-reads.txt
"""
import json
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parents[2]
SOURCES = ROOT / "macos/Sources"
CONTRACT = ROOT / "test/Bridge/fixtures/view-shapes.json"
ARTEFACT = pathlib.Path(__file__).resolve().parent / "head-reads.txt"


def served_names():
    contract = json.loads(CONTRACT.read_text())
    return {field for path in contract.get("_observed", [])
            for field in [path.rpartition(".")[2]] if field.isidentifier()}


def referenced():
    text = "\n".join(path.read_text() for path in sorted(SOURCES.glob("*.swift")))
    named = set(re.findall(r'"([A-Za-z_][A-Za-z0-9_]*)"', text))
    named |= set(re.findall(r"\b([a-z][A-Za-z0-9_]*)\b", text))
    return named


listed = sorted(served_names() & referenced())
ARTEFACT.write_text("\n".join(listed) + "\n")
print(f"{ARTEFACT.relative_to(ROOT)}: {len(listed)} contract field names this head references")
sys.exit(0)
