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
# and never touches an agent's repo. Cheaper still when nothing is happening —
# a container run per tick is the wrong price for a fleet that has stopped
# moving, so a tick whose ref fingerprint matches the last one reuses the
# previous counts and starts no container. The time series keeps its regular
# rows either way; only the cost of producing them goes away.
#
# Usage: sampler.sh <swarm-root> [interval-seconds]

set -uo pipefail

ROOT="${1:?swarm root required}"
INTERVAL="${2:-300}"
OUT="$ROOT/trajectory.tsv"
# Match swarm.sh's namespacing so a sampler watches only its own workbench.
NS="${SWARM_NS:-$(basename "$ROOT" | sed 's/^\.//; s/^jjj-swarm$/swarm/; s/^jjj-//')}"
IMAGE="${SWARM_IMAGE:-jjj-swarm-agent:0.5.1}"
# shellcheck disable=SC1091
. "$(dirname "${BASH_SOURCE[0]}")/lib-remote.sh"

# `agent_score` is the last score any agent measured in ITS OWN tree, which is
# not the shared branch's score: under the merge gate an agent's work sits on
# its own branch until approved, so this reads as progress before anything has
# actually landed. `solutions_approved` is the integration signal; to score the
# shared branch, check out main at that point and run score.sh there.
[ -s "$OUT" ] || printf 'ts\telapsed_min\tagent_score\tproblems_open\tsolutions_submitted\tsolutions_approved\tcritiques_open\tjjj_calls\tjjj_failed\n' > "$OUT"

started=$(date +%s)

# A single failed `podman ps` must not end the sampling run. Right after six
# containers are created podman's own state lock is contended, and one empty
# reply there was enough to kill a sampler seconds after it started — leaving
# the trial with no trajectory at all, which is the exact failure this script
# exists to prevent.
misses=0
last_fingerprint=""
stats=""

while true; do
    if [ -e "$ROOT/STOP" ]; then
        echo "sampler: STOP present; exiting" >&2
        break
    fi
    # Capture first, then match. `podman ps | grep -q` looks equivalent but is
    # not: grep -q exits on the first match, podman takes SIGPIPE, and under
    # `set -o pipefail` the pipeline then reports failure *because* it matched.
    # That intermittently killed the sampler seconds into a run.
    running="$(podman ps --format '{{.Names}}' 2>/dev/null)"
    if printf '%s\n' "$running" | grep -q "^${NS}-pod"; then
        misses=0
    else
        misses=$((misses + 1))
        if [ "$misses" -ge 3 ]; then
            echo "sampler: no agent containers for $misses checks; exiting" >&2
            break
        fi
        sleep 20
        continue
    fi

    # Every effect the fleet can have on shared state is a ref update in the
    # bare remote, so an unchanged fingerprint means the counts below are
    # unchanged too. Reuse them rather than starting a container to be told the
    # same four numbers.
    fingerprint="$(remote_fingerprint "$ROOT/remote.git")"
    if [ -n "$fingerprint" ] && [ "$fingerprint" = "$last_fingerprint" ] && [ -n "$stats" ]; then
        : # keep the previous $stats
    else
        last_fingerprint="$fingerprint"
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
    fi

    log="$ROOT/jjj-invocations.jsonl"
    calls=$(wc -l < "$log" 2>/dev/null | tr -d ' ')
    failed=$(grep -c '"ok": *false' "$log" 2>/dev/null | head -1)
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
