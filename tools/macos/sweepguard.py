"""Anchor a sweep to the repository and refuse when it read nothing.

Android's readinesskeys.py was pointed at a path from when their head lived in a sibling
repo. It resolved to a directory that does not exist and printed "this sweep checked
nothing" for as long as anyone cared to look; aimed at the real tree it immediately found
a dead subscription to the discredited setupComplete flag. Measured here the same night:
orphan-sweep.py, decoded-unread.py and qml-inventory.py all took RELATIVE paths, so run
from anywhere but the repository root they swept an empty list and reported "zero uses in
Sources: 0" -- a sentence indistinguishable from a clean result.

Both halves matter. anchored() removes the dependency on the caller's directory, and
refuse() makes an empty input an exit-2 failure rather than a zero, because a count of
nothing agrees with every theory about the tree it did not read.
"""

import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parents[2]


def anchored(*parts):
    return ROOT.joinpath(*parts)


def refuse(**inputs):
    empty = sorted(name for name, found in inputs.items() if not found)
    if not empty:
        return
    counted = ", ".join(f"{name}={len(found)}" for name, found in sorted(inputs.items()))
    print(f"  REFUSING to judge: {' and '.join(empty)} came back empty ({counted}), under "
          f"{ROOT}. That is this sweep failing to read its inputs, not a clean tree -- every "
          f"count below would be zero and every zero would read as good news.", file=sys.stderr)
    sys.exit(2)
