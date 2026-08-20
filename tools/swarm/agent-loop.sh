#!/usr/bin/env bash
#
# One agent in the swarm.
#
# Each iteration is a **fresh `claude -p` session with no memory of the last
# one**. That is deliberate: agents are stateless workers and every piece of
# coordination state — what exists, who holds what, what was objected to — lives
# in jjj. It makes the trial a clean test of the actual claim. If jjj is not a
# sufficient substrate, this swarm cannot function at all, and we find out.
#
# Usage: agent-loop.sh <pod-dir> <pod-name> <agent-name>
#
# Honours, checked every iteration so a runaway is always recoverable:
#   SWARM_STOP        path to a kill-switch file; its existence ends the loop
#   SWARM_DEADLINE    epoch seconds after which the loop ends
#   SWARM_MAX_ITERS   hard cap on iterations (0 = unlimited)
#   SWARM_MODEL       model to run agents on
#   SWARM_LOG         jjj invocation log (read by the shim)

set -uo pipefail

POD_DIR="${1:?pod directory required}"
POD="${2:?pod name required}"
AGENT="${3:?agent name required}"

SWARM_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
STOP="${SWARM_STOP:-$POD_DIR/../STOP}"
DEADLINE="${SWARM_DEADLINE:-0}"
MAX_ITERS="${SWARM_MAX_ITERS:-0}"
MODEL="${SWARM_MODEL:-sonnet}"

export JJJ_USER="$POD/$AGENT"     # namespaced id, per design decision 9
export JJJ_POD="$POD"             # own push bookmark jjj/{pod}
export SWARM_AGENT="$AGENT"

AGENT_LOG="$POD_DIR/../logs/$POD-$AGENT.log"
mkdir -p "$(dirname "$AGENT_LOG")"

log() { printf '[%s] %s\n' "$(date +%H:%M:%S)" "$*" >> "$AGENT_LOG"; }

log "start pod=$POD agent=$AGENT model=$MODEL dir=$POD_DIR"

read -r -d '' PROMPT <<'PROMPT_EOF' || true
You are one agent in a swarm working on a shared codebase. Several other agents
are working in this same directory at the same time. You have no memory of your
previous turns — all shared state lives in jjj, so read it rather than assume it.

Read skills/jjj/SKILL.md if it is available; otherwise `jjj --help`. Your identity
is already exported as JJJ_USER; confirm it with `jjj whoami` before writing.

Do ONE unit of work this turn, then stop. Pick whichever applies, in order:

1. If a solution of yours has an open critique against it, address it: fix the
   code, then `jjj critique address <id>`.
2. If another agent has a submitted solution with no critique from you, review it.
   Run ./score.py to check whether it actually works. If it is wrong or
   incomplete, raise a critique with the concrete failing case as evidence
   (`jjj critique new <solution-id> "..." --severity high`). If it is correct,
   say so with `jjj solution lgtm <id>`. A critique must cite evidence — a
   failing input, not an opinion.
3. Otherwise take new work: `jjj next --claim --json`. Verify you actually hold
   it (`--claim` is advisory, not a lock — another agent may have taken it), then
   implement the operation in opkit/ops/<name>.py, register it in
   opkit/registry.py, and check with ./score.py. When it passes, create and
   submit a solution.

Rules:
- Never use `--force` to bypass a critique. If approval is blocked, address the
  critique instead.
- Use ids from `--json`, never fuzzy titles.
- Other agents are editing these files right now. Re-read a file immediately
  before editing it, and keep edits small and additive.
- If `jjj` reports a conflict, resolve it and continue — do not abandon the turn.
- `./score.py -v` lists failures. The score is a count of passing cases; it is
  the only measure that matters. Nothing here is timed.

Report in one line what you did.
PROMPT_EOF

iter=0
while true; do
    if [ -e "$STOP" ]; then
        log "stopping: kill switch $STOP present"
        break
    fi
    if [ "$DEADLINE" -gt 0 ] && [ "$(date +%s)" -ge "$DEADLINE" ]; then
        log "stopping: deadline reached"
        break
    fi
    if [ "$MAX_ITERS" -gt 0 ] && [ "$iter" -ge "$MAX_ITERS" ]; then
        log "stopping: iteration cap $MAX_ITERS reached"
        break
    fi

    iter=$((iter + 1))
    started=$(date +%s)
    log "iter $iter begin"

    # --dangerously-skip-permissions is required for unattended operation: any
    # prompt would hang the loop forever. Containment is the pod directory plus
    # the fact that this runs against a scratch workbench, never a real repo.
    out=$(cd "$POD_DIR" && timeout 600 claude -p "$PROMPT" \
            --model "$MODEL" \
            --add-dir "$POD_DIR" \
            --dangerously-skip-permissions 2>&1)
    rc=$?
    elapsed=$(( $(date +%s) - started ))

    if [ $rc -ne 0 ]; then
        log "iter $iter FAILED rc=$rc after ${elapsed}s: $(echo "$out" | tail -3 | tr '\n' ' ')"
    else
        log "iter $iter ok ${elapsed}s: $(echo "$out" | tail -2 | tr '\n' ' ')"
    fi

    score=$(cd "$POD_DIR" && ./score.py 2>/dev/null || echo "? ?")
    log "iter $iter score=$score"

    # Jitter so agents do not lock-step into synchronised bursts, which would
    # make contention an artefact of the harness rather than of real behaviour.
    sleep $(( (RANDOM % 8) + 2 ))
done

log "exit after $iter iterations"
