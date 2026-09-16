#!/usr/bin/env python3
"""Runs a mutation twice and compares the two outcomes, because one run cannot tell you anything.

A control is valid iff the result DIFFERS between the mutated and unmutated versions, and
differs in the assertion you named. Checking only that the mutated run is red leaves three
ways to be fooled, and each has caught this session or the core in one night:

  the outcome did not change    a sample both mechanisms produced, so removing one changed
                                nothing and the control passed while proving nothing
  it changed in the wrong place  a red from an older guard, not from the rule under test
  it never happened              the mutation did not apply at all -- a mis-escaped anchor,
                                a shadowing local -- and the green was about an unmutated tree

So this does not report a colour. It reports the SET of assertion labels that failed after and
not before, refuses when that set is empty, and refuses when --expect names something no moved
label contains. The anchor count is asserted first, so a mutation that did not apply is a
refusal rather than a pass.

The file is restored whether or not anything above succeeds, because a restore I have to
remember is a restore I will forget while reading a diff.

WHAT THIS DOES NOT COVER, said plainly: it compares swift-checks runs, so it can only judge a
control over a SWIFT assertion. The control that fooled me worst was in a Python tool -- a
sample both the explicit list and a *Model* glob produced, so deleting the glob changed nothing
-- and this would not have caught it, because neither run goes through swift-checks at all. A
control over a tool's own guard still has to be run by hand, and the same three questions apply
to it: did the outcome change, in the place I named, from a mutation that actually applied.

Usage:
  python3 tools/macos/mutate.py <file> <old> <new> [--expect <substring of the label>]
"""
import pathlib
import re
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent.parent
FAILURE = re.compile(r'^FAIL (.*)$', re.MULTILINE)


def failures() -> set[str]:
    run = subprocess.run(['zsh', str(ROOT / 'tools/macos/swift-checks.sh')],
                         capture_output=True, text=True, cwd=ROOT)
    return set(FAILURE.findall(run.stdout + run.stderr))


def main() -> int:
    argv = sys.argv[1:]
    expect = ''
    if '--expect' in argv:
        at = argv.index('--expect')
        expect = argv[at + 1] if at + 1 < len(argv) else ''
        argv = argv[:at] + argv[at + 2:]
    if len(argv) != 3:
        print(__doc__.strip().rsplit('Usage:', 1)[-1].strip(), file=sys.stderr)
        return 2
    path, old, new = ROOT / argv[0], argv[1], argv[2]

    original = path.read_text()
    if original.count(old) != 1:
        print(f"{argv[0]} holds that text {original.count(old)} times, not once, so the mutation "
              "would land somewhere I did not choose or nowhere at all", file=sys.stderr)
        return 2

    try:
        before = failures()
        path.write_text(original.replace(old, new))
        after = failures()
    finally:
        path.write_text(original)

    moved = sorted(after - before)
    if not moved:
        print(f"INVALID CONTROL: the outcome did not change. {len(before)} failing before, "
              f"{len(after)} after, none of them new. The mutation applied and nothing noticed, "
              "so whatever is being claimed here is not pinned by any assertion", file=sys.stderr)
        return 1
    if expect and not any(expect in label for label in moved):
        print(f"WRONG PLACE: {len(moved)} assertion(s) moved and none mentions {expect!r}. A red "
              "from somewhere else is not evidence about the rule under test:", file=sys.stderr)
        for label in moved:
            print(f"    {label[:120]}", file=sys.stderr)
        return 1

    print(f"{len(moved)} assertion(s) moved, and only these:")
    for label in moved:
        print(f"    {label[:130]}")
    if before:
        print(f"  ({len(before)} were already failing before the mutation and are excluded)")
    return 0


sys.exit(main())
