#!/usr/bin/env bash
# Compile every commit on this branch in a throwaway worktree.
#
# Working agreement §0.1 of docs/plans/2026-10-19-swarm-core-plan.md requires
# that each commit build on its own, not merely in aggregate. A branch whose
# commits only compile at the tip hides the moment a caller was introduced
# before the function it calls, which is invisible to `cargo check` at HEAD and
# breaks `git bisect` for everyone afterwards.
set -uo pipefail

BASE="${1:-origin/main}"
D="$(mktemp -d)"
WT="$D/wt"
export CARGO_TARGET_DIR="$D/verify-target"   # shared, so deps compile once
trap 'git worktree remove --force "$WT" 2>/dev/null; rm -rf "$D"' EXIT

fail=0
range="$BASE..HEAD"
count=$(git log --format=%h "$range" | wc -l | tr -d ' ')
echo "VERIFY_RANGE=$range  commits=$count"

while read -r sha subject; do
  [ -z "$sha" ] && continue
  if ! git worktree add --detach -q "$WT" "$sha" 2>/dev/null; then
    echo "WT_FAIL=$sha  $subject"
    fail=1
    continue
  fi
  if (cd "$WT" && cargo check --all-targets --all-features -q 2>&1 | tail -20); then
    echo "COMPILES=$sha  $subject"
  else
    echo "BROKEN=$sha  $subject"
    fail=1
  fi
  git worktree remove --force "$WT" 2>/dev/null
done < <(git log --format='%h %s' --reverse "$range")

echo "VERIFY_EXIT=$fail"
exit "$fail"
