#!/usr/bin/env python3
"""Regenerates MACOS_QT_PASSTHROUGHS.md from qt-paths.py, keeping the decisions written under it.

The first version of this was retyped inline every time, and it rebuilt the file from the
headline to the end -- so every measured decision about a path lasted exactly until the next
conversion regenerated the table over it. A generated document that cannot carry a note is a
document that quietly forgets why a path is still there, which is the one thing the list is
for: what each path BECAME, not how many are left.

Everything from the DECISIONS marker to the end of the file is preserved verbatim. The table
and the headline above it are always rebuilt, so they cannot drift from the tool.
"""
import pathlib
import re
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parents[2]
DOC = ROOT / "MACOS_QT_PASSTHROUGHS.md"
DECISIONS = "## Decisions"


def main():
    run = subprocess.run([sys.executable, str(ROOT / "tools/macos/qt-paths.py"), "--list"],
                         capture_output=True, text=True)
    head, marker, listing = run.stdout.partition("\nPATHS\n")
    if not marker or not listing.strip():
        print(f"  REFUSING: qt-paths.py --list printed no PATHS block (exit {run.returncode}), so "
              f"the table would be rebuilt EMPTY and the file would report a finished migration. "
              f"stderr: {run.stderr.strip()[:200]}", file=sys.stderr)
        return 2
    rows = [line.strip().partition("\t")[::2] for line in listing.strip().splitlines()]
    if not DOC.exists():
        print(f"  REFUSING: {DOC} does not exist, so there is nothing to preserve", file=sys.stderr)
        return 2
    doc = DOC.read_text()
    kept = doc[doc.index(DECISIONS):] if DECISIONS in doc else DECISIONS + "\n\n"

    start = doc.index("```\n") + 4
    body = doc[:start] + head.strip() + doc[doc.index("\n```", start):]
    table = body.index("| path | use | status |")
    body = body[:table] + "| path | use | status |\n|---|---|---|\n" + "\n".join(
        f"| `{path}` | {use} | passthrough |" for path, use in rows) + "\n\n" + kept
    DOC.write_text(body)
    total = re.search(r"raw Qt total\s+(\d+)", head)
    print(f"regenerated {DOC.name}: {len(rows)} paths listed, headline says "
          f"{total.group(1) if total else '?'}, decisions section kept "
          f"({len(kept.splitlines())} lines)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
