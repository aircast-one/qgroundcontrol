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

# Every git command below means the SHARED index. Inherit a caller's GIT_INDEX_FILE and they
# all quietly mean a private one instead: the guard would inspect the private index, and the
# refreshes would resync the very index they are resyncing away from -- succeeding, reporting
# truthfully, and leaving the shared index exactly as armed as it was. That is how the phantom
# this script exists to absorb got created: a peer DID reset after each commit, inside the
# shell that still had the private index exported.
unset GIT_INDEX_FILE

if (( $# == 0 )); then
    print -u2 "usage: $0 <file> [<file> ...]   # commit message on stdin"
    exit 2
fi

# A file whose INDEX entry differs from HEAD while its WORKING TREE matches HEAD is not a staged
# revert -- it is the default index left behind by someone landing through a private
# GIT_INDEX_FILE, which moves HEAD and never updates it. That phantom is a reverse-diff of a
# peer's LANDED work and reads exactly like somebody staging a deletion of it. It blocked this
# script four times in one session before it was told the difference. Refreshing those entries
# destroys nothing: HEAD and the worktree already agree on every byte.
phantom=()
real=()
for staged in ${(f)"$(git diff --cached HEAD --name-only)"}; do
    if git diff --quiet HEAD -- "$staged"; then phantom+=("$staged"); else real+=("$staged"); fi
done
if (( ${#phantom} )); then
    print "refreshing ${#phantom} stale index entr$( (( ${#phantom} == 1 )) && print "y" || print "ies") left by a private-index commit: ${phantom}"
    git reset -q HEAD -- "${phantom[@]}"
fi
# "differs in both" was read as "therefore a real staging" and that inference is wrong. A peer who
# lands through a private index and then keeps editing leaves an entry that is stale AND has a
# changed worktree, which git status spells MM exactly like a live staging. Twice in one day that
# sentence sent someone looking for work in flight that had already landed -- and a plain `git add
# -A` on top of it would have re-applied the reverse diff of a landed commit for the second time.
# What separates them is not the worktree: it is whether the INDEX blob is the file's content at a
# commit HEAD already contains. A deliberate staging is content that has never been committed.
staleAtAncestor() {
    local blob=$(git ls-files -s -- "$1" | awk '{print $2}')
    [[ -n $blob ]] || return 1
    local commit
    for commit in ${(f)"$(git rev-list -n 50 HEAD -- "$1")"}; do
        [[ $(git rev-parse -q --verify "$commit:$1" 2>/dev/null) == $blob ]] && return 0
    done
    return 1
}
if (( ${#real} )); then
    stale=()
    deliberate=()
    for staged in "${real[@]}"; do
        if staleAtAncestor "$staged"; then stale+=("$staged"); else deliberate+=("$staged"); fi
    done
    print -u2 "the shared index is armed against someone and this commit would carry it:"
    print -u2 "$(git diff --cached HEAD --stat -- "${real[@]}")"
    print -u2 ""
    if (( ${#stale} )); then
        print -u2 "STALE, NOT STAGED: the index entry for ${stale} is that file's content at a commit"
        print -u2 "HEAD already contains, so it is a private-index leftover whose worktree has moved on"
        print -u2 "-- not work in flight. Committing over it with git add -A re-applies a landed revert."
    fi
    if (( ${#deliberate} )); then
        print -u2 "NEVER COMMITTED: ${deliberate} holds index content matching no ancestor, so somebody"
        print -u2 "staged it deliberately. Ask before touching it."
    fi
    print -u2 "index only, worktree untouched either way: git reset -q HEAD -- <files>"
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
# A head source that swift-checks cannot compile reached master unbuilt once, in 2f5c086e1, and
# blocked every session. Scoped to the files NAMED in this commit rather than to the tree, so a
# peer's uncommitted edit in macos/Sources is never mine to refuse.
if ! python3 "$root/tools/macos/built.py" "$@"; then
    print -u2 ""
    print -u2 "refusing to commit: a head source named here has not compiled since it was edited."
    print -u2 "swift-checks covers 74 of 116 files and cannot see the rest; build build-test with"
    print -u2 "cmake LAST in the command, grep the log for 'error:', then commit."
    exit 1
fi

if print -rl -- "$@" | grep -q '^macos/'; then
    if ! python3 "$root/tools/macos/stale-state.py"; then
        print -u2 "refusing to commit: a field outlives the selection that set it."
        exit 1
    fi
    if ! python3 "$root/tools/macos/models-cover-views.py"; then
        print -u2 "refusing to commit: a view root this head reads has no model in head_models.MODELS."
        print -u2 "view-fields.py and null-fallbacks.py both skip what is not in it, silently."
        exit 1
    fi
    python3 "$root/tools/macos/head-reads.py" > /dev/null
    if ! git diff --quiet HEAD -- tools/macos/head-reads.txt; then
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

# EXIT 3, NOT 1. Every refusal above this line means nothing was committed and the fix is to fix
# the thing and run again. These two run AFTER the commit is in history, so 1 here would tell a
# caller the same thing as a refusal and invite a retry that lands the work twice. Proven that a
# zsh script can finish its work and still exit non-zero: a script edited while running completes
# on the buffered text and then dies parsing the tail, which is how a peer's gate reported failure
# for suites that had all passed. The exit code has to say which side of the commit it failed on.
for check in "git diff --cached HEAD --stat" "git diff HEAD --stat -- $*"; do
    left="$(eval "$check")"
    if [[ -n "$left" ]]; then
        print -u2 "THE COMMIT LANDED -- do not run this again, it would commit twice."
        print -u2 "But afterwards '$check' is not empty:"
        print -u2 "$left"
        exit 3
    fi
done
print "committed; shared index clean and the working tree matches HEAD"
