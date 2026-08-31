#!/usr/bin/env bash
#
# Does the scorer measure what it claims?
#
# The check whose absence cost more than any bug in the agents' code: a fitness
# function is the one artifact a swarm cannot critique, so an error inside it is
# invisible to the whole process and amplified by it.
#
# NOTE: this file is copied into the agents' workbench, so it must not contain
# the answers. The checks here are structural. The one that needs an optimised
# reference — "is the ceiling actually out of reach?" — lives in
# `reference/ceiling.sh`, which is never seeded.
set -uo pipefail
cd "$(dirname "$0")"
pass=0; fail=0
ok()  { pass=$((pass+1)); printf '    ok   %s\n' "$1"; }
bad() { fail=$((fail+1)); printf '    FAIL %s\n' "$1"; }

base="$(./score.sh 2>/dev/null | tail -1 | cut -d' ' -f1)"
[ -n "$base" ] && [ "$base" -gt 0 ] && ok "baseline scores $base (non-zero, so broken is distinguishable)" \
                                    || bad "baseline scores ${base:-nothing}"

# The count must be identical run to run — that is the whole reason for counting
# rather than timing.
a="$(cd fixture && python3 measure.py | python3 -c 'import json,sys;print(json.load(sys.stdin)["ops"])')"
b="$(cd fixture && python3 measure.py | python3 -c 'import json,sys;print(json.load(sys.stdin)["ops"])')"
[ "$a" = "$b" ] && ok "cost is deterministic ($a ops twice)" || bad "cost varied: $a then $b"

# A wrong answer must score zero however cheap it is.
cp fixture/pipeline.py /tmp/pipe2.bak
python3 - <<'PY'
import pathlib
p = pathlib.Path("fixture/pipeline.py"); s = p.read_text()
p.write_text(s.replace("def stage_count(records):\n    \"\"\"How many records",
                       "def stage_count(records):\n    return 0\n    \"\"\"How many records", 1))
PY
cheap="$(./score.sh 2>/dev/null | tail -1 | cut -d' ' -f1)"
cp /tmp/pipe2.bak fixture/pipeline.py
[ "$cheap" = "0" ] && ok "a wrong answer scores 0 even though it is cheaper" \
                   || bad "wrong answer scored $cheap"

# And the score must rise for a real improvement.
cp fixture/pipeline.py /tmp/pipe2.bak
python3 - <<'PY'
import pathlib
p = pathlib.Path("fixture/pipeline.py"); s = p.read_text()
p.write_text(s.replace('tick("decode.parse", 3)', 'tick("decode.parse")', 1))
PY
better="$(./score.sh 2>/dev/null | tail -1 | cut -d' ' -f1)"
cp /tmp/pipe2.bak fixture/pipeline.py
[ -n "$better" ] && [ "$better" -gt "$base" ] && ok "score rises for a real improvement ($base -> $better)" \
                                              || bad "score did not rise ($base -> $better)"

# Editing the meter must not pay. `ops.py` is not part of the program under
# optimisation: changing it does not make anything cheaper, it makes the
# measurement lie.
cp fixture/ops.py /tmp/ops2.bak
printf '\n# tampered\n' >> fixture/ops.py
tampered="$(./score.sh 2>/dev/null | tail -1 | cut -d' ' -f1)"
cp /tmp/ops2.bak fixture/ops.py
[ "$tampered" = "0" ] && ok "editing the meter scores 0" \
                      || bad "a modified ops.py still scored $tampered"

# No single lever may dominate. The first synthetic target had exactly one, so
# the fleet pulled it in ten minutes and the score sat at its ceiling for the
# rest of the run — and an A/B trial on it could not tell its arms apart. This
# target is only worth running while the cost stays spread.
python3 - <<'PY'
import json, subprocess, sys
out = subprocess.run([sys.executable, "measure.py"], cwd="fixture",
                     capture_output=True, text=True).stdout
d = json.loads(out)
total = d["ops"]
sites = d["by_site"]
biggest = max(sites.values())
share = 100 * biggest / total
big = [k for k, v in sites.items() if v >= 20000]
if share > 40:
    print(f"    FAIL one site is {share:.0f}% of all cost — the target has a single lever")
    raise SystemExit(1)
if len(big) < 5:
    print(f"    FAIL only {len(big)} sites carry real cost; class coverage has nothing to measure")
    raise SystemExit(1)
print(f"    ok   cost is spread over {len(big)} sites, largest {share:.0f}% — no single lever")
PY
if [ $? -eq 0 ]; then pass=$((pass+1)); else fail=$((fail+1)); fi

printf '  ground truth: %d passed, %d failed\n' "$pass" "$fail"
[ "$fail" -eq 0 ]
