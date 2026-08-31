#!/usr/bin/env bash
#
# Swarm supervisor — one podman container per agent.
#
# Each agent gets its own container and its own working copy, so nothing is
# shared between agents except a bare git remote. Every piece of coordination
# has to travel through jjj: what work exists, who holds it, what was objected
# to, and the code itself. If jjj is not a sufficient substrate the swarm cannot
# function, which is exactly the claim under test.
#
# Agents are grouped into *pods* that share a JJJ_POD, so several agents push
# the same `jjj/{pod}` bookmark. That keeps ref contention (Break #5) in the
# experiment; a bookmark per agent would have quietly designed it away.
#
#   ./swarm.sh build                                  build the agent image
#   ./swarm.sh init  [--target toy|sql|sqlperf|gophics|gophicswasm|jjj] [--pods N] [--agents N] [--problems N] [--critics N]
#   ./swarm.sh start [--hours H] [--max-iters N] [--model M] [--stop-when-done]
#       SWARM_STRATEGIES=1 gives each builder a different approach (measure,
#       structure, algorithm, correctness, simplify) instead of one shared
#       prompt — the variable behind "diverse perspectives beat parallel
#       effort", which is untested. Off by default, so the control is the
#       homogeneous run.
#   ./swarm.sh status
#   ./swarm.sh logs [agent]
#   ./swarm.sh stop
#   ./swarm.sh analyze
#
# Credentials: works with a Claude subscription. The refresher writes
# $SWARM_ROOT/credentials.json and each agent copies it in per turn — never
# bind-mounted, because podman binds a file by inode and the refresher's atomic
# rename would make the mount dangle. Exactly one process refreshes
# the token — token-refresher.sh on the host, using the CLI's own refresh path —
# and containers mount the exported credential READ-ONLY. Several containers
# refreshing one credential would race to rotate it and can break the host
# login, so they never write. Set ANTHROPIC_API_KEY instead to use an API key.

set -uo pipefail

SWARM_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SWARM_DIR/../.." && pwd)"
SWARM_ROOT="${SWARM_ROOT:-$HOME/.jjj-swarm}"

# Container names are namespaced by the workbench they belong to, so two runs
# can coexist: a long research trial and a short regression run of the toy
# target, say. Without this, `swarm-pod-1-agent-01` collides and the second run
# either fails to start or — worse — `stop`/`clean` from one reaps the other's
# containers mid-flight. Default keeps the historical names.
# The strategy roster, in assignment order. Names must match `strategy_brief`
# in the container entrypoint.
STRATEGY_NAMES=(measure structure algorithm correctness simplify)
STRATEGIES="${SWARM_STRATEGIES:-0}"

SWARM_NS="${SWARM_NS:-$(basename "$SWARM_ROOT" | sed 's/^\.//; s/^jjj-swarm$/swarm/; s/^jjj-//')}"
# Exported, because the sampler and watchdog run as separate processes and both
# have to watch *this* workbench. Without it they fell back to deriving a
# namespace from the root's basename, which is not the namespace the containers
# were actually named with whenever the two were set independently — the sampler
# then saw no containers and exited within the minute, and the watchdog's
# hardcoded `swarm-` filter matched either nothing or another run's containers.
export SWARM_NS
CPREFIX="${SWARM_NS}-"
JJJ_BIN="${JJJ_BIN:-$REPO_ROOT/target/release/jjj}"
IMAGE="${SWARM_IMAGE:-jjj-swarm-agent:0.5.1}"

REMOTE="$SWARM_ROOT/remote.git"
SEED="$SWARM_ROOT/seed"
STOP_FILE="$SWARM_ROOT/STOP"
CREDS="$SWARM_ROOT/credentials.json"
LOG="$SWARM_ROOT/jjj-invocations.jsonl"

die() { echo "swarm: $*" >&2; exit 1; }
info() { echo "  $*"; }

# --- build ------------------------------------------------------------------

