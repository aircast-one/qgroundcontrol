"""Every view root this head reads has an entry in head_models.MODELS, or a reason here.

MODELS is hand-maintained and cannot report its own drift, which is the failure mode that
left SETTINGS_GROUPS eleven groups behind on Android and twelve properties out of
kFactProperties. Two instruments key off it -- view-fields.py asks whether every key a
model reads is one the core still emits, and null-fallbacks.py asks what a model does with
the ones emitted null -- so a missing entry makes BOTH blind to the same model, silently.

It drifts exactly when nobody has reason to think of it. view.instrumentGroups went
unlisted in 497034412, a commit whose whole subject was that view, written by someone who
had just read this file. That is the argument for the check rather than for more care.

Deliberately path-level. Enumerating Swift structs means parsing Swift, and that ambiguity
produces findings like a Q_PROPERTY's last token reading as a field nobody serves. A view
root either appears in a Bridge.group("view.x") call or it does not.
"""

import pathlib
import re
import sys

sys.path.insert(0, str(pathlib.Path(__file__).parent))
from head_models import MODELS  # noqa: E402

# A view root read with no model decoding it. Each needs a reason, not just an absence.
NO_MODEL = {
    "view.label": "read by Labels.swift alone as a humanising lookup, not decoded into a "
                  "struct. There are no keys for view-fields.py to check and no nulls for "
                  "null-fallbacks.py to judge, so an entry would add a name and no coverage",
}


def read_roots(sources):
    found = {}
    for path in sorted(sources.glob("*.swift")):
        for match in re.findall(r'Bridge\.group\("(view\.[a-zA-Z.]+)', path.read_text()):
            found.setdefault(match.split("(")[0], set()).add(path.name)
    return found


def covered_roots():
    roots = set()
    for value in MODELS.values():
        for path in value if isinstance(value, list) else [value]:
            roots.add(path.split("(")[0])
    return roots


def main():
    sources = pathlib.Path(__file__).parent.parent.parent / "macos" / "Sources"
    read = read_roots(sources)
    # An empty read is a broken run, not a head that stopped reading views. Without this the stale
    # branch below fires for EVERY entry in NO_MODEL -- none of them is in an empty `read` -- and
    # instructs whoever is looking to delete reasons that are all still correct. The summary line
    # would say "0 uncovered" beside it, which is the reassuring half. Same shape as head-reads.py
    # writing an empty artefact: a run that could not read its inputs has to say so rather than
    # report what it computed from nothing.
    if not read:
        print(f"  REFUSING to judge: no Bridge.group(\"view.*\") call found under {sources}. That "
              f"is this script failing to read its inputs, not the head having stopped reading "
              f"views, and every reason in NO_MODEL would otherwise be reported as stale.")
        return 2
    covered = covered_roots()
    missing = {root: files for root, files in read.items()
               if root not in covered and root not in NO_MODEL}

    stale = sorted(root for root in NO_MODEL if root not in read)
    for root in stale:
        print(f"  {root} carries a reason for having no model but is no longer read at all. "
              f"Drop the reason with the read.")

    for root, files in sorted(missing.items()):
        print(f"  {root} is read by {', '.join(sorted(files))} and has no entry in "
              f"head_models.MODELS, so view-fields.py and null-fallbacks.py both skip it. "
              f"Add the model that decodes it, or a reason in NO_MODEL.")

    print(f"checked {len(read)} view roots this head reads against {len(covered)} in MODELS: "
          f"{len(missing)} uncovered, {len(NO_MODEL)} excused with a reason")
    return 1 if missing or stale else 0


if __name__ == "__main__":
    sys.exit(main())
