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
   `jjj critique address <id>`. `jjj solution list --mine --json` shows yours.
2. Otherwise TAKE NEW WORK. `jjj next --claim --json` gives you an unimplemented
   operation. Implement it in opkit/ops/<name>.py, register it in
   opkit/registry.py, verify with ./score.py, then create and submit a solution.
3. Only if `jjj next` offers nothing at all, review another agent's submitted
   solution and critique or `jjj solution lgtm` it.

Your job is to make ./score.py go up. Reviewing is not your job unless there is
genuinely nothing left to build.

Rules:
- Never `--force` past a critique.
- Only `solution approve`, `solution withdraw` and `problem dissolve` take
  `--rationale` / `--no-rationale`. Other commands reject those flags.
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
   `jjj solution lgtm <id> --rationale "ran ./score.py; all N cases pass"`.
   Evidence, never opinion — a sign-off should say what you actually ran. You do
   not need to be assigned a review to sign off; take submitted work off the
   queue. You cannot sign off your own solution.
2. If every submitted solution has your review, and an approved solution's
   problem is still open, solve the problem.
3. Only if there is nothing to review at all, take new work with
   `jjj next --claim --json` and implement it.

Useful: `jjj solution list --status submitted --json` is your queue;
`jjj solution list --mine` is what you own; `jjj events --user <agent>` shows
what another agent has been doing.

A critique that cannot name a failing input is not a critique. Prefer one
well-evidenced objection over three vague ones.

Rules:
- Never `--force` past a critique.
- Only `solution approve`, `solution withdraw` and `problem dissolve` take
  `--rationale` / `--no-rationale`. Other commands reject those flags.
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
stuck=0
failures=0
idle=0
while true; do
    if [ -e "$STOP" ]; then log "stopping: kill switch present"; break; fi
    if [ "$DEADLINE" -gt 0 ] && [ "$(date +%s)" -ge "$DEADLINE" ]; then
        log "stopping: deadline reached"; break
    fi
    if [ "$MAX_ITERS" -gt 0 ] && [ "$iter" -ge "$MAX_ITERS" ]; then
        log "stopping: iteration cap $MAX_ITERS reached"; break
    fi

    iter=$((iter + 1))

    # Refresh the credential from the shared directory.
    #
    # It is copied per turn rather than bind-mounted, because podman binds a
    # *file* by inode: the host-side refresher writes atomically via rename,
    # which replaces the inode, and the container's mount is then a dangling
    # reference — the file simply disappears inside the container. That killed a
    # three-hour run 45 minutes in, with every agent reporting "Not logged in"
    # while the host credential was perfectly valid. Mounting the *directory*
    # would also work; copying keeps each container's ~/.claude private, which
    # matters because the CLI writes other state there.
    if [ -f /swarm/credentials.json ]; then
        mkdir -p "$HOME/.claude"
        cp /swarm/credentials.json "$HOME/.claude/.credentials.json" 2>/dev/null
        chmod 600 "$HOME/.claude/.credentials.json" 2>/dev/null
    fi

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

    # If there is no work and nothing to review, wait rather than paying for a
    # model call to be told there is nothing to do. A finished backlog otherwise
    # costs exactly as much as a busy one: after the score maxed out at minute
    # 35 of a one-hour run, nine agents made 4,100 further jjj calls for no gain.
    if [ "$(jjj next --json 2>/dev/null)" = "null" ] \
       && [ -z "$(jjj solution list --status submitted --json 2>/dev/null | tr -d '[] \n')" ]; then
        idle=$((idle + 1))
        log "iter $iter nothing to do (idle $idle); waiting"
        sleep $(( idle < 6 ? idle * 30 : 180 ))
        continue
    fi
    idle=0

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
        failures=$((failures + 1))
        # Exponential backoff, capped. A turn that fails in a second will fail
        # again in a second; spinning on it burns the run and buries the real
        # signal in noise.
        backoff=$(( failures * failures * 5 )); [ "$backoff" -gt 300 ] && backoff=300
        log "iter $iter backing off ${backoff}s after $failures consecutive failures"
        sleep "$backoff"
    else
        failures=0
        log "iter $iter ok ${elapsed}s: $(echo "$out" | tail -2 | tr '\n' ' ')"
    fi

    # Push after producing an artefact — the other half of decision 8. Code goes
    # through git; metadata through jjj. Both must land for another agent to see
    # this work at all.
    if ! git diff --quiet HEAD 2>/dev/null || [ -n "$(git status --porcelain 2>/dev/null)" ]; then
        # Never publish an unresolved conflict: one committed marker breaks the
        # package import and takes every agent's score to zero. jjj already
        # refuses this for metadata by validating entity bodies before push; the
        # code path had no equivalent.
        markers="$(grep -rlE '^(<{7} |={7}$|>{7} )' --include='*.py' . 2>/dev/null | tr '\n' ' ')"
        if [ -n "$markers" ]; then
            stuck=$((stuck + 1))
            log "iter $iter not pushing code: markers in $markers (stuck $stuck)"

            # An agent that cannot resolve its tree is dead weight for the rest
            # of the run, so give it a bounded number of turns and then rejoin
            # the fleet. Local work is lost, which is worth saying out loud —
            # but an agent contributing nothing for three hours is worse.
            if [ "$stuck" -ge 3 ]; then
                log "iter $iter RESETTING to origin after $stuck stuck turns; local work discarded"
                git merge --abort 2>/dev/null
                git reset -q --hard origin/main 2>/dev/null || git reset -q --hard origin/HEAD 2>/dev/null
                git clean -qfd 2>/dev/null
                stuck=0
            fi
        else
            stuck=0
            git add -A 2>/dev/null
            git commit -q -m "$JJJ_USER: iter $iter" 2>/dev/null
            for attempt in 1 2 3; do
                git fetch -q origin 2>/dev/null
                if ! git merge -q --no-edit origin/HEAD 2>/dev/null; then
                    # Do not auto-pick a side; the next turn's pull hands the
                    # conflict to the agent, which resolves it properly.
                    log "iter $iter merge conflict on push; deferring to next turn"
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
    fi

    # Metadata pushes regardless of the code's state. Claims, critiques and
    # sign-offs are how the rest of the fleet coordinates, and withholding them
    # because a Python file has a marker in it makes one agent's local mess
    # everyone's problem — an earlier `continue` here did exactly that.
    jjj push >/dev/null 2>&1 || log "iter $iter jjj push failed"

    after=$(./score.py 2>/dev/null || echo "? ?")
    log "iter $iter end score=$after"

    # Jitter, so agents do not lock-step into synchronised bursts and make the
    # contention an artefact of the harness.
    sleep $(( (RANDOM % 8) + 2 ))
done

log "agent $JJJ_USER exiting after $iter iterations"