cmd_build() {
    # Build context is the repo root: the image compiles jjj from this working
    # tree so a trial tests the code in front of us, not the last release. The
    # first shakedown ran against 0.5.1 and every pod push failed on a bug that
    # was already fixed locally.
    cp "$SWARM_DIR/jjj-shim" "$SWARM_DIR/container/jjj-shim"
    podman build -t "$IMAGE" -f "$SWARM_DIR/container/Containerfile" "$REPO_ROOT" \
        || die "image build failed"
    echo "Built $IMAGE from $(git -C "$REPO_ROOT" rev-parse --short HEAD)"
}

# --- init -------------------------------------------------------------------

cmd_init() {
    local pods=2 agents=3 problems=8 critics=1 target="toy"
    while [ $# -gt 0 ]; do
        case "$1" in
            --pods) pods="$2"; shift 2 ;;
            --agents) agents="$2"; shift 2 ;;
            --problems) problems="$2"; shift 2 ;;
            --critics) critics="$2"; shift 2 ;;
            --target) target="$2"; shift 2 ;;
            *) die "unknown option $1" ;;
        esac
    done

    [ -x "$JJJ_BIN" ] || die "jjj not found at $JJJ_BIN (cargo build --release)"
    podman image exists "$IMAGE" || die "image $IMAGE missing; run: $0 build"

    local total=$((pods * agents))
    echo "Building swarm in $SWARM_ROOT"
    rm -rf "$SWARM_ROOT"
    mkdir -p "$SWARM_ROOT/logs"
    : > "$LOG"
    chmod 666 "$LOG"

    if [ "$problems" -ge "$total" ]; then
        info "note: $problems problems for $total agents — seeding fewer problems"
        info "      than agents produces more claim contention, which is the point"
    fi

    case "$target" in
        toy)
            info "seeding the toy workbench ($problems operations)"
            python3 "$SWARM_DIR/toy/seed.py" "$SEED" --problems "$problems" \
                --jjj "$JJJ_BIN" --force \
                >"$SWARM_ROOT/logs/seed.log" 2>&1 \
                || { cat "$SWARM_ROOT/logs/seed.log"; die "seeding failed"; }
            ;;
        jjj)
            # A clone of the repository, never the repository: a nine-agent
            # fleet editing the tree we ship from is more blast radius than an
            # experiment deserves. The clone has no origin, so nothing an agent
            # does can reach it, and whatever survives is merge-gated by hand.
            info "cloning jjj as the workbench (self-improvement target)"
            "$SWARM_DIR/targets/jjj/seed.sh" "$SEED" "$REPO_ROOT" "$JJJ_BIN" \
                >"$SWARM_ROOT/logs/seed.log" 2>&1 \
                || { cat "$SWARM_ROOT/logs/seed.log"; die "seeding failed"; }
            grep -E '^(baseline|problems)' "$SWARM_ROOT/logs/seed.log" | sed 's/^/  /'
            ;;
        sql)
            # A fresh workbench holding a nearly-empty engine and an oracle it
            # cannot reach. Sized for a long run: the score has no ceiling to
            # bump into, because truth comes from SQLite over a corpus that is
            # regenerated every time rather than from a fixed checklist.
            info "seeding the SQL-engine workbench (differential test vs SQLite)"
            "$SWARM_DIR/targets/sql/seed.sh" "$SEED" "$REPO_ROOT" "$JJJ_BIN" \
                >"$SWARM_ROOT/logs/seed.log" 2>&1 \
                || { cat "$SWARM_ROOT/logs/seed.log"; die "seeding failed"; }
            grep -E '^(baseline|problems)' "$SWARM_ROOT/logs/seed.log" | sed 's/^/  /'
            ;;
        sqlperf)
            # Correctness is the starting point here, not the goal: the engine
            # already works. Latency has no ceiling to saturate against, which
            # is what the correctness target failed to provide.
            info "seeding the SQL latency workbench (budgets, oracle = SQLite)"
            "$SWARM_DIR/targets/sqlperf/seed.sh" "$SEED" "$REPO_ROOT" "$JJJ_BIN" \
                >"$SWARM_ROOT/logs/seed.log" 2>&1 \
                || { cat "$SWARM_ROOT/logs/seed.log"; die "seeding failed"; }
            grep -E '^(baseline|problems)' "$SWARM_ROOT/logs/seed.log" | sed 's/^/  /'
            ;;
        gophics)
            # A clone of a real project with no origin. The source is read and
            # never written; whatever survives is merge-gated by hand.
            info "cloning gophics as the workbench (frame-cost target)"
            "$SWARM_DIR/targets/gophics/seed.sh" "$SEED" "$REPO_ROOT" "$JJJ_BIN" \
                >"$SWARM_ROOT/logs/seed.log" 2>&1 \
                || { cat "$SWARM_ROOT/logs/seed.log"; die "seeding failed"; }
            grep -E '^(baseline|problems)' "$SWARM_ROOT/logs/seed.log" | sed 's/^/  /'
            ;;
        gophicswasm)
            # Same clone-with-no-origin discipline as the gophics target: the
            # source is read and never written.
            info "cloning gophics as the workbench (wasm-size target)"
            "$SWARM_DIR/targets/gophicswasm/seed.sh" "$SEED" "$REPO_ROOT" "$JJJ_BIN" \
                >"$SWARM_ROOT/logs/seed.log" 2>&1 \
                || { cat "$SWARM_ROOT/logs/seed.log"; die "seeding failed"; }
            grep -E '^(baseline|problems)' "$SWARM_ROOT/logs/seed.log" | sed 's/^/  /'
            ;;
        synth)
            # One seeded problem, on purpose: decomposition is the thing under
            # test, not a preliminary to it.
            info "seeding the synthetic decomposition target"
            "$SWARM_DIR/targets/synth/seed.sh" "$SEED" "$REPO_ROOT" "$JJJ_BIN" \
                >"$SWARM_ROOT/logs/seed.log" 2>&1 \
                || { cat "$SWARM_ROOT/logs/seed.log"; die "seeding failed"; }
            grep -E '^(baseline|problems)' "$SWARM_ROOT/logs/seed.log" | sed 's/^/  /'
            ;;
        *) die "unknown target '$target' (expected toy, sql, sqlperf, gophics, gophicswasm, synth or jjj)" ;;
    esac

    # The skill is half of what is under test, so it ships inside the repo the
    # agents clone rather than being mounted in.
    mkdir -p "$SEED/skills/jjj"
    cp "$REPO_ROOT/skills/jjj/SKILL.md" "$SEED/skills/jjj/SKILL.md"

    info "creating bare remote"
    git init -q --bare "$REMOTE"
    # Every agent pushes to this one repo from its own container, and rootless
    # podman maps each container's uid to a subuid that is not the host user.
    # Without this, objects are written creator-only and a concurrent push hits
    # "unable to open loose object ...: Permission denied" — 69 of 478 pushes in
    # one run, each retried five times. The remote is a throwaway test fixture,
    # so permissive is the right trade.
    git --git-dir="$REMOTE" config core.sharedRepository 0666
    ( cd "$SEED"
      git add -A && git commit -q -m "seed: skill" 2>/dev/null
      git remote add origin "$REMOTE"
      git push -q origin HEAD:refs/heads/main
      "$JJJ_BIN" push >/dev/null 2>&1 ) || die "seed push failed"

    # Bare repos reject pushes to the checked-out branch; agents all push main.
    ( cd "$REMOTE" && git symbolic-ref HEAD refs/heads/main )

    cat > "$SWARM_ROOT/config" <<EOF
