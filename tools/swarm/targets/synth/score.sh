#!/usr/bin/env bash
#
# Fitness for the synthetic decomposition target.
#
# Cost is counted, never timed: `ops.tick` charges operations, so the number is
# identical on an idle machine and a saturated one. Every timing-based fitness
# function in this harness has had to be rewritten after contention made an
# unchanged tree score anywhere from 0 to 28.
#
# Correctness gates everything, and is computed independently of the pipeline —
# `measure.py` works the expected answer out the plain way from the raw records.
# An "optimisation" that changes the answer scores zero.

set -uo pipefail
cd "$(dirname "$0")"

fail() { echo "0 100"; [ -n "${1:-}" ] && echo "$1" >&2; exit 0; }

out="$(cd fixture && python3 measure.py 2>/tmp/measure.err)" || {
    fail "measure failed: $(tail -2 /tmp/measure.err)"
}

python3 - "$out" <<'PY'
import json, math, sys

# A budget and a floor, not the current number. Anchoring the top of the scale
# to what the code costs today would put the starting score at exactly zero,
# which tells an agent nothing about whether it is getting warmer and makes a
# minimal tree indistinguishable from a broken one.
BUDGET = 1_000_000          # over this scores nothing; the tree starts at 700,004
FLOOR = 60_000              # full marks at or under — reachable, but only by
                            # fixing the shared decoder rather than the stages

d = json.loads(sys.argv[1])
if not d.get("correct"):
    print("the pipeline no longer produces the right answer", file=sys.stderr)
    print("0 100")
    raise SystemExit

ops = d["ops"]
# Logarithmic between baseline and floor, so the scale keeps paying all the way
# down instead of saturating once the obvious win is taken.
if ops >= BUDGET:
    score = 0
else:
    frac = math.log(BUDGET / ops) / math.log(BUDGET / FLOOR)
    score = max(0, min(100, round(100 * frac)))

print(f"  ops {ops:,} of {BUDGET:,} budget  ({100*ops//BUDGET}%)", file=sys.stderr)
for k, v in list(d["by_site"].items())[:5]:
    print(f"    {k:24s} {v:>9,}", file=sys.stderr)
print(f"{score} 100")
PY
