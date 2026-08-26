#!/usr/bin/env python3
"""Run the workload and report what it cost, and whether it was correct.

Correctness is a fixed expected result, computed independently of the pipeline:
an optimisation that changes the answer is not an optimisation.
"""

import json
import sys

import ops
import pipeline
import workload


def expected(recs):
    """The answer, computed the plain way — no pipeline code involved."""
    parsed = [r.split("|") for r in recs]
    return {
        "count": sum(1 for p in parsed if p[1] == "a"),
        "sum": sum(int(p[2]) for p in parsed),
        "groups": len({p[3] for p in parsed}),
        "filtered": sum(1 for p in parsed if p[1] == "b" and int(p[2]) > 50),
    }


def main():
    recs = workload.records()
    want = expected(recs)
    ops.reset()
    got = pipeline.run(recs)
    result = {
        "ops": ops.total(),
        "correct": got == want,
        "by_site": ops.by_site(),
    }
    if not result["correct"]:
        result["want"] = want
        result["got"] = got
    print(json.dumps(result))
    return 0 if result["correct"] else 1


if __name__ == "__main__":
    sys.exit(main())
