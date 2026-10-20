#!/bin/sh
# Checks that the branch being committed to or pushed is `main` or follows
# Conventional Branch (https://conventional-branch.github.io/).
set -eu

# A push checks the remote ref and a commit the current branch. Tags and a
# detached HEAD, as during a rebase, pass.
ref=${PRE_COMMIT_REMOTE_BRANCH:-$(git symbolic-ref -q HEAD || true)}
case $ref in
refs/heads/*) branch=${ref#refs/heads/} ;;
*) exit 0 ;;
esac

if ! printf '%s\n' "$branch" |
  grep -Eqx 'main|(feature|feat|bugfix|fix|hotfix|release|chore)/[a-z0-9]+([.-][a-z0-9]+)*'; then
  echo "Branch '$branch' must be main or <type>/<description>:" >&2
  echo "type is feature, feat, bugfix, fix, hotfix, release or chore;" >&2
  echo "description is lowercase letters, digits, and single hyphens or dots between them." >&2
  exit 1
fi
