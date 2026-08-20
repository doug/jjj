#!/usr/bin/env bash
#
# One containerised swarm agent.
#
# Clones the shared remote into the container's own filesystem, then loops:
# pull, do one unit of work via a fresh `claude -p` session, push. Nothing is
# shared with any other agent except the git remote — so every collision that
# happens is a genuine merge, not a filesystem race.
#
# Each iteration is a fresh session with no memory. All state lives in jjj.
#
# Required:
#   SWARM_REMOTE     path to the shared bare repo (mounted)
#   JJJ_USER         namespaced identity, e.g. pod-a/agent-01
#   JJJ_POD          pod name; several agents may share one, which is what puts
#                    the per-pod bookmark under contention
# Optional:
#   SWARM_STOP       kill-switch path (mounted); its existence ends the loop
#   SWARM_DEADLINE   epoch seconds after which to stop
#   SWARM_MAX_ITERS  hard iteration cap (0 = unlimited)
#   SWARM_MODEL      model for agents (default sonnet)
#   SWARM_LOG        shared JSONL the shim appends to (mounted)

set -uo pipefail

: "${SWARM_REMOTE:?SWARM_REMOTE is required}"
: "${JJJ_USER:?JJJ_USER is required}"
: "${JJJ_POD:?JJJ_POD is required}"

STOP="${SWARM_STOP:-/swarm/STOP}"
DEADLINE="${SWARM_DEADLINE:-0}"
MAX_ITERS="${SWARM_MAX_ITERS:-0}"
MODEL="${SWARM_MODEL:-sonnet}"
WORK=/work/repo

log() { printf '[%s] %s\n' "$(date -u +%H:%M:%S)" "$*"; }

log "agent $JJJ_USER starting (pod=$JJJ_POD model=$MODEL)"

# --- clone and identify -----------------------------------------------------

git clone -q "$SWARM_REMOTE" "$WORK" || { log "FATAL: clone failed"; exit 1; }
cd "$WORK"

git config user.name "$JJJ_USER"
git config user.email "${JJJ_USER//\//-}@swarm.invalid"
jj git init --colocate >/dev/null 2>&1
jj config set --repo user.name "$JJJ_USER" >/dev/null 2>&1
jj config set --repo user.email "${JJJ_USER//\//-}@swarm.invalid" >/dev/null 2>&1

# Pull the metadata bookmark so the agent can see the seeded problems.
jjj fetch >/dev/null 2>&1 || log "warning: initial jjj fetch failed"

log "clone ready; $(jjj problem list 2>/dev/null | tail -n +3 | wc -l | tr -d ' ') problems visible"

read -r -d '' PROMPT <<'PROMPT_EOF' || true
You are one agent in a swarm. Other agents are working on this same project
right now, each in their own container. You share nothing with them except a git
remote — all coordination goes through jjj. You have no memory of previous
turns, so read the current state rather than assuming it.

Read skills/jjj/SKILL.md first if present. Your identity is already exported as
JJJ_USER; confirm with `jjj whoami` before writing anything.

Do ONE unit of work this turn, then stop. In priority order:

1. If one of your solutions has an open critique, address it: fix the code, then
   `jjj critique address <id>`.
2. If another agent has a submitted solution you have not reviewed, review it.
   Run ./score.py to check whether it actually works. If it is wrong, raise a
   critique citing the concrete failing case as evidence
   (`jjj critique new <solution-id> "..." --severity high`). If it is correct,
   `jjj solution lgtm <id>`. Evidence, never opinion.
3. Otherwise take new work: `jjj next --claim --json`. Verify you actually hold
   it — `--claim` is advisory, not a lock — then implement the operation in
   opkit/ops/<name>.py, register it in opkit/registry.py, and check ./score.py.
   When it passes, create and submit a solution.

Rules:
- Never `--force` past a critique. If approval is blocked, address the critique.
- Use ids from `--json`, never fuzzy titles.
- Other agents are editing opkit/registry.py and tests/cases.py concurrently.
  Keep your edits small and additive so they merge cleanly.
- If jjj reports a conflict, resolve it and continue; do not abandon the turn.
- ./score.py -v lists failures. The score counts passing cases. Nothing is timed.

Report in one line what you did.
PROMPT_EOF

# --- loop -------------------------------------------------------------------

iter=0
while true; do
    if [ -e "$STOP" ]; then log "stopping: kill switch present"; break; fi
    if [ "$DEADLINE" -gt 0 ] && [ "$(date +%s)" -ge "$DEADLINE" ]; then
        log "stopping: deadline reached"; break
    fi
    if [ "$MAX_ITERS" -gt 0 ] && [ "$iter" -ge "$MAX_ITERS" ]; then
        log "stopping: iteration cap $MAX_ITERS reached"; break
    fi

    iter=$((iter + 1))

    # Pull before choosing work — design decision 8's sync cadence.
    git fetch -q origin 2>/dev/null && git merge -q --no-edit origin/HEAD 2>/dev/null
    jjj fetch >/dev/null 2>&1

    before=$(./score.py 2>/dev/null || echo "? ?")
    log "iter $iter begin (score $before)"

    started=$(date +%s)
    out=$(timeout 600 claude -p "$PROMPT" \
            --model "$MODEL" \
            --add-dir "$WORK" \
            --dangerously-skip-permissions 2>&1)
    rc=$?
    elapsed=$(( $(date +%s) - started ))

    if [ $rc -ne 0 ]; then
        log "iter $iter FAILED rc=$rc after ${elapsed}s: $(echo "$out" | tail -3 | tr '\n' ' ')"
    else
        log "iter $iter ok ${elapsed}s: $(echo "$out" | tail -2 | tr '\n' ' ')"
    fi

    # Push after producing an artefact — the other half of decision 8. Code goes
    # through git; metadata through jjj. Both must land for another agent to see
    # this work at all.
    if ! git diff --quiet HEAD 2>/dev/null || [ -n "$(git status --porcelain 2>/dev/null)" ]; then
        git add -A 2>/dev/null
        git commit -q -m "$JJJ_USER: iter $iter" 2>/dev/null
        for attempt in 1 2 3; do
            git fetch -q origin 2>/dev/null
            if ! git merge -q --no-edit origin/HEAD 2>/dev/null; then
                # Do NOT auto-pick a side. `checkout --ours` here silently threw
                # away another agent's registry entry, so a correct operation
                # scored zero — the exact lossy auto-resolve decision 10 warns
                # about. Abort and let an agent reconcile on its next turn,
                # which is what the prompt instructs.
                log "iter $iter merge conflict; aborting and deferring to next turn"
                git merge --abort 2>/dev/null
                break
            fi
            if git push -q origin HEAD:refs/heads/main 2>/dev/null; then
                log "iter $iter pushed code (attempt $attempt)"
                break
            fi
            sleep $((attempt * 2))
        done
    fi
    jjj push >/dev/null 2>&1 || log "iter $iter jjj push failed"

    after=$(./score.py 2>/dev/null || echo "? ?")
    log "iter $iter end score=$after"

    # Jitter, so agents do not lock-step into synchronised bursts and make the
    # contention an artefact of the harness.
    sleep $(( (RANDOM % 8) + 2 ))
done

log "agent $JJJ_USER exiting after $iter iterations"
