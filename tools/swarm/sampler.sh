#!/usr/bin/env bash
#
# Record what the fleet is doing, every few minutes, to a TSV.
#
# Without this a run can only be judged by its end state, which is exactly the
# wrong summary for an experiment about whether a swarm *converges*: a fleet
# that reached a good answer at minute 20 and spent 100 minutes churning looks
# identical to one that improved steadily throughout. A trial ended with no
# time-series at all and the question was unanswerable after the fact.
#
# Cheap by construction: it reads the shared metadata through a throwaway clone
# and never touches an agent's repo.
#
# Usage: sampler.sh <swarm-root> [interval-seconds]

set -uo pipefail

ROOT="${1:?swarm root required}"
INTERVAL="${2:-300}"
OUT="$ROOT/trajectory.tsv"
IMAGE="${SWARM_IMAGE:-jjj-swarm-agent:0.5.1}"

[ -s "$OUT" ] || printf 'ts\telapsed_min\tscore\tproblems_open\tsolutions_submitted\tsolutions_approved\tcritiques_open\tjjj_calls\tjjj_failed\n' > "$OUT"

started=$(date +%s)

while true; do
    [ -e "$ROOT/STOP" ] && break
    podman ps --format '{{.Names}}' 2>/dev/null | grep -q '^swarm-pod' || break

    stats=$(podman run --rm -u swarm -v "$ROOT:/swarm:rw" --entrypoint /bin/bash "$IMAGE" -c '
        git clone -q /swarm/remote.git /tmp/s 2>/dev/null && cd /tmp/s || exit 1
        jj git init --colocate >/dev/null 2>&1
        export JJJ_USER=sampler
        jjj.real fetch >/dev/null 2>&1
        po=$(jjj.real problem list --status open --json 2>/dev/null | python3 -c "import json,sys;print(len(json.load(sys.stdin)))" 2>/dev/null || echo 0)
        ss=$(jjj.real solution list --status submitted --json 2>/dev/null | python3 -c "import json,sys;print(len(json.load(sys.stdin)))" 2>/dev/null || echo 0)
        sa=$(jjj.real solution list --status approved --json 2>/dev/null | python3 -c "import json,sys;print(len(json.load(sys.stdin)))" 2>/dev/null || echo 0)
        co=$(jjj.real critique list --status open --json 2>/dev/null | python3 -c "import json,sys;print(len(json.load(sys.stdin)))" 2>/dev/null || echo 0)
        printf "%s\t%s\t%s\t%s\n" "$po" "$ss" "$sa" "$co"
    ' 2>/dev/null | tail -1)

    log="$ROOT/jjj-invocations.jsonl"
    calls=$(wc -l < "$log" 2>/dev/null | tr -d ' ' || echo 0)
    failed=$(grep -c '"ok": *false' "$log" 2>/dev/null || echo 0)
    score=$(cut -f1 "$ROOT/last_score" 2>/dev/null || echo "")

    printf '%s\t%s\t%s\t%s\t%s\t%s\n' \
        "$(date +%H:%M:%S)" \
        "$(( ($(date +%s) - started) / 60 ))" \
        "${score:-?}" \
        "${stats:-?	?	?	?}" \
        "${calls:-0}" \
        "${failed:-0}" >> "$OUT"

    sleep "$INTERVAL"
done
