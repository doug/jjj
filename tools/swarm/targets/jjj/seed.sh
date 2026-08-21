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
new_problem "Reduce jjj's own time in the fetch path" high "perf,sync,fetch" >/dev/null
new_problem "Find which jjj operations scale with corpus size rather than delta size" \
    high "perf,sync,investigation" >/dev/null

cat > "$ROOT/SWARM.md" <<'BRIEF'
# The target

`jjj sync` must complete in under a second for a small delta. Design decision 3
calls this a **hard** requirement, because sync sits in the synchronous critical
path of every agent's loop. It is currently violated, and the reason is not where
it was assumed to be.

## What is measured

Profiling a delta sync (100 changed files) at a 25,000-entity corpus:

    delta_push:  11,898ms total — 1,367ms in jj (12 calls), 10,531ms in jjj
    delta_fetch:  5,826ms total — 2,014ms in jj  (7 calls),  3,812ms in jjj

**88% of a push is jjj's own work**, not jj and not subprocess overhead. The call
count is already near minimal, so this is not a batching problem.

Holding the delta constant at 100 files and growing the corpus:

| corpus | jjj's own time (push) | jj's time |
|---|---|---|
| 2,000 | 1,182ms | 932ms |
| 8,000 | 3,244ms | 972ms |
| 25,000 | 10,401ms | 1,395ms |

Linear in the **corpus**, flat in the **delta**. The work being done is
proportional to how much history exists rather than to how much changed.

## The score

`./score.sh` prints `<score> 100`, where the score is `100 / ratio` and the ratio
is jjj's own time on a 5,000-entity corpus divided by its time on a 500-entity
one, for a 50-file delta.

    baseline today: 26/100   (ratio ~3.8x)
    ratio 2.0x:     50/100
    ratio 1.0x:    100/100   cost no longer depends on corpus size

A ratio rather than milliseconds, because this is measured on a machine
saturated by the swarm itself: both sides suffer the same contention, so the
ratio survives where an absolute timing would not.

**Correctness gates the score.** A tree that fails to build, or fails the library
tests, scores zero however fast it is. Making sync fast by breaking it is not
progress, and the full suite runs at the merge gate regardless.

## What is not provided

The responsible code path. That is the problem.

## Rules

- Measure before you optimise, and again after. `./score.sh` is the arbiter.
- A critique must cite a number, not an opinion.
- `cargo test` must pass. Speed bought with correctness scores zero.
- Several approaches are plausible here. If another agent is already pursuing
  one, take a different one — rival approaches are the point, duplicates are not.
BRIEF

git add -A
git commit -q -m "seed: jjj self-improvement target"

echo "workbench: $ROOT"
echo "baseline:  $(cd "$ROOT" && ./score.sh 2>/dev/null | tail -1)"
echo "problems:  $("$JJJ" problem list --json | python3 -c 'import sys,json;print(len(json.load(sys.stdin)))')"
