#!/usr/bin/env python3
"""Refuse to commit a head source that has not been through a compiler since it was edited.

2f5c086e1 went to master with a Swift compile error and blocked every session. Two things had
to go wrong together. swift-checks.sh was green and always would have been -- PlanWindow is one
of the nineteen files it cannot compile, a limit I had written into that very commit's message.
And I read an exit code of 0 from a backgrounded `... ; cmake --build ... ; echo $? ; tail`,
which is TAIL'S status. cmake had already failed.

So the rule this enforces is: a file compiles, or it does not get committed. Not "swift-checks
passed", which covers 74 of 116 files, and not "the build said zero", which said no such thing.

WHAT IT COMPARES, and why this artifact rather than the two more obvious ones.

NOT the app binary. Four sessions build build-test, so its clock moves for reasons that have
nothing to do with this file.

NOT the per-file <name>.swift.o. Measured, and it is WRONG: after a clean full rebuild,
FenceRally.swift.o was OLDER than FenceRally.swift, because swiftc does not rewrite an object
whose contents did not change. A guard on per-file objects refuses files that compiled
perfectly. That is the version of this script that would have shipped if I had not checked it
against a build I already knew the answer for.

libQGCNativeUI.a is the whole-module archive. Swift here compiles macos/Sources as ONE module,
so the archive advances only when every file in it compiled. Verified by firing both branches
rather than by reasoning about ninja: appending a deliberate type error to PlanWindow.swift and
building gave cmake rc=1, five errors, and the archive's mtime UNCHANGED at 1789315570 before
and after.

THE SHARED TREE MAKES THIS SOUND RATHER THAN BREAKING IT. Four sessions, one worktree: a peer's
build compiles whatever is in macos/Sources at that moment, including my uncommitted edits. So
"the archive is newer than my file" means my file has been through a compiler, no matter who
started it -- and a failed build leaves the archive where it was.

WHAT IT CANNOT SEE: whether the file is correct, only that it parses and type-checks; and
anything outside macos/Sources, since Tests/main.swift belongs to a different module that
swift-checks compiles on every run anyway.

Usage: python3 tools/macos/built.py <file> [<file> ...]
"""
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parents[2]
ARCHIVE = ROOT / "build-test/Debug/lib/libQGCNativeUI.a"
SOURCES = "macos/Sources/"


def stale(paths):
    if not ARCHIVE.exists():
        return None
    built = ARCHIVE.stat().st_mtime
    return [p for p in paths if (ROOT / p).exists() and (ROOT / p).stat().st_mtime > built]


named = [p for p in sys.argv[1:] if p.startswith(SOURCES) and p.endswith(".swift")]
if not named:
    sys.exit(0)

behind = stale(named)
if behind is None:
    print(f"  NOT BUILT {ARCHIVE.relative_to(ROOT)} does not exist, so nothing says these files "
          f"compile. Build build-test before committing them", file=sys.stderr)
    sys.exit(1)

for path in behind:
    print(f"  UNBUILT {path} is newer than the module archive, so it has not been through a "
          f"compiler since it was edited. swift-checks cannot compile every head file and a "
          f"build's exit code is easy to read off the wrong process -- build build-test, grep "
          f"the log for 'error:', and commit after that", file=sys.stderr)

print(f"checked {len(named)} head source(s) against the module archive: {len(behind)} have not "
      f"been through swiftc since they were edited")
sys.exit(1 if behind else 0)
