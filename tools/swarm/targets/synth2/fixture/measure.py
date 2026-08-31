#!/usr/bin/env python3
"""Run the workload and report what it cost, and whether it was correct.

Correctness is a fixed expected result, computed independently of the pipeline —
straight from the raw records, touching no pipeline code. An optimisation that
changes the answer is not an optimisation, and the scorer refuses it however
cheap it is.
"""

import json
import sys

import ops
import pipeline
import workload

# Duplicated from pipeline on purpose. If the expected answer imported the
# pipeline's constants, an agent could "optimise" by editing them and the check
# would agree with the change it was supposed to catch.
KINDS = (
    "a", "A", "bb", "Bb", "ccc", "d", "D",
    "e", "ff", "Gg", "hhh", "i",
)
STEPS = (
    (10, 1), (20, 2), (30, 3), (40, 4), (50, 5),
    (60, 6), (70, 7), (80, 8), (90, 9), (100, 10),
)


def expected(recs):
    """The answer, computed the plain way — no pipeline code involved."""
    parsed = [r.split("|") for r in recs]

    total = 0
    for p in parsed:
        size = int(p[2])
        weight = 1
        for threshold, w in STEPS:
            if size >= threshold:
                weight = w
        total += size * weight

    return {
        "count": sum(1 for p in parsed if p[1] in KINDS),
        "sum": total,
        "groups": len({p[3] for p in parsed}),
        "filtered": sum(
            1 for p in parsed if p[1].strip().lower().startswith("b") and int(p[2]) > 50
        ),
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
