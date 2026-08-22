#!/usr/bin/env bash
#
# Fitness for the SQL latency target.
#
# A query counts only if it is **correct and inside its budget**. Correctness
# comes from SQLite over the same data; the budget is absolute, and stated in
# the workload rather than derived from SQLite's own timings — SQLite is C with
# a B-tree, and a multiple of its milliseconds would be either unreachable or
# meaningless depending on the query.
#
# Meeting a budget is worth half the marks. The rest arrives logarithmically as
# you get further under it, and full marks need eight times inside. That is on
# purpose: the previous version of this target scored correctness alone, could
# be maxed out, and was — 99 out of 100 in ten minutes. A ceiling you can reach
# is a checklist.
#
# The dataset is generated on first use rather than committed: it is 45MB, it is
# identical everywhere because the seed is fixed, and keeping it out of git
# keeps every clone and merge fast.

set -uo pipefail
cd "$(dirname "$0")"

fail() { echo "0 100"; [ -n "${1:-}" ] && echo "$1" >&2; exit 0; }

[ -d sqlengine ] || fail "no sqlengine/ package found"

if grep -rnE '^[[:space:]]*(import|from)[[:space:]]+(sqlite3|_sqlite3|subprocess|ctypes|duckdb|sqlalchemy|pandas|polars|numpy)\b' \
        sqlengine/ 2>/dev/null | head -3; then
    fail "the engine imports the oracle, or a library that would do the work"
fi

if [ ! -f data/manifest.json ]; then
    echo "generating the dataset (once, ~20s)..." >&2
    python3 harness.py gen --out data --scale "${SQLPERF_SCALE:-1.0}" >&2 \
        || fail "could not generate the dataset"
fi

python3 harness.py score --data data --engine "$PWD" ${SQLPERF_VERBOSE:+-v}
