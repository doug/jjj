#!/usr/bin/env bash
#
# Seed a swarm workbench for the SQL-engine target.
#
# The workbench is a fresh repository holding a nearly-empty engine, a test
# harness, and an oracle it cannot reach. Nothing here is a clone of anything
# real, so there is no blast radius to speak of.
#
# Usage: seed.sh <workbench-dir> <repo-root> <jjj-binary>

set -euo pipefail

ROOT="${1:?workbench directory required}"
REPO="${2:?repo root required}"
JJJ="${3:?jjj binary required}"
HERE="$(cd "$(dirname "$0")" && pwd)"

rm -rf "$ROOT"
mkdir -p "$ROOT/sqlengine"

cp "$HERE/harness.py" "$HERE/runner.py" "$ROOT/"
cp "$HERE/score.sh" "$ROOT/score.sh"
chmod +x "$ROOT/score.sh"
cp "$HERE/skeleton.py" "$ROOT/sqlengine/__init__.py"

cat > "$ROOT/.gitignore" <<'IGN'
__pycache__/
*.pyc
IGN

# A fixed-seed corpus to iterate against. The corpus that decides the score is
# generated fresh on every run — see score.sh — so this one can be read, run,
# and studied freely without becoming a way to fake progress.
python3 "$HERE/harness.py" gen --out "$ROOT/dev_corpus.json" --cases 2000 >/dev/null

cd "$ROOT"
git init -q .
git config user.name "swarm-seed"
git config user.email "swarm-seed@example.invalid"
git add -A
git commit -q -m "seed: sql engine workbench"
jj git init --colocate >/dev/null 2>&1 || true
jj config set --repo user.name "swarm-seed" >/dev/null 2>&1 || true
jj config set --repo user.email "swarm-seed@example.invalid" >/dev/null 2>&1 || true
"$JJJ" init >/dev/null

new_problem() {
    "$JJJ" problem new "$1" --priority "$2" --tags "$3" --body "$4" --force --json \
        | python3 -c 'import sys,json; print(json.load(sys.stdin)["id"])'
}

new_problem "Replace the regex parser with a real tokenizer and AST" critical "parser,keystone" \
"The skeleton parses SQL with two regexes. That is enough for \`SELECT * FROM t\`
and nothing beyond it — every tier above the first needs structure the regex
cannot express: precedence, nesting, expressions in the select list.

This is the piece everything else sits on, so it is also the piece most likely
to be worked on by several agents at once. Land something small and correct
early rather than something complete and late." >/dev/null

new_problem "Projection and comparison filters (tier 1)" high "tier1,expressions" \
"Select named columns rather than only \`*\`, and support WHERE with the six
comparison operators against integer literals." >/dev/null

new_problem "Boolean logic, ordering and limits (tier 2)" high "tier2,expressions" \
"AND, OR, NOT, IS NULL / IS NOT NULL, arithmetic in the select list, ORDER BY
with ASC/DESC, LIMIT and OFFSET.

ORDER BY is where sort stability and NULL placement start to matter: SQLite
sorts NULLs first ascending. Check rather than assume." >/dev/null

new_problem "Aggregates and grouping (tier 3)" high "tier3,aggregates" \
"COUNT(*), COUNT(col), SUM, AVG, MIN, MAX, GROUP BY, HAVING, DISTINCT.

COUNT(*) and COUNT(col) differ on NULLs, SUM of no rows is NULL rather than 0,
and AVG ignores NULLs. These are the cases the corpus is full of." >/dev/null

