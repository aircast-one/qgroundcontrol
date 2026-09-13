#!/bin/bash
# Refuse to commit while the shared index holds anything.
#
#   tools/index-guard.sh            check before committing
#
# Four Claude sessions share this working tree and therefore one .git/index.
# A staged file belongs to whoever staged it, and `git diff HEAD` cannot see
# staging at all, so nothing else warns you before your commit carries it.
#
# This refuses rather than filtering. A filter has to be right about which
# paths are yours; a refusal does not have to be right about anything.
staged=$(git diff --cached --name-only)
[ -z "$staged" ] && exit 0

echo "the shared index is armed against someone and a commit would carry it:" >&2
git diff --cached --stat >&2
echo "" >&2
echo "the working tree is not the question - 'git diff HEAD' cannot see this." >&2
echo "if it is not a deliberate staging of yours, and the content is reachable" >&2
echo "from a commit (git rev-parse :<path>, then git log --all -S<content>)," >&2
echo "clear it with: git reset -q HEAD -- <files>" >&2
exit 1
