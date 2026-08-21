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
#   ./swarm.sh init  [--target toy|jjj] [--pods N] [--agents N] [--problems N] [--critics N]
#   ./swarm.sh start [--hours H] [--max-iters N] [--model M] [--stop-when-done]
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
        *) die "unknown target '$target' (expected toy or jjj)" ;;
    esac

    # The skill is half of what is under test, so it ships inside the repo the
    # agents clone rather than being mounted in.
    mkdir -p "$SEED/skills/jjj"
    cp "$REPO_ROOT/skills/jjj/SKILL.md" "$SEED/skills/jjj/SKILL.md"

    info "creating bare remote"
    git init -q --bare "$REMOTE"
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
            local agent name
            agent="agent-$(printf '%02d' "$a")"
            name="swarm-$pod-$agent"
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
                ${use_key:+$([ "$use_key" = 1 ] && echo "-e ANTHROPIC_API_KEY=$ANTHROPIC_API_KEY")} \
                -v "$SWARM_ROOT:/swarm:rw" \
                "$IMAGE" >/dev/null || die "failed to start $name"
            echo "  started $name ($role)"
        done
    done

    if [ "$stop_when_done" = 1 ]; then
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
    ooms=$(podman ps -a --filter "name=swarm-" --format "{{.Names}} {{.Status}}" 2>/dev/null \
        | grep -c "Exited (137)" || true)
    if [ "${ooms:-0}" -gt 0 ]; then
        echo "  !! $ooms container(s) were OOM-killed — raise SWARM_AGENT_MEMORY"
    fi
}

cmd_status() {
    [ -f "$SWARM_ROOT/config" ] || die "not initialised"
    # shellcheck disable=SC1091
    . "$SWARM_ROOT/config"

    echo "containers:"
    podman ps -a --filter "name=swarm-" \
        --format "  {{.Names}}  {{.Status}}" 2>/dev/null | sort
    [ -e "$STOP_FILE" ] && echo "  (kill switch SET)"
    report_ooms

    if [ -d "$SEED" ]; then
        local scorer="./score.py"; [ -x "$SEED/score.sh" ] && scorer="./score.sh"
        echo "seed score: $(cd "$SEED" && $scorer 2>/dev/null | tail -1)"
    fi

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
        for c in $(podman ps -a --filter "name=swarm-" --format "{{.Names}}" | sort); do
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
    for c in $(podman ps -a --filter "name=swarm-" --format "{{.Names}}"); do
        podman stop -t 10 "$c" >/dev/null 2>&1 && echo "  stopped $c"
    done
}

cmd_clean() {
    cmd_stop
    for c in $(podman ps -a --filter "name=swarm-" --format "{{.Names}}"); do
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
