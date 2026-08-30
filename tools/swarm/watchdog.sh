#!/usr/bin/env bash
#
# Stop the fleet once it has converged.
#
# A swarm has no idea when it is finished. In a one-hour trial the score reached
# its ceiling at minute 35 and the agents then made 4,100 further jjj calls and
# raised 36 more critiques for no gain — over half the run spent working on
# finished work. Nothing in jjj can detect that; it is the harness's job.
#
# "Converged" means all three of:
#   * the score has not moved for `--patience` samples,
#   * no problem is still open,
#   * no solution is still awaiting review.
#
# The second and third matter: a stalled score alone might mean the fleet is
# stuck on something hard, which is worth letting run. Only when there is also
# nothing left to do is it genuinely done.
#
# There is a second reason to stop, unrelated to convergence: an agent has raised
# an escalation and nobody cleared it. An escalation means "only a person can
# unblock this", so every turn after it is spent failing. A grace period first,
# because the person may be seconds away from acting — but not the whole
# remaining deadline, which is what happened when a dead credential burned 6.8
# hours of a run with nothing able to say so.
#
# Usage: watchdog.sh <swarm-root> [--interval SECONDS] [--patience N]

set -uo pipefail

ROOT="${1:?swarm root required}"; shift || true
INTERVAL="${SWARM_WATCHDOG_INTERVAL:-120}"
# How many consecutive unchanged polls count as "finished".
#
# Five polls is ten minutes, which suits a run measured in hours and is far too
# eager for one measured in days: a long target plateaus for an hour at a time
# while agents work through a hard tier, and stopping there would end a
# 24-hour trial before lunch. Raise it with SWARM_WATCHDOG_PATIENCE for long
# runs — at the default interval, 90 is three hours of no movement.
PATIENCE="${SWARM_WATCHDOG_PATIENCE:-5}"

# How long an escalation may stand before the fleet is stopped. Long enough for
# someone watching to act, short enough that an unattended run does not spend
# its budget on turns that cannot succeed.
ESCALATION_GRACE="${SWARM_ESCALATION_GRACE:-900}"

while [ $# -gt 0 ]; do
    case "$1" in
        --interval) INTERVAL="$2"; shift 2 ;;
        --patience) PATIENCE="$2"; shift 2 ;;
        *) echo "watchdog: unknown option $1" >&2; exit 1 ;;
    esac
done

JJJ="${JJJ_BIN:-$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)/target/release/jjj}"
log() { printf '[%s] watchdog: %s\n' "$(date +%H:%M:%S)" "$*"; }

log "watching $ROOT (every ${INTERVAL}s, patience $PATIENCE)"

last_score=""
stable=0

while [ -n "$(podman ps -q --filter 'name=swarm-' 2>/dev/null)" ]; do
    sleep "$INTERVAL"

    work="$(mktemp -d)"
    if ! git clone -q "$ROOT/remote.git" "$work/repo" 2>/dev/null; then
        chmod -R u+w "$work" 2>/dev/null; rm -rf "$work"; continue
    fi

    scorer="./score.py"; [ -x "$work/repo/score.sh" ] && scorer="./score.sh"
    score="$(cd "$work/repo" && $scorer 2>/dev/null | tail -1 | awk '{print $1}')"

    # Ask the metadata what is left, not just the score: a flat score with work
    # outstanding means stuck, which deserves more time, not a shutdown.
    open_problems=0
    awaiting_review=0
    escalations=0
    oldest_escalation=0
    if (cd "$work/repo" && jj git init --colocate >/dev/null 2>&1 \
        && jj config set --repo user.name watchdog >/dev/null 2>&1 \
        && jj config set --repo user.email watchdog@invalid >/dev/null 2>&1 \
        && "$JJJ" fetch >/dev/null 2>&1); then
        # `grep -c` prints its count AND exits non-zero when the count is zero,
        # so `|| echo 0` appends a *second* line: the variable becomes "0\n0"
        # and every later `[ "$x" -eq 0 ]` dies with "integer expected". The
        # watchdog then never converged — a toy run finished its work at minute
        # 15 and the fleet burned the remaining 45 minutes of its deadline.
        open_problems=$(cd "$work/repo" && "$JJJ" problem list --status open --json 2>/dev/null \
            | grep -c '"id"' | head -1)
        awaiting_review=$(cd "$work/repo" && "$JJJ" solution list --status submitted --json 2>/dev/null \
            | grep -c '"id"' | head -1)
        # Escalations come through jjj like everything else, so the watchdog sees
        # what any agent or human would see rather than a private side channel.
        read -r escalations oldest_escalation <<<"$(cd "$work/repo" \
            && "$JJJ" escalate --json 2>/dev/null \
            | python3 -c "
import json, sys, datetime
try:
    rows = json.load(sys.stdin)
except Exception:
    rows = []
now = datetime.datetime.now(datetime.timezone.utc)
oldest = 0
for r in rows:
    try:
        t = datetime.datetime.fromisoformat(r['raised_at'].replace('Z', '+00:00'))
        oldest = max(oldest, int((now - t).total_seconds()))
    except Exception:
        pass
print(len(rows), oldest)
" 2>/dev/null || echo "0 0")"
        escalations="${escalations:-0}"; oldest_escalation="${oldest_escalation:-0}"
    fi
    chmod -R u+w "$work" 2>/dev/null
    rm -rf "$work"

    if [ "$score" = "$last_score" ]; then
        stable=$((stable + 1))
    else
        stable=0
        last_score="$score"
    fi

    log "score=$score stable=$stable/$PATIENCE open=$open_problems awaiting_review=$awaiting_review escalations=$escalations"

    if [ "${escalations:-0}" -gt 0 ]; then
        log "!! $escalations open escalation(s); oldest ${oldest_escalation}s — a person is needed"
        log "   see: jjj escalate    clear with: jjj escalate --clear <id>"
        if [ "${oldest_escalation:-0}" -ge "$ESCALATION_GRACE" ]; then
            log "escalation unanswered for ${oldest_escalation}s (grace ${ESCALATION_GRACE}s)"
            log "stopping the fleet rather than spending the rest of the run blocked"
            touch "$ROOT/STOP"
            exit 0
        fi
    fi

    if [ "$stable" -ge "$PATIENCE" ] && [ "${open_problems:-1}" -eq 0 ] \
       && [ "${awaiting_review:-1}" -eq 0 ]; then
        log "converged: score steady at $score with nothing open and nothing awaiting review"
        log "stopping the fleet"
        touch "$ROOT/STOP"
        exit 0
    fi
done

log "no containers left; exiting"
