#!/usr/bin/env bash
#
# Fitness for the SQL-engine target.
#
# Prints "<score> <ceiling>" like every other target. The score is the mean of
# five tier percentages — equal weight per tier, not per case, so a fleet cannot
# maximise it by grinding the easy tier while the hard semantics that make this
# a long target go untouched.
#
# **The corpus scored against is generated fresh, with a random seed, on every
# run.** The workbench ships a `dev_corpus.json` with a fixed seed to iterate
# against; that one is visible, and memorising it is possible, which is exactly
# why it is not the one that counts. Same generator, same distribution, so the
# two track each other closely — but a lookup table keyed on query text scores
# nothing here. This is an ordinary train/test split, and it is load-bearing:
# an agent optimising against a gate it can defeat will defeat it.
#
# Sampling variance across 2,000 cases is under a point or two; compare a before
# and after from the same turn, as with every other target here.

set -uo pipefail
cd "$(dirname "$0")"

fail() { echo "0 100"; [ -n "${1:-}" ] && echo "$1" >&2; exit 0; }

[ -d sqlengine ] || fail "no sqlengine/ package found"

# Belt and braces with runner.py's import blocker: this catches the attempt in
# review, where a person can see it, rather than only at run time.
if grep -rnE '^[[:space:]]*(import|from)[[:space:]]+(sqlite3|_sqlite3|subprocess|ctypes|duckdb|sqlalchemy|pandas)\b' \
        sqlengine/ 2>/dev/null | head -3; then
    fail "the engine imports the oracle, or a route to it — implement the semantics instead"
fi

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

seed="$(python3 -c 'import random; print(random.randrange(1, 2**31))')"
python3 harness.py gen --out "$tmp/eval.json" --seed "$seed" \
    --cases "${SQL_CASES:-2000}" >/dev/null 2>&1 \
    || fail "could not generate the evaluation corpus"

python3 harness.py score --corpus "$tmp/eval.json" --engine "$PWD" \
    --timeout "${SQL_TIMEOUT:-2}"
