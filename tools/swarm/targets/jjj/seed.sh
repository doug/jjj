#!/usr/bin/env bash
#
# Seed a swarm workbench for the jjj self-improvement target.
#
# The workbench is a **clone** of the jjj repository, never the repository
# itself: a nine-agent fleet editing the tree we ship from is more blast radius
# than an experiment deserves. Nothing an agent does can reach `origin`; the
# swarm has its own bare remote, and whatever survives is merge-gated by hand.
#
# Usage: seed.sh <workbench-dir> <source-repo> <jjj-binary>

set -euo pipefail

ROOT="${1:?workbench directory required}"
SRC="${2:?source repo required}"
JJJ="${3:?jjj binary required}"

rm -rf "$ROOT"
mkdir -p "$ROOT"

# A clone at the current commit, with no `origin` pointing anywhere real.
git clone -q --no-hardlinks "$SRC" "$ROOT"
cd "$ROOT"
git remote remove origin 2>/dev/null || true
git config user.name "swarm-seed"
git config user.email "swarm-seed@example.invalid"

# The fitness function lives at the repo root, where every target's score
# script is expected to be.
cp "$SRC/tools/swarm/targets/jjj/score.sh" "$ROOT/score.sh"
chmod +x "$ROOT/score.sh"

jj git init --colocate >/dev/null 2>&1
jj config set --repo user.name "swarm-seed" >/dev/null 2>&1
jj config set --repo user.email "swarm-seed@example.invalid" >/dev/null 2>&1
"$JJJ" init >/dev/null

new_problem() {
    "$JJJ" problem new "$1" --priority "$2" --tags "$3" --force --json \
        | python3 -c 'import sys,json; print(json.load(sys.stdin)["id"])'
}

# One hard problem, stated as a measurement and a requirement — deliberately
# NOT as a diagnosis. Naming the responsible code path would delete the search,
# and the search is what this trial exists to measure.
main=$(new_problem \
    "Make jjj sync cost proportional to the delta, not the corpus" \
    critical "perf,sync,keystone")

# Sub-problems are directions, not instructions: each is a place to look, so
# agents that pick different ones explore genuinely different approaches.
new_problem "Reduce jjj's own time in the push path" high "perf,sync,push" >/dev/null
# The fetch path is already delta-proportional (1.0x); seeding it as open work
# would send agents to re-solve a solved problem.
new_problem "Make push validation incremental rather than a full-corpus reload" \
    high "perf,sync,push,validation" >/dev/null
new_problem "Find which jjj operations scale with corpus size rather than delta size" \
    high "perf,sync,investigation" >/dev/null

cat > "$ROOT/SWARM.md" <<'BRIEF'
# The target

`jjj sync` must complete in under a second for a small delta. Design decision 3
calls this a **hard** requirement, because sync sits in the synchronous critical
path of every agent's loop.

Half of it is now fixed. **Push is what remains.**

## What is measured

`./score.sh` prints `<score> 100`, where the score is `100 / ratio` and the
ratio is CPU time on a 5,000-entity corpus divided by CPU time on a 500-entity
one, for a 50-file delta. 1.0x means the cost no longer depends on how much
history exists. The worse of push and fetch sets the score.

    push   ~2.2x    <- the remaining problem
    fetch   1.0x    <- solved; leave it alone
    score  ~47/100

CPU time rather than wall-clock, because this runs on a machine the swarm
itself saturates and contention does not tax both corpora equally. Compare your
before and after within a turn; do not compare against a number from another run.

## What is already known

Fetch used to be O(corpus) for the same reason it looked mysterious: a pod that
only ever fetched never advanced its merge base, so every fetch paid a full
cold-start reconcile. That is fixed. `docs/design/sync-scaling-investigation.md`
records the investigation, and its analysis of the **push** path is accurate
about where the cost is.

**One approach has already been tried and rejected — do not repeat it.** Push
spends its time in a full markdown→SQLite reload used to validate before
publishing. Skipping that reload when the SQLite cache is "clean" makes the
number fall and is **wrong**: the dirty flag means "a sync was interrupted", not
"the markdown has not changed". A clean-but-stale cache then passes validation,
and a dangling reference or a conflict-marked body reaches every clone.
`./score.sh` will now score that change zero and name the failing test.

The honest version is to make validation itself delta-proportional — reload and
validate only what changed since the cache was written — rather than to skip it.
That is harder, and it is the actual problem.

## How work lands

Nothing reaches the shared branch except through review:

    jjj solution new "..." --body "what and why"  --problem <id>
    jjj solution attach <id>      # link your jj change
    jjj solution submit <id>      # publishes the diff for a reviewer
    -> a critic reviews the real diff and approves, or critiques it
    -> approved work is merged to main automatically

Your own pushes go to your own branch. An unreviewed change helps nobody.

## Rules

- Measure before you optimise, and again after. `./score.sh` is the arbiter.
- A critique must cite a number or a failing input, not an opinion.
- Speed bought with correctness scores zero, and the scorer checks for it.
- Several approaches are plausible. If another agent is already pursuing one,
  take a different one — rival approaches are the point, duplicates are not.
BRIEF

git add -A
git commit -q -m "seed: jjj self-improvement target"

echo "workbench: $ROOT"
echo "baseline:  $(cd "$ROOT" && ./score.sh 2>/dev/null | tail -1)"
echo "problems:  $("$JJJ" problem list --json | python3 -c 'import sys,json;print(len(json.load(sys.stdin)))')"
