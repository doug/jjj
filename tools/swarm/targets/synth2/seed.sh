#!/usr/bin/env bash
#
# Seed the five-lever synthetic decomposition target.
#
# **One problem is seeded, deliberately.** Every other target hands the fleet a
# ready-made backlog, which means the decomposition — the part that actually
# needs several minds — was done before the trial started. Here it is the thing
# under test: the fleet has to find the structure of the work itself, say which
# parts matter, and spread across them.
#
# This is `synth` with the flaw removed. That target had one lever; six agents
# pulled it in ten minutes and the score sat at its ceiling for the rest of the
# hour, so the A/B trial run on it could not tell its arms apart. Here the cost
# is spread over five independent levers and full marks is unreachable by
# construction.
#
# `reference/` is NOT copied: it holds a worked answer.
#
# Usage: seed.sh <workbench-dir> <repo-root> <jjj-binary>

set -euo pipefail

ROOT="${1:?workbench directory required}"
REPO="${2:?repo root required}"
JJJ="${3:?jjj binary required}"
HERE="$(cd "$(dirname "$0")" && pwd)"

rm -rf "$ROOT"; mkdir -p "$ROOT"
cp -r "$HERE/fixture" "$ROOT/fixture"
rm -rf "$ROOT/fixture/__pycache__"
cp "$HERE/score.sh" "$HERE/verify.sh" "$HERE/groundtruth.sh" "$ROOT/"
chmod +x "$ROOT/score.sh" "$ROOT/verify.sh" "$ROOT/groundtruth.sh"
printf '__pycache__/\n*.pyc\n' > "$ROOT/.gitignore"

cd "$ROOT"
git init -q .
git config user.name "swarm-seed"
git config user.email "swarm-seed@example.invalid"
git add -A && git commit -q -m "seed: synthetic pipeline (five levers)"
jj git init --colocate >/dev/null 2>&1 || true
jj config set --repo user.name "swarm-seed" >/dev/null 2>&1 || true
jj config set --repo user.email "swarm-seed@example.invalid" >/dev/null 2>&1 || true
"$JJJ" init >/dev/null

MILESTONE="Make the pipeline cheap"
"$JJJ" milestone new "$MILESTONE" --body "The work this trial is scored on." >/dev/null

"$JJJ" problem new "The pipeline costs 1,460,004 operations; make it cost far fewer" \
    --priority critical --tags "cost,keystone" --milestone "$MILESTONE" --force \
    --body "That is the whole brief. Nobody has broken this down yet — working
out what the parts are, which of them matter, and in what order is the first
piece of work, not a preliminary to it.

\`./score.sh\` shows where the operations go, site by site. Read it before
conjecturing. Two things worth knowing before you start:

- The cost is **spread**. No single change takes most of it; the largest site is
  about a fifth. A fix that helps one site leaves the others exactly where they
  were.
- One of the functions in here is long, ugly, and costs four operations." >/dev/null

cat > "$ROOT/SWARM.md" <<'BRIEF'
# The target

`fixture/pipeline.py` costs 1,460,004 operations for a fixed workload. Make it
cost far fewer, without changing the answer.

## The score

`./score.sh` prints `<score> 100` and shows where the operations go. Baseline is
18. The scale is logarithmic between a 3,000,000-operation budget and a
50,000-operation floor, so it keeps paying all the way down.

**You cannot reach 100.** The floor is set below anything a correct program can
achieve — parsing the records at all costs 60,000 operations, and no correct
answer can skip a record. This is deliberate: the previous version of this target
had an attainable ceiling, the fleet hit it in ten minutes, and the rest of the
run measured nothing. Treat the score as a direction, not a target.

**Cost is counted, not timed.** `ops.tick` charges operations, so the number is
identical on an idle machine and a saturated one. Do not optimise for wall clock;
nothing here is timed.

**Correctness gates everything.** `measure.py` works out the expected answer
independently, from the raw records, without touching pipeline code. An
optimisation that changes the answer scores zero however cheap it is.

**`fixture/ops.py` is the meter, not the program.** Editing it scores zero.
Reducing a `tick` count without removing the work it stands for is the same
thing done more quietly — a reviewer should treat it as a wrong answer.

## The shape of the work

The cost is spread across several independent sites, and they want *different
kinds* of change — restructuring, a better data structure, moving work out of a
loop, doing less of it. Fixing one leaves the others untouched. A fleet that
finds one is a fifth of the way.

This is why the decomposition matters more than any single fix, and why six
agents on the same site is worse than six agents on six.

## What is actually being tested

One problem is seeded, not seven. Every other trial in this harness handed the
fleet a ready-made backlog — which meant the decomposition was done before the
trial began. Here it is the work:

- **Split it up.** `jjj problem new "..." --parent <id>` makes a sub-problem.
  Ground each in something you measured.
- **Record what you measured.** `jjj finding new <problem> "..." --method "..."`
  keeps a profile where the next agent will find it. An investigation filed as a
  solution gets withdrawn as "not fixed", and the measurement is lost with it.
- **Say what matters.** `jjj rank set <problem>...` and `jjj rank move`. A
  ranking is a judgement about what will matter to what comes after — not a
  re-statement of the biggest number in the profile.
- **Spread out.** `jjj contention` shows where the fleet is doubled up and where
  nobody is. In an earlier trial nobody ranked anything, six agents converged on
  one problem while others sat untouched, and 62% of solutions were withdrawn as
  superseded. That waste was concentration, not competition.

## How work lands

One integrator accepts and merges; nobody else does. Critics raise objections
and sign off, but a solution becomes accepted when the integrator has read the
open critiques and judged what survives. A solution is never scored or voted on:
an objection either stands, is answered, or is refuted.

    jjj solution new "..." --body "what and why" --problem <id>
    jjj solution attach <id>
    jjj solution submit <id>      # publishes the diff for review

## Rules

- Profile before conjecturing. One function is long and ugly and costs 4 ops.
- Measure before and after, in the same turn, and put the numbers in the body.
- A critique must cite an operation count or a wrong answer, not an opinion.
BRIEF

score="$(./score.sh 2>/dev/null | tail -1)"
echo "baseline: $score"
echo "problems: $("$JJJ" problem list --json | python3 -c 'import sys,json;print(len(json.load(sys.stdin)))')"
