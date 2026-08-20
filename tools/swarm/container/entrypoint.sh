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

log "agent $JJJ_USER starting (pod=$JJJ_POD role=${SWARM_ROLE:-builder} model=$MODEL)"

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

# The identity line is emphatic because agents got it wrong: two of six in an
# earlier trial *set* JJJ_USER rather than reading it, and their work landed
# under identities that belong to nobody.
IDENTITY_RULE='Your identity is already set in the environment. Do NOT export,
change, or invent JJJ_USER — run `jjj whoami` to read it and use exactly that.'

# Roles exist because a single shared priority list does not distribute. When
# every agent was told "review if anything is reviewable, otherwise take new
# work", six agents produced 193 reviewing calls against 13 producing ones and
# implemented two operations in 36 turns: reviewing is always available and
# cheaper, so the fleet starved itself of production.
read -r -d '' BUILDER_PROMPT <<'PROMPT_EOF' || true
You are a BUILDER in a swarm. Other agents work on this same project right now,
each in their own container, sharing nothing but a git remote — all coordination
goes through jjj. You have no memory of previous turns; read the state.

Read skills/jjj/SKILL.md if present.

Do ONE unit of work this turn, then stop. In priority order:

1. If one of YOUR solutions has an open critique, address it: fix the code, then
   `jjj critique address <id>`.
2. Otherwise TAKE NEW WORK. `jjj next --claim --json` gives you an unimplemented
   operation. Implement it in opkit/ops/<name>.py, register it in
   opkit/registry.py, verify with ./score.py, then create and submit a solution.
3. Only if `jjj next` offers nothing at all, review another agent's submitted
   solution and critique or `jjj solution lgtm` it.

Your job is to make ./score.py go up. Reviewing is not your job unless there is
genuinely nothing left to build.

Rules:
- Never `--force` past a critique.
- Use ids from `--json`, never fuzzy titles.
- Other agents edit opkit/registry.py concurrently; keep edits small and additive.
- ./score.py -v lists failures. The score counts passing cases; nothing is timed.
- If any file contains `<<<<<<<` conflict markers, resolving them is the most
  valuable thing you can do this turn: one such file breaks the import and takes
  every agent's score to zero.

Report in one line what you did.
PROMPT_EOF

read -r -d '' CRITIC_PROMPT <<'PROMPT_EOF' || true
You are a CRITIC in a swarm. Other agents work on this same project right now,
each in their own container, sharing nothing but a git remote — all coordination
goes through jjj. You have no memory of previous turns; read the state.

Read skills/jjj/SKILL.md if present.

Do ONE unit of work this turn, then stop. In priority order:

1. Review a submitted solution you have not yet reviewed. Actually run ./score.py
   and exercise the code — do not review by reading alone. If it is wrong or
   incomplete, raise a critique citing the concrete failing input as evidence
   (`jjj critique new <solution-id> "..." --severity high`). If it is correct,
   `jjj solution lgtm <id>`. Evidence, never opinion.
2. If every submitted solution has your review, and an approved solution's
   problem is still open, solve the problem.
3. Only if there is nothing to review at all, take new work with
   `jjj next --claim --json` and implement it.

A critique that cannot name a failing input is not a critique. Prefer one
well-evidenced objection over three vague ones.

Rules:
- Never `--force` past a critique.
- Use ids from `--json`, never fuzzy titles.
- ./score.py -v lists failures. The score counts passing cases; nothing is timed.

Report in one line what you did.
PROMPT_EOF

case "${SWARM_ROLE:-builder}" in
    critic) PROMPT="$CRITIC_PROMPT" ;;
    *)      PROMPT="$BUILDER_PROMPT" ;;
esac
PROMPT="$PROMPT

$IDENTITY_RULE"

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

    # Pull before choosing work — design decision 8's sync cadence. A failed
    # merge must be aborted: left alone it writes conflict markers into the tree,
    # and the commit below would then publish a file that does not parse. That
    # happened — a committed `<<<<<<< HEAD` in one operation broke the package
    # import and took the whole fleet's score to zero.
    # Pull before choosing work — design decision 8's sync cadence.
    #
    # A conflict here is left IN the tree on purpose. Three shell-side policies
    # were tried and all three lost work: `checkout --ours` discarded another
    # agent's registry entry; `merge --abort` dropped the incoming work; and
    # `checkout -- .` threw away the agent's own. Resolving a semantic merge is
    # not a job for bash — but every agent is a Claude session that can do it in
    # seconds. So the conflict becomes this turn's work item (decision 10:
    # the agent auto-resolves and re-pushes).
    git fetch -q origin 2>/dev/null
    CONFLICTED=""
    if ! git merge --no-edit origin/HEAD >/dev/null 2>&1; then
        CONFLICTED="$(git diff --name-only --diff-filter=U 2>/dev/null | tr '\n' ' ')"
        log "iter $iter pull conflicted in: $CONFLICTED (handing to the agent)"
    fi
    jjj fetch >/dev/null 2>&1

    before=$(./score.py 2>/dev/null || echo "? ?")
    log "iter $iter begin (score $before)"

    turn_prompt="$PROMPT"
    if [ -n "$CONFLICTED" ]; then
        turn_prompt="A merge left unresolved conflict markers in: $CONFLICTED

RESOLVE THEM FIRST, before anything else. Keep BOTH sides' work — these files are
additive (an operation registry and a case table), so the correct resolution is
almost always the union of both, not a choice between them. Then \`git add\` the
files. Do not commit; the harness commits for you.

Once resolved, continue with your normal task below.

$PROMPT"
    fi

    started=$(date +%s)
    out=$(timeout 600 claude -p "$turn_prompt" \
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
        # Never publish an unresolved conflict. jjj already refuses this for
        # metadata (it validates entity bodies before push); the code had no
        # such guard, and one committed marker is enough to break every agent's
        # score at once.
        # Never publish an unresolved conflict: one committed marker breaks the
        # package import and takes every agent's score to zero. If the agent did
        # not manage to resolve it, keep the work in this container and try again
        # next turn rather than discarding it.
        if grep -rlE '^(<{7} |={7}$|>{7} )' --include='*.py' . 2>/dev/null | grep -q .; then
            log "iter $iter not pushing: markers still present in $(grep -rlE '^(<{7} |={7}$|>{7} )' --include='*.py' . 2>/dev/null | tr '\n' ' ')"
            continue
        fi
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
