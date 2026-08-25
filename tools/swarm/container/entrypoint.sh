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

# A per-agent branch name: JJJ_USER is "pod-1/agent-02", and a slash would make
# refs/heads/swarm-pod-1/agent-02 a directory where a sibling ref is a file.
SWARM_BRANCH="${JJJ_USER//\//-}"

# Does the tree still build?
#
# This was `cargo build --release` at both call sites, which is correct for the
# jjj target and vacuous everywhere else: on a Python workbench it fails for
# want of a Cargo.toml, so the pre-push gate refused every push and the
# integration step refused every approved solution. A fleet ran for an hour with
# ten approvals, zero merges and zero shared branches — six agents each
# optimising a private tree, which is the opposite of the thing being tested.
#
# A target can define its own check in `verify.sh`; otherwise this guesses from
# what is in the workbench.
verify_build() {
    if [ -x ./verify.sh ]; then
        ./verify.sh >/dev/null 2>&1
    elif [ -f Cargo.toml ]; then
        cargo build --release --quiet 2>/dev/null
    elif ls ./*.py >/dev/null 2>&1 || [ -d sqlengine ]; then
        python3 -m compileall -q . >/dev/null 2>&1
    else
        return 0
    fi
}

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
#
# Fatal rather than a warning. An agent that cannot fetch sees an empty
# repository, finds nothing to do, and idles politely until the deadline — a
# whole fleet once burned twenty minutes that way over a too-old git in the
# image, and the only symptom was a warning nobody was watching. Dying here puts
# the cause in `swarm.sh status` instead.
fetch_err=""
for attempt in 1 2 3; do
    if fetch_err="$(jjj fetch 2>&1)"; then
        fetch_err=""
        break
    fi
    log "initial jjj fetch failed (attempt $attempt/3)"
    sleep 5
done

if [ -n "$fetch_err" ]; then
    log "FATAL: cannot fetch jjj metadata; this agent would idle with nothing to do"
    printf '%s\n' "$fetch_err" | tail -5 | while IFS= read -r line; do log "  $line"; done
    exit 1
fi

# `--status all`, not the default. The default lists only *open* problems, so a
# fleet that had solved its entire backlog looked identical to one whose fetch
# was silently broken, and six agents killed themselves on arrival at the moment
# they succeeded. What this guard is actually for is proving the metadata
# arrived; whether any of it is still open is the loop's business, not the
# startup check's.
visible="$(jjj problem list --status all 2>/dev/null | tail -n +3 | grep -c .)"
open_now="$(jjj problem list 2>/dev/null | tail -n +3 | grep -c .)"
if [ "${visible:-0}" -eq 0 ]; then
    log "FATAL: fetch succeeded but no metadata is visible — the workbench looks empty"
    exit 1
fi

log "clone ready; $visible problems visible ($open_now open)"

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

Read SWARM.md for the target, and skills/jjj/SKILL.md for how to use jjj.

Do ONE unit of work this turn, then stop. In priority order:

1. If one of YOUR solutions has an open critique, address it, then
   `jjj critique address <id>`. `jjj solution list --mine --json` shows yours.
2. Otherwise TAKE NEW WORK: `jjj next --claim --json`. Do the work, verify with
   ./score.sh, then create and submit a solution describing your approach.
3. Only if `jjj next` offers nothing at all, review another agent's submitted
   solution.

Your job is to make ./score.sh go up. Reviewing is not your job unless there is
genuinely nothing left to build.

REBASE BEFORE YOU SUBMIT. Your turn takes tens of minutes and main moves under
you while it runs: `git fetch origin && git merge origin/main`, re-run
./score.sh, and only then submit. Half of all solutions in one run were
withdrawn, most of them as "rebased onto current main" or "superseded by what
landed while I was working" — that is your own effort thrown away, and a
reviewer's on top of it.

Measure before and after — ./score.sh is the arbiter, not your judgement of
whether a change ought to help. Say the numbers in your solution, and put your
reasoning in the body: `jjj solution new "Title" --body "..."` (or `--body -`
to read stdin for anything long). A title is a label, not an argument.

YOUR CODE ONLY REACHES THE OTHERS IF IT IS APPROVED. Working, submitting and
being reviewed is the whole loop:
  jjj solution new "..." --body "what you changed and why" --problem <id>
  jjj solution attach <id>     # link your jj change
  jjj solution submit <id>     # publishes the diff so a critic can read it
Attaching without submitting leaves your work invisible. If a critic raises a
critique, fix it and `jjj critique address <id>` — do not argue past it.
PROMPT_EOF

read -r -d '' CRITIC_PROMPT <<'PROMPT_EOF' || true
You are a CRITIC in a swarm. Other agents work on this same project right now,
each in their own container, sharing nothing but a git remote — all coordination
goes through jjj. You have no memory of previous turns; read the state.

Read SWARM.md for the target, and skills/jjj/SKILL.md for how to use jjj.

Do ONE unit of work this turn, then stop. In priority order:

1. Review a submitted solution you have not yet reviewed. Actually run
   ./score.sh and exercise the change — do not review by reading alone. If it is
   wrong, or does not improve the score, raise a critique citing the concrete
   number or failing case as evidence
   (`jjj critique new <solution-id> "..." --severity high`). If it holds up,
   `jjj solution lgtm <id> --rationale "ran ./score.sh: 26 -> 41"`.
   You do not need to be assigned a review to sign off; take submitted work off
   the queue. You cannot sign off your own solution.

   SIGNING OFF IS NOT APPROVING, and nothing merges until a solution is
   Approved. Sign off and approve in one step:
       jjj solution lgtm <id> --approve --rationale "ran ./score.sh: 29 -> 48"
   It refuses while any critique is still open, which is what you want. An lgtm
   you do not follow through leaves the work stranded on its branch — which is
   where a whole trial's output ended up: eight reviewed solutions, none landed.
2. If every submitted solution has your review, and an approved solution's
   problem is still open, solve the problem.
3. Only if there is nothing to review at all, take new work with
   `jjj next --claim --json` and do it.

A critique that cannot name a number or a failing input is not a critique.
Prefer one well-evidenced objection over three vague ones. Put the evidence in
the body — `jjj critique new <sid> "Short title" --body -` reads stdin, so a
long argument survives intact. A title is a label, not the argument.

A submitted solution has its diff published as a branch, so review the real
code: `git fetch origin review-s-<solution-id> && git diff main...FETCH_HEAD`.

Your sign-off is what lets code reach the shared branch — nothing merges
without it. That cuts both ways: waving through a change that lowers the score,
or that makes a number look good by removing a correctness check, costs
everyone. ./score.sh already refuses to score a tree that fails the sync
correctness tests; if it prints 0, the change is broken, not fast.

Useful: `jjj solution list --status submitted --json` is your queue;
`jjj events --user <agent>` shows what another agent has been doing.
PROMPT_EOF

# Different priors, for the hypothesis that a swarm's advantage is not parallel
# effort but parallel *perspective* — that several agents attacking a problem
# from genuinely different directions is what stops the group getting stuck
# where one agent would.
#
# Untested until now: every builder in every trial so far had an identical
# prompt, so the runs measured six copies of one search, not six searches.
# Empty by default, so a homogeneous run remains the control.
strategy_brief() {
    case "$1" in
        measure) cat <<'EOF'

YOUR APPROACH: measure before you change anything.

Profile first and let the numbers choose the target. Do not optimise what you
believe is slow — find what the measurement says is slow, and say what you
measured in your solution body. If a change does not move the number, say that
too and withdraw it; a refuted conjecture reported honestly is worth more to the
others than a plausible one left standing.
EOF
;;
        structure) cat <<'EOF'

YOUR APPROACH: attack the representation, not the code around it.

Ask what the data *is* before asking what the loop does — layout, indexes, what
gets materialised and when, what is copied that could be borrowed. The largest
wins here usually come from storing something differently rather than from
doing the same work faster.
EOF
;;
        algorithm) cat <<'EOF'

YOUR APPROACH: attack the asymptotics, not the constants.

Look for the operation whose cost grows with the wrong thing — a scan where a
lookup would do, a sort where a bounded heap would do, a nested loop where a
hash would do, work repeated per row that could be done once. Shaving a constant
off an O(n^2) path is worth less than making it O(n).
EOF
;;
        correctness) cat <<'EOF'

YOUR APPROACH: hunt for wrong answers.

A fast engine that is wrong scores nothing, and a subtle wrong answer is worth
more to find than a slow one — NULLs, empty inputs, ties, type coercion,
boundaries. Look for cases the others' optimisations would break, and critique
them with a reproduction rather than a suspicion.
EOF
;;
        simplify) cat <<'EOF'

YOUR APPROACH: make it smaller.

The fastest code is the code that does not run. Look for work that is redundant,
paths that can be specialised, layers that can be collapsed, and things being
computed that nobody reads. Prefer deleting to adding, and be suspicious of any
change that makes the code longer for a small gain.
EOF
;;
    esac
}

case "${SWARM_ROLE:-builder}" in
    critic) PROMPT="$CRITIC_PROMPT" ;;
    *)      PROMPT="$BUILDER_PROMPT" ;;
esac
PROMPT="$PROMPT$(strategy_brief "${SWARM_STRATEGY:-}")

$IDENTITY_RULE

Rules:
- Never \`--force\` past a critique.
- Use ids from \`--json\`, never fuzzy titles.
- Only \`solution approve\`, \`solution withdraw\` and \`problem dissolve\` take
  \`--rationale\` / \`--no-rationale\`. Other commands reject those flags.
- Other agents edit the same files concurrently; keep edits small and additive.
- If any file contains \`<<<<<<<\` conflict markers, resolving them is the most
  valuable thing you can do this turn: one such file breaks everyone at once.
- ./score.sh prints \`<score> <ceiling>\`. Higher is better. Nothing is timed, so
  do not optimise for wall-clock.
- **./score.sh costs about two minutes.** Run it once to get your baseline and
  once to check your change — not in a loop. Your turn is time-boxed, and a turn
  that runs out mid-edit publishes nothing.
- **Finish the loop before you run out of time.** A change that is not submitted
  helps nobody: it stays on your branch and no reviewer can approve it. If time
  is short, submit what you have with an honest body rather than polishing.

Report in one line what you did, including the score before and after."

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
    UNMERGED=""
    if ! git merge --no-edit origin/HEAD >/dev/null 2>&1; then
        CONFLICTED="$(git diff --name-only --diff-filter=U 2>/dev/null | tr '\n' ' ')"
        log "iter $iter pull conflicted in: $CONFLICTED (handing to the agent)"
    fi
    jjj fetch >/dev/null 2>&1

    # Integrate approved work — the ONLY path by which code reaches the shared
    # branch when SWARM_MERGE_GATE is on.
    #
    # A previous trial let every agent commit straight to main each turn, so
    # code landed whether or not anyone had reviewed it: the critics were real,
    # their reasoning was good, and nothing they concluded could stop a merge.
    # That measures six agents editing a shared branch, not an economy of
    # critique. `jjj solution submit` now publishes a review-s-<id> branch, so a
    # reviewer can read the actual diff — and approval can mean something.
    if [ "${SWARM_MERGE_GATE:-0}" = "1" ]; then
        for sid in $(jjj solution list --status approved --json 2>/dev/null \
                     | python3 -c 'import json,sys
try: print(" ".join(s["id"] for s in json.load(sys.stdin)))
except Exception: pass' 2>/dev/null); do
            b="review-s-$sid"
            git fetch -q origin "$b" 2>/dev/null || continue
            # Already in? Nothing to do.
            git merge-base --is-ancestor FETCH_HEAD origin/main 2>/dev/null && continue
            if git merge -q --no-edit FETCH_HEAD 2>/dev/null \
               && verify_build; then
                if git push -q origin HEAD:refs/heads/main 2>/dev/null; then
                    log "iter $iter integrated approved solution $sid"
                fi
            else
                # Six agents editing one module means approved work routinely
                # conflicts with whatever landed while it was in review.
                # Resolving that is a semantic job, not a shell one — but
                # logging it and walking away means approved work never lands
                # at all, which is how a run ends with a pile of approvals and
                # an untouched main.
                #
                # So it becomes an agent's work item, the same way a conflicted
                # pull already does.
                git merge --abort 2>/dev/null
                # Bounded, per agent. Handing a stuck solution to whoever is
                # next is right; handing the *same* one to all six agents every
                # turn forever is a livelock — one unmergeable solution was
                # retried 120 times, and each attempt left conflict markers that
                # took the retrying agent's own score to zero.
                tries=$(( $(cat "/tmp/unmerged-$sid" 2>/dev/null || echo 0) + 1 ))
                echo "$tries" > "/tmp/unmerged-$sid"
                if [ "$tries" -le 2 ]; then
                    UNMERGED="$UNMERGED $sid"
                    log "iter $iter approved solution $sid does not merge cleanly; handing to the agent (try $tries)"
                else
                    log "iter $iter approved solution $sid still will not merge after $tries tries; leaving it"
                fi
            fi
        done
    fi

    # A tree carrying conflict markers scores zero no matter how good the
    # engine is, and the agent then spends its turn reading a meaningless
    # number. If the previous turn left markers behind, say so plainly and let
    # the guard below count it toward a reset.
    LEFTOVER="$(git diff --name-only --diff-filter=U 2>/dev/null | tr '\n' ' ')"
    if [ -z "$LEFTOVER" ]; then
        LEFTOVER="$(git diff --name-only HEAD 2>/dev/null | while IFS= read -r f; do
            [ -f "$f" ] && grep -qE '^(<{7} |={7}$|>{7} )' -- "$f" 2>/dev/null && printf '%s ' "$f"
        done)"
    fi
    if [ -n "$LEFTOVER" ]; then
        log "iter $iter tree still carries conflict markers in: $LEFTOVER"
        CONFLICTED="$LEFTOVER"
    fi

    SCORER="./score.py"; [ -x ./score.sh ] && SCORER="./score.sh"
    # Keep the scorer's stderr: it carries both the measured ratio and the
    # reason a gate failed. Discarding it once turned a fleet-wide score of zero
    # into a mystery that took a container autopsy to explain.
    before=$($SCORER 2>/tmp/score.err | tail -1 || echo "? ?")
    if [ "${before%% *}" = "0" ]; then
        log "iter $iter scorer gate FAILED: $(tail -2 /tmp/score.err | tr '\n' ' ')"
    fi

    # If there is no work and nothing to review, wait rather than paying for a
    # model call to be told there is nothing to do. A finished backlog otherwise
    # costs exactly as much as a busy one: after the score maxed out at minute
    # 35 of a one-hour run, nine agents made 4,100 further jjj calls for no gain.
    if [ "$(jjj next --json 2>/dev/null)" = "null" ] \
       && [ -z "$(jjj solution list --status submitted --json 2>/dev/null | tr -d '[] \n')" ]; then
        idle=$((idle + 1))
        # An empty backlog is not the same as a finished job. A four-hour run
        # cleared its seven seeded problems in the first hour and then idled for
        # two and a half — 58% of the run producing nothing, with the score
        # nowhere near its ceiling. Six agents empty a hand-written backlog far
        # faster than anyone can write one, so after a couple of idle turns the
        # fleet is asked to find its own work rather than wait for more.
        if [ "$idle" -ge 2 ] && [ "${SWARM_SELF_SEED:-1}" = "1" ]; then
            log "iter $iter backlog empty (idle $idle); asking for new problems"
            turn_prompt="$PROMPT

THE BACKLOG IS EMPTY AND THE SCORE IS NOT AT ITS CEILING. Nobody is going to
hand you more work, so find some.

Profile or measure first, then write down what you found as new problems:

    jjj problem new \"...\" --body \"what the measurement says, and why it is
    worth doing\" --priority high

Ground every one in a number you took this turn. A problem that says
'harfbuzz is 2MB of the 12MB binary, here is the symbol breakdown' is work
somebody can pick up; 'improve performance' is not, and will waste six agents'
time rather than one's.

Two or three good problems is a better turn than one mediocre solution."
            out=$(timeout "${SWARM_TURN_TIMEOUT:-1500}" claude -p "$turn_prompt" \
                    --dangerously-skip-permissions --model "$MODEL" 2>&1)
            log "iter $iter seeded work: $(printf '%s' "$out" | tail -1 | cut -c1-160)"
            jjj push >/dev/null 2>&1
            idle=0
            continue
        fi
        log "iter $iter nothing to do (idle $idle); waiting"
        sleep $(( idle < 6 ? idle * 30 : 180 ))
        continue
    fi
    idle=0

    log "iter $iter begin (score $before)"

    # A briefing, because every turn starts with no memory and re-derives the
    # world: one run made 2,322 orientation calls against 138 that produced
    # anything, a ratio of 17 to 1. The harness already knows this state and can
    # hand it over for four cheap queries instead of each agent paying for
    # twenty.
    brief="$(jjj next --json 2>/dev/null | python3 -c '
import json, sys
try:
    d = json.load(sys.stdin) or {}
except Exception:
    d = {}
if d:
    print(f"  next up: [{d.get(\"category\",\"?\")}] {d.get(\"title\",\"?\")}")
    print(f"           id {d.get(\"entity_id\",\"?\")} — {d.get(\"summary\",\"\")}")
' 2>/dev/null)
$(jjj solution list --status submitted --json 2>/dev/null | python3 -c '
import json, sys
try:
    d = json.load(sys.stdin) or []
except Exception:
    d = []
if d:
    print(f"  awaiting review ({len(d)}):")
    for x in d[:5]:
        print(f"    {x[\"id\"][:8]}  {x[\"title\"][:66]}")
' 2>/dev/null)
$(jjj critique list --status open --json 2>/dev/null | python3 -c '
import json, sys
try:
    d = json.load(sys.stdin) or []
except Exception:
    d = []
if d:
    print(f"  open critiques ({len(d)}):")
    for x in d[:5]:
        print(f"    on {x.get(\"solution_id\",\"?\")[:8]}  {x[\"title\"][:62]}")
' 2>/dev/null)"

    turn_prompt="$PROMPT"
    if [ -n "$(printf '%s' "$brief" | tr -d '[:space:]')" ]; then
        turn_prompt="$turn_prompt

STATE AS OF THIS TURN (already fetched; you do not need to re-query it):
$brief"
    fi
    if [ -n "$UNMERGED" ]; then
        turn_prompt="$turn_prompt

APPROVED WORK IS STUCK. These solutions passed review but no longer merge into
main, because main moved while they were being reviewed:$UNMERGED

Landing one of them is the most valuable thing you can do this turn — it is
finished, reviewed work that currently helps nobody. For a solution <id>:

    git fetch origin review-s-<id>
    git merge FETCH_HEAD          # resolve the conflicts in the working tree
    ./verify.sh && ./score.sh     # confirm it still builds and still helps
    git commit && git push origin HEAD:refs/heads/main

Keep both sides' work. Two agents optimising different query classes in the
same module is the normal case here, and the union is almost always what was
intended."
    fi
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
    # 600s was too short once the fitness function had a real correctness gate.
    # `./score.sh` costs ~110s under six-way contention — a release build, the
    # library tests, three integration tests and the benchmark — and an agent
    # naturally runs it before and after its change. Every turn of one trial
    # died at rc=124 with the work half-done: agents kept editing and never
    # reached `jjj solution submit`, so the fleet produced six divergent local
    # trees and not one reviewable solution.
    out=$(timeout "${SWARM_TURN_TIMEOUT:-1500}" claude -p "$turn_prompt" \
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

        # An expired OAuth session is not a transient failure: every turn will
        # fail in about a second until a person logs in on the host. Spinning
        # through it burns the run — 400 turns of one run died this way, at a
        # 90% failure rate, while the containers looked healthy. Wait long
        # enough that recovery costs one turn rather than hundreds.
        case "$out" in
            *"OAuth session expired"*|*"Failed to authenticate"*)
                backoff=300
                log "iter $iter AUTH: the host session has expired — run \`claude\` there and log in"
                ;;
        esac
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
        # Scan whatever git is actually about to commit, rather than a fixed
        # file glob. This once read `--include='*.py'`, carried over from the
        # Python toy target, so on the Rust target it matched nothing and the
        # guard silently passed everything: a fleet ran two hours and ended with
        # nested `<<<<<<< HEAD` markers committed into src/commands/fetch.rs, a
        # shared branch that would not compile, and every score at zero. A guard
        # that only protects one language is worse than none, because it reads
        # as protection.
        markers="$(git diff --name-only HEAD 2>/dev/null; git ls-files -o --exclude-standard 2>/dev/null)"
        markers="$(printf '%s\n' "$markers" | sort -u | while IFS= read -r f; do
            [ -n "$f" ] && [ -f "$f" ] || continue
            grep -qE '^(<{7} |={7}$|>{7} )' -- "$f" 2>/dev/null && printf '%s ' "$f"
        done)"
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
        elif ! verify_build; then
            # Do not publish a tree that does not compile. The marker guard
            # above catches conflict markers, but a half-finished refactor
            # compiles to nothing just as effectively — and everything here is
            # pushed straight to a branch every other agent builds on. One
            # fleet ended a two-hour run on a shared branch that would not
            # build, with every agent's score at zero, because nothing checked.
            stuck=$((stuck + 1))
            log "iter $iter not pushing code: the tree does not build (stuck $stuck)"
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
            if [ "${SWARM_MERGE_GATE:-0}" = "1" ]; then
                # Work stays on this agent's own branch. It reaches main only
                # by being submitted, reviewed and approved — see the
                # integration step above.
                git push -q -f origin "HEAD:refs/heads/swarm-$SWARM_BRANCH" 2>/dev/null \
                    && log "iter $iter pushed to swarm-$SWARM_BRANCH (awaiting review)"
                continue
            fi
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

    after=$($SCORER 2>/dev/null | tail -1 || echo "? ?")
    log "iter $iter end score=$after"
    # Publish the latest score for the host-side sampler. Best of whatever the
    # agents last measured — any one of them is representative, since they all
    # score the same shared tree.
    printf '%s\n' "${after%% *}" > /swarm/last_score 2>/dev/null || true

    # Jitter, so agents do not lock-step into synchronised bursts and make the
    # contention an artefact of the harness.
    sleep $(( (RANDOM % 8) + 2 ))
done

log "agent $JJJ_USER exiting after $iter iterations"
