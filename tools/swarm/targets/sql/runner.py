#!/usr/bin/env python3
"""Run the engine against the corpus with the oracle out of reach.

This process never learns an expected result, and cannot reach `sqlite3` — or
any obvious route back to it — because the fastest way to score 100% on a
differential test is to *be* the reference implementation. That is not a
hypothetical: a swarm twice "improved" a benchmark by deleting the correctness
check it was measured against, and an agent that can import the oracle will.

A blocked import raises `ImportError` inside the engine, so an engine that
tries it fails loudly rather than scoring well.

  runner.py <corpus.json> <per-case-timeout-seconds>   -> results JSON on stdout
"""

import importlib.abc
import json
import pathlib
import signal
import sys

BLOCKED = {"sqlite3", "subprocess", "ctypes", "_sqlite3", "sqlalchemy",
           "duckdb", "pandas", "os.system"}


class _Blocker(importlib.abc.MetaPathFinder):
    """Refuse the oracle however it is asked for.

    A `sys.modules` entry alone is not enough: `importlib.reload`, a fresh
    `__import__`, or a C-level import can route around it. Sitting at the front
    of `sys.meta_path` catches every path that resolves a module name.
    """

    def find_spec(self, fullname, path=None, target=None):
        root = fullname.split(".")[0]
        if fullname in BLOCKED or root in BLOCKED:
            raise ImportError(
                f"'{fullname}' is not available to the engine: it is the oracle "
                f"this target is measured against, or a route to it. Implement "
                f"the semantics instead."
            )
        return None


sys.meta_path.insert(0, _Blocker())

# `os` itself stays available — engines legitimately need it — but the two
# process-spawning doors in it do not.
import os  # noqa: E402

for _name in ("system", "popen", "execv", "execvp", "spawnv", "posix_spawn"):
    if hasattr(os, _name):
        setattr(os, _name, None)


def main():
    corpus = json.loads(pathlib.Path(sys.argv[1]).read_text())
    timeout = float(sys.argv[2]) if len(sys.argv) > 2 else 2.0

    sys.path.insert(0, os.getcwd())
    try:
        from sqlengine import Database
    except Exception as e:
        print(json.dumps({"__import_error__": repr(e)}))
        return

    results = {}
    try:
        db = Database()
        for stmt in corpus["schema"] + corpus["data"]:
            db.execute(stmt)
    except Exception as e:
        print(json.dumps({"__setup_error__": repr(e)}))
        return

    # A per-case wall-clock budget. Without one, a single query that loops
    # forever takes the whole run down and the score reads 0 with no clue which
    # case did it; with one, that case simply fails and the rest still score.
    def _timeout(signum, frame):
        raise TimeoutError("case exceeded its time budget")

    signal.signal(signal.SIGALRM, _timeout)

    for case in corpus["cases"]:
        signal.setitimer(signal.ITIMER_REAL, timeout)
        try:
            rows = db.execute(case["sql"])
            results[str(case["id"])] = {"rows": [list(r) for r in (rows or [])]}
        except Exception as e:
            results[str(case["id"])] = {"error": type(e).__name__}
        finally:
            signal.setitimer(signal.ITIMER_REAL, 0)

    print(json.dumps(results))


if __name__ == "__main__":
    main()
