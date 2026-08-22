#!/usr/bin/env bash
#
# Is this tree safe to publish?
#
# Cheap on purpose: it runs before every push and before every merge of an
# approved solution, so it checks that the code loads and answers a trivial
# query — not that it is fast, and not that it is right, which is what
# ./score.sh is for.

set -uo pipefail
cd "$(dirname "$0")"

python3 -m compileall -q sqlengine >/dev/null 2>&1 || exit 1

python3 - <<'PY' || exit 1
import sys
sys.path.insert(0, ".")
from sqlengine import Database

db = Database()
db.execute("CREATE TABLE _t (a INTEGER, b TEXT)")
db.bulk_load("_t", [(1, "x"), (2, None)])
rows = db.execute("SELECT a, b FROM _t")
assert sorted(rows) == [(1, "x"), (2, None)], rows
PY
