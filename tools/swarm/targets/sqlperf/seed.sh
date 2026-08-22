#!/usr/bin/env bash
#
# Seed a swarm workbench for the SQL latency target.
#
# The engine it starts from is a working one — a swarm wrote it in a single turn
# against the correctness version of this target. Correctness is therefore the
# starting point rather than the goal, and the whole run goes into the part that
# does not saturate.
#
# Usage: seed.sh <workbench-dir> <repo-root> <jjj-binary>

set -euo pipefail

ROOT="${1:?workbench directory required}"
REPO="${2:?repo root required}"
JJJ="${3:?jjj binary required}"
HERE="$(cd "$(dirname "$0")" && pwd)"

rm -rf "$ROOT"
mkdir -p "$ROOT"

cp "$HERE/harness.py" "$HERE/runner.py" "$HERE/spec.py" "$ROOT/"
cp "$HERE/score.sh" "$ROOT/score.sh"
cp "$HERE/verify.sh" "$ROOT/verify.sh"
chmod +x "$ROOT/score.sh" "$ROOT/verify.sh"
cp -r "$HERE/../sql/reference/sqlengine" "$ROOT/sqlengine"
rm -rf "$ROOT/sqlengine/__pycache__"

cat > "$ROOT/.gitignore" <<'IGN'
__pycache__/
*.pyc
data/
IGN

cd "$ROOT"
git init -q .
git config user.name "swarm-seed"
git config user.email "swarm-seed@example.invalid"
git add -A
git commit -q -m "seed: sql latency workbench"
jj git init --colocate >/dev/null 2>&1 || true
jj config set --repo user.name "swarm-seed" >/dev/null 2>&1 || true
jj config set --repo user.email "swarm-seed@example.invalid" >/dev/null 2>&1 || true
"$JJJ" init >/dev/null

new_problem() {
    "$JJJ" problem new "$1" --priority "$2" --tags "$3" --body "$4" --force --json \
        | python3 -c 'import sys,json; print(json.load(sys.stdin)["id"])'
}

new_problem "Index the tables at load time" critical "lookup,indexes,keystone" \
"Every query is a full scan today, so a point lookup by primary key costs the
same as counting the table. \`bulk_load\` is not timed — building whatever
structures you want there is free, and that is the point.

This is the piece the other problems sit on: a join or a subquery gets fast
because a lookup got fast. Expect several agents here at once; land something
small and correct early." >/dev/null

new_problem "Hash joins instead of nested loops" critical "join,algorithms" \
"A nested loop over these tables is minutes, not seconds — at a fiftieth of
this data it already takes five. Build the smaller side into a hash table and
probe with the larger one.

LEFT JOIN needs the unmatched left rows NULL-extended, and a NULL join key
matches nothing on either side." >/dev/null

new_problem "Push predicates down, and stop materialising columns you discard" high "scan,planner" \
"\`WHERE users.city = 'lima'\` applied after a join does the join for rows that
were never going to survive it. Filter first. The same argument applies to
projection: building whole rows to return two columns is most of the cost of a
scan." >/dev/null

new_problem "Top-N without sorting the table" high "topn,algorithms" \
"\`ORDER BY price DESC LIMIT 10\` over a million rows sorts a million rows. A
bounded heap does it in one pass with ten items of state." >/dev/null

new_problem "Aggregate in one pass" high "group,algorithms" \
"GROUP BY should hash once, not sort or scan per group. HAVING is a filter on
the aggregated result and should not re-read anything." >/dev/null

new_problem "Do not re-run a subquery for every row" high "sub,planner" \
"\`WHERE user_id IN (SELECT ...)\` should evaluate the inner query once into a
set. Executing it per outer row is the difference between milliseconds and
minutes." >/dev/null

new_problem "topn_group returns the wrong rows" high "correctness,topn" \
"\`SELECT sku, COUNT(*) FROM items GROUP BY sku ORDER BY COUNT(*) DESC LIMIT 10\`
disagrees with SQLite on this data. Ordering by an aggregate is the suspect.

Correctness still gates everything: a wrong answer scores zero however fast it
is." >/dev/null

new_problem "Store columns rather than rows" medium "scan,storage" \
"A list of tuples costs a pointer chase per field and builds a tuple per row
even when two columns are wanted. Columnar storage makes scans and aggregates
much cheaper, and is a large change — worth conjecturing against the simpler
wins first, and worth measuring rather than assuming." >/dev/null

cat > "$ROOT/SWARM.md" <<'BRIEF'
# The target

Make a working SQL engine fast. Correctness is already there — a swarm wrote
this engine in a single turn — so the whole run goes into latency.

## The score

`./score.sh` prints `<score> 100`. A query counts only if it is **correct and
inside its budget**; wrong and fast is worth nothing.

Meeting the budget is worth **half** the marks. The rest arrives logarithmically
as you get further under it, and full marks need to be **eight times inside**.
There is no point at which the score stops paying for being faster.

Six classes, weighted equally, so no single win carries the run:

| class | what it exercises |
|---|---|
| lookup | point and selective predicates — indexes |
| scan | full scans and projection |
| group | GROUP BY, HAVING, aggregates |
| join | INNER and LEFT joins — hash joins |
| topn | ORDER BY ... LIMIT — bounded heaps |
| sub | IN (SELECT ...) — evaluate once |

`./score.sh` with `SQLPERF_VERBOSE=1` lists every query, its time, its budget,
and SQLite's time on the same query.

## The rules of the measurement

- **Loading is not timed.** `bulk_load` may build any structure you like —
  indexes, sorted columns, dictionaries. Only queries are timed.
- **A query is abandoned once it misses its budget**, so a slow one costs you
  its marks, not the run.
- **SQLite's time is shown for context, not as the target.** It has no indexes
  here, so on some joins you are expected to beat it; on scans it is C and you
  will not. The budget is the requirement.
- `sqlite3` cannot be imported, and neither can numpy, polars or pandas. This
  target is about the algorithms; vectorising with someone else's C would say
  nothing about the engine.

## Where the time goes today

Baseline is around 9/100. Joins, top-N and subqueries score zero — every one of
them times out — and the fastest thing in the workload is still a full scan.
The first index is worth more than any amount of micro-optimisation.

## How work lands

Nothing reaches the shared branch except through review:

    jjj solution new "..." --body "what and why"  --problem <id>
    jjj solution attach <id>      # link your jj change
    jjj solution submit <id>      # publishes the diff for a reviewer
    -> a critic reviews the real diff and approves, or critiques it
    -> approved work is merged to main automatically

## Rules

- Measure before and after, in the same turn. Timings move with machine load,
  so a number from another turn is not a comparison.
- A critique must cite a query, its time, and its budget — not an opinion.
- The six classes are deliberately independent. If another agent is inside the
  join code, take a different class.
- A change that makes one class faster and another slower is a trade, not a
  win. Report both.
BRIEF

score="$(./score.sh 2>/dev/null | tail -1)"
echo "baseline: $score"
echo "problems: $("$JJJ" problem list --json | python3 -c 'import sys,json;print(len(json.load(sys.stdin)))')"
