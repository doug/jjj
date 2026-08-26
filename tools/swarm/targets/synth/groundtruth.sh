#!/usr/bin/env bash
#
# Does the scorer measure what it claims?
#
# The check whose absence cost more than any bug in the agents' code: a fitness
# function is the one artifact a swarm cannot critique, so an error inside it is
# invisible to the whole process and amplified by it.
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
cp fixture/pipeline.py /tmp/pipe.bak
python3 - <<'PY'
import pathlib
p = pathlib.Path("fixture/pipeline.py"); s = p.read_text()
p.write_text(s.replace("def stage_count(records):\n    n = 0",
                       "def stage_count(records):\n    return 0\n    n = 0", 1))
PY
cheap="$(./score.sh 2>/dev/null | tail -1 | cut -d' ' -f1)"
cp /tmp/pipe.bak fixture/pipeline.py
[ "$cheap" = "0" ] && ok "a wrong answer scores 0 even though it is cheaper" \
                   || bad "wrong answer scored $cheap"

# And the score must rise for a real improvement.
cp fixture/pipeline.py /tmp/pipe.bak
python3 - <<'PY'
import pathlib
p = pathlib.Path("fixture/pipeline.py"); s = p.read_text()
p.write_text(s.replace('tick("decode.parse", 3)', 'tick("decode.parse")', 1))
PY
better="$(./score.sh 2>/dev/null | tail -1 | cut -d' ' -f1)"
cp /tmp/pipe.bak fixture/pipeline.py
[ -n "$better" ] && [ "$better" -gt "$base" ] && ok "score rises for a real improvement ($base -> $better)" \
                                              || bad "score did not rise ($base -> $better)"

printf '  ground truth: %d passed, %d failed\n' "$pass" "$fail"
[ "$fail" -eq 0 ]
