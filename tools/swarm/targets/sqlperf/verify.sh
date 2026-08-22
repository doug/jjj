#!/usr/bin/env bash
#
# Is this tree safe to publish?
#
# Runs before every push and before every merge of an approved solution, so it
# has to be fast — a second or two, not the fifteen that ./score.sh costs.
#
# It checks **correctness, not speed**, on a deliberately tiny dataset. An
# earlier version only checked that the engine imported and answered
# `SELECT a, b FROM t`, which let a real regression through: two of the three
# top-N queries on the shared branch started returning wrong rows, because
# ORDER BY ... DESC put NULLs at the wrong end. Agents had filed five critiques
# about it and it merged anyway, since nothing between "it imports" and "the
# full benchmark" was looking.
#
# Semantics do not depend on scale, so 1% data catches the same bug in a
# fiftieth of the time.

set -uo pipefail
cd "$(dirname "$0")"

python3 -m compileall -q sqlengine >/dev/null 2>&1 || exit 1

[ -f data/tiny/manifest.json ] || \
    python3 harness.py gen --out data/tiny --scale 0.01 --seed 7 >/dev/null 2>&1 || exit 1

# A ratchet, not a fixed bar. The seed engine has known bugs — one of them is a
# seeded problem — so demanding perfection here would reject everything. What
# must hold is that correctness never goes *down*: the floor starts wherever the
# workbench starts and rises as the swarm fixes things.
FLOOR_FILE=.correctness_floor
floor="$(cat "$FLOOR_FILE" 2>/dev/null || echo 0)"

now="$(python3 harness.py score --data data/tiny --engine "$PWD" --correctness-only 2>/dev/null \
       | tail -1 | cut -d' ' -f1)"
case "$now" in ''|*[!0-9]*) echo "verify: could not score correctness" >&2; exit 1 ;; esac

if [ "$now" -lt "$floor" ]; then
    echo "verify: correctness went backwards ($floor -> $now on the sample workload)." >&2
    echo "        Run: python3 harness.py score --data data/tiny --engine \$PWD --correctness-only -v" >&2
    exit 1
fi

# Raise the floor when it improves, so a fix cannot be silently undone later.
[ "$now" -gt "$floor" ] && printf '%s\n' "$now" > "$FLOOR_FILE"
exit 0
