#!/bin/zsh
# Commit named files through a private index, and refuse while the shared index is armed.
#
# Three sessions share one worktree AND one git index. A commit made through a private
# GIT_INDEX_FILE never updates the shared one, so the shared index goes on describing
# whatever tree it last saw: a staged revert of work that has already landed, invisible to
# `git diff HEAD` because that compares the working tree. Four such reverts appeared in a
# single day, none of them found by the session whose work was staged for deletion.
#
# Usage: tools/macos/commit.sh <file> [<file> ...]   # message on stdin
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$root"

if (( $# == 0 )); then
    print -u2 "usage: $0 <file> [<file> ...]   # commit message on stdin"
    exit 2
fi

armed="$(git diff --cached HEAD --stat)"
if [[ -n "$armed" ]]; then
    print -u2 "the shared index is armed against someone and this commit would carry it:"
    print -u2 "$armed"
    print -u2 ""
    print -u2 "the working tree is not the question - 'git diff HEAD' cannot see this."
    print -u2 "check it is not a deliberate staging of yours, then: git reset -q HEAD -- <files>"
    exit 1
fi

# macos/Tests/main.swift is not a CMake target and swift-checks.sh was invoked by nothing, so
# its assertions ran only when someone remembered. Scoped to macos/ so a C++ session's commit
# pays none of the seventeen seconds.
if print -rl -- "$@" | grep -q '^macos/'; then
    if ! "$root/tools/macos/swift-checks.sh"; then
        print -u2 ""
        print -u2 "refusing to commit: the Swift checks are red and this commit touches macos/."
        print -u2 "they compile the working tree, so if you are not the session editing"
        print -u2 "macos/Sources the failure above may not be yours - say so rather than"
        print -u2 "working around it."
        exit 1
    fi
fi

# The core cannot derive which fields a head stopped reading -- only the head knows the names it
# references. This emits that set for them to diff, and is scoped to macos/ commits the same way
# the Swift checks are, so a C++ session never picks it up. Included only when it actually moved,
# and said out loud, because silently adding a file to someone's commit is what this script exists
# to prevent.
if print -rl -- "$@" | grep -q '^macos/'; then
    python3 "$root/tools/macos/head-reads.py" > /dev/null
    if ! git diff --quiet -- tools/macos/head-reads.txt; then
        set -- "$@" tools/macos/head-reads.txt
        print "including tools/macos/head-reads.txt: the names this head references changed"
    fi
fi

# A comparison against a tr()'d string is never true outside English, and this head wrote three
# of them in one session. The check is cheap and about half precise, so it gates only comparisons
# it has no reason for; the two it knows are real and unfixable here are printed, not blocking.
if print -rl -- "$@" | grep -q '^macos/'; then
    if ! python3 "$root/tools/macos/translated-join.py" > /dev/null; then
        print -u2 ""
        print -u2 "refusing to commit: a string this head compares against is one QGC translates."
        print -u2 "Key on an id, a class or a number. If it is a coincidence, say why in the table."
        exit 1
    fi
fi

message="$(cat)"
if [[ -z "${message// }" ]]; then
    print -u2 "refusing to commit with an empty message"
    exit 1
fi

private="${TMPDIR:-/tmp}/qgc-commit-index.$$"
trap 'rm -f "$private"' EXIT
GIT_INDEX_FILE="$private" git read-tree HEAD
GIT_INDEX_FILE="$private" git add -- "$@"
print -r -- "$message" | GIT_INDEX_FILE="$private" git commit -F -

# The private index is what committed, so the shared one still describes the tree from before:
# for a file that was untracked it now reads as a deletion of what just landed. Sync exactly
# the paths committed, which is the arming this script exists to stop anyone else inheriting.
git reset -q HEAD -- "$@"

for check in "git diff --cached HEAD --stat" "git diff HEAD --stat -- $*"; do
    left="$(eval "$check")"
    if [[ -n "$left" ]]; then
        print -u2 "after committing, '$check' is not empty:"
        print -u2 "$left"
        exit 1
    fi
done
print "committed; shared index clean and the working tree matches HEAD"
