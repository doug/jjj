#!/usr/bin/env python3
"""Load the data into the engine and time the workload, oracle out of reach.

Loading is not timed — bulk-loading a million rows is not what this measures,
and an engine that spends thirty seconds building indexes up front and then
answers every query in a millisecond is doing exactly what it should. Only the
queries are timed, each against its own budget.

  runner.py <data-dir> <workload.json>   -> {name: {"ms": float, "digest": str}} on stdout
"""

import csv
import importlib.abc
import json
import pathlib
import signal
import sys
import time

BLOCKED = {"sqlite3", "_sqlite3", "subprocess", "ctypes", "duckdb",
           "sqlalchemy", "pandas", "polars", "numpy"}


class _Blocker(importlib.abc.MetaPathFinder):
    """Refuse the oracle, and the libraries that would do the work for you.

    numpy and polars are blocked alongside sqlite3 on purpose: this target is
    about the algorithms — indexes, hash joins, pushdown — and vectorising with
    someone else's C is a different exercise that would also make the result
    say nothing about the engine.
    """

    def find_spec(self, fullname, path=None, target=None):
        root = fullname.split(".")[0]
        if fullname in BLOCKED or root in BLOCKED:
            raise ImportError(
                f"'{fullname}' is not available here: it is either the oracle "
                f"this target is measured against or a library that would do "
                f"the work. Implement the algorithm."
            )
        return None


sys.meta_path.insert(0, _Blocker())

import os  # noqa: E402

for _n in ("system", "popen", "execv", "execvp", "spawnv", "posix_spawn"):
    if hasattr(os, _n):
        setattr(os, _n, None)

sys.path.insert(0, os.getcwd())
from spec import SCHEMA, coerce, ddl, digest  # noqa: E402


def main():
    data = pathlib.Path(sys.argv[1])
    workload = [tuple(x) for x in json.loads(pathlib.Path(sys.argv[2]).read_text())]
    try:
        from sqlengine import Database
    except Exception as e:
        print(json.dumps({"__error__": f"import: {e!r}"}))
        return

    try:
        db = Database()
        for t in SCHEMA:
            db.execute(ddl(t))
            with (data / f"{t}.csv").open() as f:
                r = csv.reader(f)
                next(r)
                rows = [tuple(coerce(v, ty) for v, (_, ty) in zip(row, SCHEMA[t]))
                        for row in r]
            db.bulk_load(t, rows)
    except Exception as e:
        print(json.dumps({"__error__": f"load: {e!r}"}))
        return

    # Abandon a query once it has missed its budget. Without this the run does
    # not finish: a nested-loop join over these tables takes minutes even at a
    # fiftieth of full scale, so the honest report — "too slow" — would arrive
    # hours late, and scoring has to be cheap enough to run twice a turn.
    #
    # A little headroom over the budget, so a query that is merely marginal is
    # reported as SLOW with a real number rather than as a timeout.
    def _expired(signum, frame):
        raise TimeoutError("budget exceeded")

    signal.signal(signal.SIGALRM, _expired)

    out = {}
    for name, sql, budget, _cls in workload:
        signal.setitimer(signal.ITIMER_REAL, budget * 1.5 + 0.05)
        try:
            t0 = time.perf_counter()
            rows = db.execute(sql)
            ms = (time.perf_counter() - t0) * 1000
            out[name] = {"ms": ms,
                         "digest": digest(rows or [], " order by " in sql.lower())}
        except TimeoutError:
            out[name] = {"error": "Timeout"}
        except Exception as e:
            out[name] = {"error": type(e).__name__}
        finally:
            signal.setitimer(signal.ITIMER_REAL, 0)
    print(json.dumps(out))


if __name__ == "__main__":
    main()
