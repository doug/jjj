#!/usr/bin/env bash
#
# Is the ceiling out of reach, and is the range worth measuring?
#
# Kept out of the seeded workbench because it contains a worked answer. Run it
# from the repository whenever the fixture or the scale changes.
set -uo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
TARGET="$(cd "$HERE/.." && pwd)"
pass=0; fail=0
ok()  { pass=$((pass+1)); printf '    ok   %s\n' "$1"; }
bad() { fail=$((fail+1)); printf '    FAIL %s\n' "$1"; }

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
mkdir -p "$work/fixture"
cp "$TARGET"/fixture/*.py "$TARGET/fixture/.ops.sha" "$work/fixture/"
cp "$TARGET/score.sh" "$work/"

base="$(cd "$work" && ./score.sh 2>/dev/null | tail -1 | cut -d' ' -f1)"
cp "$HERE/optimised_pipeline.py" "$work/fixture/pipeline.py"
best="$(cd "$work" && ./score.sh 2>/dev/null | tail -1 | cut -d' ' -f1)"
ops="$(cd "$work/fixture" && python3 measure.py | python3 -c 'import json,sys;print(json.load(sys.stdin)["ops"])')"

[ -n "$best" ] && [ "$best" -lt 100 ] \
    && ok "a fully optimised tree scores $best, not 100 — the ceiling is out of reach" \
    || bad "an optimised tree scores ${best:-nothing}; the scale saturates, as synth did"

[ -n "$best" ] && [ -n "$base" ] && [ $(( best - base )) -ge 40 ] \
    && ok "range is $base -> $best, wide enough to separate partial progress" \
    || bad "range ${base:-?} -> ${best:-?} is too narrow to measure anything"

[ "${ops:-0}" -gt 50000 ] \
    && ok "the optimum costs $ops ops, above the $((50000)) full-marks floor" \
    || bad "the optimum ($ops ops) reaches the full-marks floor"

printf '  ceiling: %d passed, %d failed\n' "$pass" "$fail"
[ "$fail" -eq 0 ]
