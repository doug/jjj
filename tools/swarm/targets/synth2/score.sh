#!/usr/bin/env bash
#
# Fitness for the five-lever synthetic decomposition target.
#
# Cost is counted, never timed: `ops.tick` charges operations, so the number is
# identical on an idle machine and a saturated one. Every timing-based fitness
# function in this harness has had to be rewritten after contention made an
# unchanged tree score anywhere from 0 to 28.
#
# Correctness gates everything, and is computed independently of the pipeline —
# `measure.py` works the expected answer out the plain way from the raw records.
# An "optimisation" that changes the answer scores zero.
#
# # Why the ceiling is out of reach
#
# The first synthetic target set full marks at 60,000 operations, which was
# *attainable*: six agents reached 60,004 in ten minutes, the score sat at 100
# for the rest of the hour, and an A/B trial run on it could not distinguish its
# two arms because both finished at the ceiling. A metric with a reachable
# ceiling stops discriminating the moment anyone reaches it.
#
# So FLOOR here is deliberately below anything a correct tree can achieve.
# `decode.parse` is charged 3 operations for each of the 20,000 records and no
# correct answer can skip a record, so 60,000 is a hard lower bound; a fully
# optimised reference implementation with all five levers pulled measures
# 100,004. Full marks would need 50,000. It is unreachable by construction, and
# the scale keeps paying all the way down instead of saturating.

set -uo pipefail
cd "$(dirname "$0")"

fail() { echo "0 100"; [ -n "${1:-}" ] && echo "$1" >&2; exit 0; }

# The meter is not part of the program under optimisation. Editing `ops.py` does
# not make anything cheaper, it makes the measurement lie — and the fitness
# function is the one artifact a swarm cannot critique, so it has to defend
# itself.
meter="$(shasum fixture/ops.py 2>/dev/null | awk '{print $1}')"
expected_meter="$(cat fixture/.ops.sha 2>/dev/null)"
if [ -n "$expected_meter" ] && [ -n "$meter" ] && [ "$meter" != "$expected_meter" ]; then
    fail "fixture/ops.py has been modified — the meter is not part of the program"
fi

out="$(cd fixture && python3 measure.py 2>/tmp/measure.err)" || {
    fail "measure failed: $(tail -2 /tmp/measure.err)"
}

python3 - "$out" <<'PY'
import json, math, sys

# A budget and a floor, not the current number. Anchoring the top of the scale to
# what the code costs today would put the starting score at exactly zero, which
# tells an agent nothing about whether it is getting warmer and makes a minimal
# tree indistinguishable from a broken one.
BUDGET = 3_000_000          # over this scores nothing; the tree starts at 1,460,004
FLOOR = 50_000              # full marks — deliberately BELOW the 60,000-operation
                            # parse floor, so 100 cannot be reached and the scale
                            # never saturates. A fully optimised reference with
                            # all five levers pulled measures 100,004, or 83.

d = json.loads(sys.argv[1])
if not d.get("correct"):
    print("the pipeline no longer produces the right answer", file=sys.stderr)
    print("0 100")
    raise SystemExit

ops = d["ops"]
# Logarithmic between budget and floor, so the scale keeps paying all the way
# down instead of saturating once the obvious win is taken.
if ops >= BUDGET:
    score = 0
else:
    frac = math.log(BUDGET / ops) / math.log(BUDGET / FLOOR)
    score = max(0, min(100, round(100 * frac)))

print(f"  ops {ops:,} of {BUDGET:,} budget  ({100*ops//BUDGET}%)", file=sys.stderr)
for k, v in list(d["by_site"].items())[:8]:
    print(f"    {k:24s} {v:>9,}", file=sys.stderr)
print(f"{score} 100")
PY