target=$target
pods=$pods
agents=$agents
problems=$problems
critics=$critics
EOF

    echo
    echo "Ready: $pods pods x $agents agents = $total containers"
    local scorer="./score.py"; [ -x "$SEED/score.sh" ] && scorer="./score.sh"
    echo "  baseline: $(cd "$SEED" && $scorer 2>/dev/null | tail -1)"
    echo "  start with: $0 start --hours 1"
}

# --- start ------------------------------------------------------------------

cmd_start() {
    local hours=0 max_iters=0 model="sonnet" stop_when_done=0
    while [ $# -gt 0 ]; do
        case "$1" in
            --hours) hours="$2"; shift 2 ;;
            --max-iters) max_iters="$2"; shift 2 ;;
            --model) model="$2"; shift 2 ;;
            --stop-when-done) stop_when_done=1; shift ;;
            *) die "unknown option $1" ;;
        esac
    done

    [ -f "$SWARM_ROOT/config" ] || die "not initialised; run: $0 init"
    # shellcheck disable=SC1091
    . "$SWARM_ROOT/config"

    rm -f "$STOP_FILE"

    # Credential: an API key if there is one, otherwise the subscription token
    # kept fresh by exactly one host-side refresher.
    local use_key=0
    if [ -n "${ANTHROPIC_API_KEY:-}" ]; then
        use_key=1
        info "using ANTHROPIC_API_KEY"
    else
        security find-generic-password -s "Claude Code-credentials" -w >/dev/null 2>&1 \
            || die "no ANTHROPIC_API_KEY and no Claude Code Keychain entry.
  Log in on the host with \`claude\` first, or export ANTHROPIC_API_KEY."

        pkill -f "token-refresher.sh $CREDS" 2>/dev/null
        nohup "$SWARM_DIR/token-refresher.sh" "$CREDS" \
            >"$SWARM_ROOT/logs/refresher.log" 2>&1 &
        echo $! > "$SWARM_ROOT/refresher.pid"

        # The refresher must produce a credential before any agent starts, or
        # every container fails its first turn on a missing file.
        for _ in $(seq 1 20); do
            [ -s "$CREDS" ] && break
            sleep 1
        done
        [ -s "$CREDS" ] || die "refresher produced no credential; see $SWARM_ROOT/logs/refresher.log"
        info "subscription token exported; refresher pid $(cat "$SWARM_ROOT/refresher.pid")"
    fi

    local deadline=0
    if [ "$hours" != "0" ]; then
        deadline=$(python3 -c "import time,sys;print(int(time.time()+float(sys.argv[1])*3600))" "$hours")
    fi

    echo "Starting $((pods * agents)) agent containers (model=$model)"
    [ "$deadline" != "0" ] && echo "  deadline: $(date -r "$deadline" '+%H:%M:%S')"
    [ "$max_iters" != "0" ] && echo "  cap: $max_iters iterations per agent"

    for p in $(seq 1 "$pods"); do
        local pod="pod-$p" role="builder"
        # The last `critics` pods review; the rest build. A single shared
        # priority list does not distribute — six identically-prompted agents
        # produced 193 reviewing calls against 13 producing ones, because
        # reviewing is always available and cheaper than building.
        if [ "$p" -gt "$((pods - ${critics:-0}))" ]; then role="critic"; fi
        for a in $(seq 1 "$agents"); do
            local agent name strategy=""
            agent="agent-$(printf '%02d' "$a")"
            # One strategy per builder, round-robin and deterministic, so a run
            # is reproducible and two runs differ only in this. Critics keep
            # their own brief; the question is whether *builders* searching from
            # different priors get stuck less often than six copies of one.
            # Exactly one integrator, and only when the merge gate is on.
            #
            # Integration used to be something every agent did at the top of its
            # turn: whoever noticed an approved solution first merged it. That
            # is not a decision, it is a race, and it shows — seven rival
            # solutions to one problem were merged and withdrawn in whatever
            # order agents happened to wake up.
            #
            # Choosing among solutions that have survived criticism is a
            # judgement someone should make deliberately, looking at all the
            # candidates and the objections against each. So one agent does it,
            # and nobody else merges.
            if [ "${SWARM_MERGE_GATE:-0}" = "1" ] && [ "$p" -eq "$pods" ] \
               && [ "$a" -eq "$agents" ]; then
                role="integrator"
            fi
            if [ "$STRATEGIES" = 1 ] && [ "$role" = "builder" ]; then
                local idx=$(( ( (p - 1) * agents + a - 1 ) % ${#STRATEGY_NAMES[@]} ))
                strategy="${STRATEGY_NAMES[$idx]}"
            fi
            name="$CPREFIX$pod-$agent"
            podman rm -f "$name" >/dev/null 2>&1

            podman run -d --name "$name" \
                --memory "${SWARM_AGENT_MEMORY:-3g}" --cpus "${SWARM_AGENT_CPUS:-2}" \
                -e "JJJ_USER=$pod/$agent" \
                -e "JJJ_POD=$pod" \
                -e "SWARM_AGENT=$agent" \
                -e "SWARM_REMOTE=/swarm/remote.git" \
                -e "SWARM_LOG=/swarm/jjj-invocations.jsonl" \
                -e "SWARM_STOP=/swarm/STOP" \
                -e "SWARM_DEADLINE=$deadline" \
                -e "SWARM_MAX_ITERS=$max_iters" \
                -e "SWARM_MODEL=$model" \
                -e "SWARM_ROLE=$role" \
                -e "SWARM_STRATEGY=$strategy" \
                -e "SWARM_MERGE_GATE=${SWARM_MERGE_GATE:-0}" \
                -e "SWARM_TURN_TIMEOUT=${SWARM_TURN_TIMEOUT:-1500}" \
                ${use_key:+$([ "$use_key" = 1 ] && echo "-e ANTHROPIC_API_KEY=$ANTHROPIC_API_KEY")} \
                -v "$SWARM_ROOT:/swarm:rw" \
                "$IMAGE" >/dev/null || die "failed to start $name"
            echo "  started $name ($role${strategy:+, $strategy})"
        done
    done

    nohup "$SWARM_DIR/sampler.sh" "$SWARM_ROOT" "${SWARM_SAMPLE_INTERVAL:-300}" \
        >"$SWARM_ROOT/logs/sampler.log" 2>&1 &
    echo "$!" > "$SWARM_ROOT/sampler.pid"
    echo "  sampling every $(( ${SWARM_SAMPLE_INTERVAL:-300} / 60 ))m to $SWARM_ROOT/trajectory.tsv"

    if [ "$stop_when_done" = 1 ]; then
        # A watchdog that cannot fire before the deadline is decoration. With
        # patience 60 at a 120s poll it needs two hours of no movement, which on
        # a two-hour run is never: one trial reached its ceiling at minute 40 and
        # then sat there for eighty more, watchdog counting stable=27/60 as the
        # deadline arrived.
        local wd_patience="${SWARM_WATCHDOG_PATIENCE:-5}"
        local wd_interval="${SWARM_WATCHDOG_INTERVAL:-120}"
        if [ "$hours" != "0" ]; then
            local budget=$(( $(printf '%.0f' "$(echo "$hours * 3600" | bc -l 2>/dev/null || echo 0)") ))
            local needed=$(( wd_patience * wd_interval ))
            if [ "$budget" -gt 0 ] && [ "$needed" -ge "$budget" ]; then
                local capped=$(( budget / wd_interval / 2 ))
                [ "$capped" -lt 3 ] && capped=3
                info "watchdog patience $wd_patience would need $((needed/60))m of stillness on a $((budget/60))m run; using $capped"
                wd_patience="$capped"
            fi
        fi
        SWARM_WATCHDOG_PATIENCE="$wd_patience" \
        SWARM_WATCHDOG_INTERVAL="$wd_interval" \
        nohup "$SWARM_DIR/watchdog.sh" "$SWARM_ROOT" >"$SWARM_ROOT/logs/watchdog.log" 2>&1 &
        echo "$!" > "$SWARM_ROOT/watchdog.pid"
        echo "  watchdog running: will stop the fleet once the work converges"
    fi

    echo "  watch: $0 status    stop: $0 stop"
}

# --- status / logs / stop / analyze -----------------------------------------

# Exit 137 is SIGKILL, which for these containers means the OOM reaper. It is
# indistinguishable from a crash in `podman ps` output, so call it out.
report_ooms() {
    local ooms
    ooms=$(podman ps -a --filter "name=^${CPREFIX}pod-" --format "{{.Names}} {{.Status}}" 2>/dev/null \
        | grep -c "Exited (137)" || true)
    if [ "${ooms:-0}" -gt 0 ]; then
        echo "  !! $ooms container(s) were OOM-killed — raise SWARM_AGENT_MEMORY"
    fi
}


# Assert the things a run has to be doing, rather than describing what it looks
# like.
#
# Descriptive status is how a fleet spent 6.8 hours of a 24-hour run failing 90%
# of its turns while every container stayed up, the sampler kept writing rows,
# and "0 failures" was true of the field being read. Each check below is an
# invariant that was silently violated once, and each names the cause rather
# than the symptom.
cmd_health() {
    local running warn=0
    running="$(podman ps --filter "name=^${CPREFIX}pod-" --format '{{.Names}}' 2>/dev/null)"
    [ -z "$running" ] && return 0

    echo "health:"

    # 0. Has anyone asked for a person? Above every other check, because it is
    #    the only one the fleet cannot resolve by itself — and because the
    #    outage this whole function exists for was precisely a fleet with no way
    #    to say "I am blocked on something only you can fix".
    if [ -d "$SEED/.jj" ] && [ -x "$JJJ_BIN" ]; then
        ( cd "$SEED" && "$JJJ_BIN" fetch >/dev/null 2>&1 ) || true
        local esc_json esc_n
        esc_json="$(cd "$SEED" && "$JJJ_BIN" escalate --json 2>/dev/null || echo '[]')"
        esc_n="$(printf '%s' "$esc_json" | grep -c '"reason"' || true)"
        if [ "${esc_n:-0}" -gt 0 ]; then
            echo "  escalation: !! $esc_n open — a person is needed"; warn=1
            printf '%s' "$esc_json" | python3 -c "
import json, sys
try:
    for r in json.load(sys.stdin):
        print('                {} [{}] {}'.format(r['id'][:8], r['by'], r['reason']))
except Exception:
    pass" 2>/dev/null
            echo "              clear with: (cd $SEED && jjj escalate --clear <id>)"
        fi
    fi

    # 1. Are turns succeeding? An expired host login fails every turn in about a
    #    second, which quadratic backoff never escapes.
    # A recent window, not the whole run. Cumulative counts stay red forever
    # after any outage, and a permanently red light is one nobody reads.
    local ok fail total window="${SWARM_HEALTH_WINDOW:-10}"
    ok=0; fail=0
    for c in $running; do
        local recent
        recent="$(podman logs "$c" 2>&1 | grep -E "iter [0-9]+ (ok|FAILED)" | tail -"$window")"
        ok=$((ok + $(printf '%s\n' "$recent" | grep -c " ok" || true)))
        fail=$((fail + $(printf '%s\n' "$recent" | grep -c " FAILED" || true)))
    done
    total=$((ok + fail))
    if [ "$total" -eq 0 ]; then
        echo "  turns:      none finished yet"
    elif [ $((ok * 2)) -lt "$total" ]; then
        echo "  turns:      !! $ok ok / $fail failed in the last $window per agent"
        local why
        why="$(for c in $running; do podman logs "$c" 2>&1; done \
               | grep -E "iter [0-9]+ FAILED" | tail -20 \
               | grep -oE "OAuth session expired|rc=124|rc=137" | sort | uniq -c | head -2 | tr '\n' ' ')"
        [ -n "$why" ] && echo "              $why(137 is the OOM killer, 124 a turn timeout)"
        warn=1
    else
        echo "  turns:      $ok ok / $fail failed (last $window per agent)"
    fi

    # 2. Is shared work actually landing? The merge path broke twice today and
    #    both times the fleet looked entirely healthy.
    if [ -d "$REMOTE" ]; then
        local last age
        last="$(git --git-dir="$REMOTE" log -1 --format=%ct main 2>/dev/null || echo 0)"
        age=$(( ( $(date +%s) - ${last:-0} ) / 60 ))
        if [ "${last:-0}" -eq 0 ]; then
            echo "  main:       !! no commits — nothing has ever merged"; warn=1
        elif [ "$age" -gt 45 ]; then
            echo "  main:       !! last advanced ${age}m ago — approved work may not be merging"; warn=1
        else
            echo "  main:       advanced ${age}m ago"
        fi
    fi

    # 3. Do the agents agree? They score the same shared tree, so a wide spread
    #    means they are NOT sharing — which is how a run of six private trees
    #    passed for a healthy fleet, scores climbing apart from 18 to 55.
    # The best of each agent's last few *completed* turns, not the score its
    # current turn opened with.
    #
    # A turn can legitimately open at zero — a merge left the tree unbuildable —
    # and the agent then fixes it within that same turn. Reading the opening
    # score reported three of six agents at 0 while they were in fact at 73, so
    # the check cried wolf on exactly the transient it should ignore. Sustained
    # divergence, which is the thing worth alarming on, survives taking a
    # maximum over several turns; a transient does not.
    local scores lo hi
    scores="$(for c in $running; do
                  podman logs "$c" 2>&1 | grep -oE "iter [0-9]+ end score=[0-9]+" \
                      | tail -4 | grep -oE "[0-9]+$" | sort -n | tail -1
              done | sort -n)"
    lo="$(echo "$scores" | head -1)"; hi="$(echo "$scores" | tail -1)"
    if [ -n "$lo" ] && [ -n "$hi" ] && [ "$hi" -gt 0 ]; then
        # A wide spread alone is not a fault under the merge gate: an agent
        # holding an improvement that has not been reviewed yet *should* score
        # above one working from main, and a gophics run showed 27 against 68
        # for exactly that reason — three agents with real unmerged work.
        #
        # What distinguishes "waiting for review" from "not sharing" is whether
        # anything is landing. A wide spread with main advancing is the system
        # working; a wide spread with main stale is the merge path broken, which
        # is the case worth waking someone for.
        if [ $(( hi - lo )) -gt 25 ] && [ "${age:-0}" -gt 30 ]; then
            echo "  agreement:  !! scores span $lo-$hi and main is stale — work is not landing"; warn=1
        elif [ $(( hi - lo )) -gt 25 ]; then
            echo "  agreement:  scores $lo-$hi (spread is unmerged work awaiting review)"
        else
            echo "  agreement:  scores $lo-$hi"
        fi
    fi

    # 4. Is the credential still good? Its expiry is knowable in advance, and
    #    only an interactive login on the host renews a dead session.
    if [ -f "$SWARM_ROOT/AUTH_DEAD" ]; then
        echo "  auth:       !! the host OAuth session has expired — run \`claude\` and log in"; warn=1
    elif [ -f "$SWARM_ROOT/credentials.json" ]; then
        local left
        left="$(python3 -c "
import json,sys,time
try:
    d=json.load(open('$SWARM_ROOT/credentials.json'))['claudeAiOauth']
    print(int((d['expiresAt']/1000 - time.time())/60))
except Exception: print(-1)" 2>/dev/null)"
        if [ "${left:--1}" -lt 0 ]; then
            echo "  auth:       !! credential unreadable or expired"; warn=1
        elif [ "$left" -lt 60 ]; then
            echo "  auth:       !! expires in ${left}m"; warn=1
        else
            echo "  auth:       valid ${left}m"
        fi
    fi

    [ "$warn" = 1 ] && echo "  -> something is wrong; the run is probably not producing work"
    return 0
}

cmd_status() {
    [ -f "$SWARM_ROOT/config" ] || die "not initialised"
    # shellcheck disable=SC1091
    . "$SWARM_ROOT/config"

    echo "containers:"
    podman ps -a --filter "name=^${CPREFIX}pod-" \
        --format "  {{.Names}}  {{.Status}}" 2>/dev/null | sort
    [ -e "$STOP_FILE" ] && echo "  (kill switch SET)"
    report_ooms

    if [ -d "$SEED" ]; then
        local scorer="./score.py"; [ -x "$SEED/score.sh" ] && scorer="./score.sh"
        echo "seed score: $(cd "$SEED" && $scorer 2>/dev/null | tail -1)"
    fi

    cmd_health

    if [ -s "$LOG" ]; then
        echo "jjj calls: $(wc -l < "$LOG" | tr -d ' ')"
        python3 - "$LOG" <<'PY'
import json, sys, collections
cmds, fails, actors = collections.Counter(), 0, set()
for line in open(sys.argv[1]):
    try: r = json.loads(line)
    except Exception: continue
    cmds[r["cmd"]] += 1; fails += r["exit"] != 0; actors.add(r["actor"])
print(f"  {len(actors)} actors")
for cmd, n in cmds.most_common(6):
    print(f"  {n:5d}  {cmd}")
print(f"  {fails} failed")
PY
    fi
}

cmd_logs() {
    if [ $# -gt 0 ]; then
        podman logs -f "swarm-$1" 2>&1
    else
        for c in $(podman ps -a --filter "name=^${CPREFIX}pod-" --format "{{.Names}}" | sort); do
            echo "=== $c ==="
            podman logs --tail 8 "$c" 2>&1
        done
    fi
}

cmd_stop() {
    touch "$STOP_FILE"
    if [ -f "$SWARM_ROOT/watchdog.pid" ]; then
        kill "$(cat "$SWARM_ROOT/watchdog.pid")" 2>/dev/null
        rm -f "$SWARM_ROOT/watchdog.pid"
    fi
    if [ -f "$SWARM_ROOT/refresher.pid" ]; then
        kill "$(cat "$SWARM_ROOT/refresher.pid")" 2>/dev/null && echo "  stopped token refresher"
        rm -f "$SWARM_ROOT/refresher.pid"
    fi
    echo "Kill switch set; agents exit after their current turn."
    echo "Force-stopping containers..."
    for c in $(podman ps -a --filter "name=^${CPREFIX}pod-" --format "{{.Names}}"); do
        podman stop -t 10 "$c" >/dev/null 2>&1 && echo "  stopped $c"
    done
}

cmd_clean() {
    cmd_stop
    for c in $(podman ps -a --filter "name=^${CPREFIX}pod-" --format "{{.Names}}"); do
        podman rm -f "$c" >/dev/null 2>&1
    done
    echo "Removed containers. Data kept in $SWARM_ROOT"
}

case "${1:-}" in
    build) shift; cmd_build "$@" ;;
    init) shift; cmd_init "$@" ;;
    start) shift; cmd_start "$@" ;;
    status) shift; cmd_status "$@" ;;
    logs) shift; cmd_logs "$@" ;;
    stop) shift; cmd_stop "$@" ;;
    clean) shift; cmd_clean "$@" ;;
    analyze) shift; python3 "$SWARM_DIR/analyze.py" "$SWARM_ROOT" ;;
    *) sed -n '2,28p' "$0" | sed 's/^# \{0,1\}//'; exit 1 ;;
esac