new_problem "Joins (tier 4)" high "tier4,joins" \
"INNER JOIN and LEFT JOIN with an ON condition, qualified column names, and
\`table.*\`. A LEFT JOIN emits NULL-extended rows for unmatched left rows —
including when the join key is itself NULL, which never matches." >/dev/null

new_problem "NULL is not a value (tier 5)" high "tier5,null-semantics" \
"Three-valued logic. \`x = NULL\` is never true, NOT NULL is NULL, and
\`NULL AND false\` is false while \`NULL AND true\` is NULL. WHERE keeps a row
only when the predicate is true — not when it is NULL.

\`NOT IN\` with a NULL in the list is the classic trap: it yields no rows." >/dev/null

new_problem "Type affinity and comparison across types (tier 5)" medium "tier5,types" \
"SQLite compares NULLs, then integers and reals together, then text, then blobs.
Integers and reals compare numerically; text never equals a number. Ordering
across mixed types follows that same class order." >/dev/null

new_problem "Subqueries, IN, BETWEEN, CASE, LIKE (tier 5)" medium "tier5,expressions" \
"Scalar and list subqueries in IN / NOT IN, BETWEEN, CASE WHEN with an ELSE,
LIKE with % and _, and string concatenation with ||." >/dev/null

cat > "$ROOT/SWARM.md" <<'BRIEF'
# The target

Implement a SQL engine in pure Python. Correctness is defined by **SQLite**:
the same schema, the same data, the same query, the same rows.

## The contract

The harness depends on exactly this, and it must not change:

```python
from sqlengine import Database
db = Database()
db.execute("CREATE TABLE t (a INTEGER, b TEXT)")   # -> None
db.execute("INSERT INTO t VALUES (1, 'x')")        # -> None
db.execute("SELECT * FROM t")                      # -> [(1, 'x')]
```

`execute` returns a list of tuples for a SELECT and None otherwise. **Raise on
anything you cannot answer.** Returning `[]` for a query you did not understand
looks identical to a query that genuinely matched no rows, and it hides the
failure from you and from your reviewer.

## The score

`./score.sh` prints `<score> 100` and a per-tier breakdown on stderr:

| tier | what it covers |
|---|---|
| 1 | projection and comparison filters |
| 2 | boolean logic, arithmetic, ORDER BY, LIMIT |
| 3 | aggregates, GROUP BY, HAVING, DISTINCT |
| 4 | INNER and LEFT joins |
| 5 | NULL semantics, type affinity, subqueries, LIKE, CASE |

The score is the **mean of the five tier percentages** — equal weight per tier,
not per case. Grinding tier 1 cannot carry a run, and the difficulty lives in
tiers 4 and 5. Rows are compared as a multiset unless the query has an ORDER BY,
because SQL promises no order without one.

## The corpus you can see is not the corpus you are scored on

`dev_corpus.json` has a fixed seed. Read it, run it, study it — that is what it
is for. The corpus that decides the score is generated fresh, with a random
seed, every time `./score.sh` runs. Same generator, same distribution, so the
two track each other closely.

This is a train/test split and it is deliberate. A lookup table keyed on query
text scores about 4 out of 100 here, however much of `dev_corpus.json` it
memorises.

For the same reason `sqlite3` — and `subprocess`, and `ctypes` — cannot be
imported by the engine. The import raises. Implement the semantics.

## How work lands

Nothing reaches the shared branch except through review:

    jjj solution new "..." --body "what and why"  --problem <id>
    jjj solution attach <id>      # link your jj change
    jjj solution submit <id>      # publishes the diff for a reviewer
    -> a critic reviews the real diff and approves, or critiques it
    -> approved work is merged to main automatically

Your own pushes go to your own branch. An unreviewed change helps nobody.

## Rules

- Measure before and after. `./score.sh` is the arbiter, not your reading of
  the spec — and SQLite is the spec, including where it surprises you.
- A critique must cite a failing query and the two results, not an opinion.
- The tiers are deliberately independent. If another agent is already inside the
  parser, take a tier instead — rival approaches are the point, duplicates
  are not.
- When SQLite does something that looks wrong, it is still the answer. Write the
  case down in your solution body; that is the most useful thing you can leave
  for the next agent.
BRIEF

score="$(./score.sh 2>/dev/null | tail -1)"
echo "baseline: $score"
echo "problems: $("$JJJ" problem list --json | python3 -c 'import sys,json;print(len(json.load(sys.stdin)))')"
