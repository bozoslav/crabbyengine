#!/usr/bin/env bash
# Adds one result document to the benchmark history branch and pushes it:
#
#   scripts/bench-history-push.sh <bench_track> <result.json> <path-in-history>
#
# <path-in-history> is `<machine>/<commit>.json` for a commit on main, or
# `releases/<tag>/<machine>.json` for a release. The branch (BENCH_HISTORY_BRANCH,
# default `bench-history`) holds only these files and the README that
# `bench_track history --write` makes from them; it shares no history with
# main. It is created on the first push. Run from a checkout whose remote
# (BENCH_HISTORY_REMOTE, default `origin`) accepts the push; in CI that is the
# workflow's own GITHUB_TOKEN with `contents: write`. Another run pushing at
# the same time makes the push fail: it then starts again from the new tip,
# up to five times.
set -euo pipefail

if [ "$#" -ne 3 ]; then
  echo "usage: $0 <bench_track> <result.json> <path-in-history>" >&2
  exit 2
fi
bench_track=$(cd "$(dirname "$1")" && pwd)/$(basename "$1")
result=$(cd "$(dirname "$2")" && pwd)/$(basename "$2")
target=$3
case "$target" in
  /* | *..* | "")
    echo "the path in the history must be relative, without '..': $target" >&2
    exit 2
    ;;
esac
branch=${BENCH_HISTORY_BRANCH:-bench-history}
remote=${BENCH_HISTORY_REMOTE:-origin}
work=$(mktemp -d)
# The first push starts the branch from a temporary local orphan branch.
orphan="bench-history-tmp-$$"
# shellcheck disable=SC2329 # called by the EXIT trap
cleanup() {
  git worktree remove --force "$work/tree" >/dev/null 2>&1 || true
  git branch -D "$orphan" >/dev/null 2>&1 || true
  rm -rf "$work"
}
trap cleanup EXIT

for attempt in 1 2 3 4 5; do
  git worktree remove --force "$work/tree" >/dev/null 2>&1 || true
  git branch -D "$orphan" >/dev/null 2>&1 || true
  rm -rf "$work/tree"
  if git ls-remote --exit-code --heads "$remote" "$branch" >/dev/null; then
    git fetch --quiet "$remote" "refs/heads/$branch"
    git worktree add --quiet --detach "$work/tree" FETCH_HEAD
  else
    git worktree add --quiet --detach "$work/tree"
    git -C "$work/tree" switch --quiet --orphan "$orphan"
  fi
  mkdir -p "$work/tree/$(dirname "$target")"
  cp "$result" "$work/tree/$target"
  "$bench_track" history "$work/tree" --write
  git -C "$work/tree" add -A
  git -C "$work/tree" \
    -c user.name="github-actions[bot]" \
    -c user.email="41898282+github-actions[bot]@users.noreply.github.com" \
    commit --quiet -m "Record $target"
  if git -C "$work/tree" push --quiet "$remote" "HEAD:refs/heads/$branch"; then
    echo "recorded $target on $branch"
    exit 0
  fi
  echo "push attempt $attempt failed; trying again from the new tip" >&2
  sleep $((attempt * 3))
done
echo "could not push $target to $branch" >&2
exit 1
