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
#   ./swarm.sh init  [--pods N] [--agents N] [--problems N]
#   ./swarm.sh start [--hours H] [--max-iters N] [--model M]
#   ./swarm.sh status
#   ./swarm.sh logs [agent]
#   ./swarm.sh stop
#   ./swarm.sh analyze
#
# Credentials: set ANTHROPIC_API_KEY. The OAuth credentials file is deliberately
# NOT used — it holds a short-lived access token that must be refreshed by
# writing back to the file, so N containers sharing it would race to rotate one
# token and can break the host login.

set -uo pipefail

SWARM_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SWARM_DIR/../.." && pwd)"
SWARM_ROOT="${SWARM_ROOT:-$HOME/.jjj-swarm}"
JJJ_BIN="${JJJ_BIN:-$REPO_ROOT/target/release/jjj}"
IMAGE="${SWARM_IMAGE:-jjj-swarm-agent:0.5.1}"

REMOTE="$SWARM_ROOT/remote.git"
SEED="$SWARM_ROOT/seed"
STOP_FILE="$SWARM_ROOT/STOP"
LOG="$SWARM_ROOT/jjj-invocations.jsonl"

die() { echo "swarm: $*" >&2; exit 1; }
info() { echo "  $*"; }

# --- build ------------------------------------------------------------------

cmd_build() {
    cp "$SWARM_DIR/jjj-shim" "$SWARM_DIR/container/jjj-shim"
    podman build -t "$IMAGE" "$SWARM_DIR/container" || die "image build failed"
    echo "Built $IMAGE"
}

# --- init -------------------------------------------------------------------

cmd_init() {
    local pods=2 agents=3 problems=8
    while [ $# -gt 0 ]; do
        case "$1" in
            --pods) pods="$2"; shift 2 ;;
            --agents) agents="$2"; shift 2 ;;
            --problems) problems="$2"; shift 2 ;;
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

    info "seeding workbench ($problems operations)"
    python3 "$SWARM_DIR/toy/seed.py" "$SEED" --problems "$problems" --jjj "$JJJ_BIN" --force \
        >"$SWARM_ROOT/logs/seed.log" 2>&1 || { cat "$SWARM_ROOT/logs/seed.log"; die "seeding failed"; }

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
pods=$pods
agents=$agents
problems=$problems
EOF

    echo
    echo "Ready: $pods pods x $agents agents = $total containers"
    echo "  ceiling: $(cd "$SEED" && ./score.py | awk '{print $2}') conformance cases"
    echo "  start with: $0 start --hours 1"
}

# --- start ------------------------------------------------------------------

cmd_start() {
    local hours=0 max_iters=0 model="sonnet"
    while [ $# -gt 0 ]; do
        case "$1" in
            --hours) hours="$2"; shift 2 ;;
            --max-iters) max_iters="$2"; shift 2 ;;
            --model) model="$2"; shift 2 ;;
            *) die "unknown option $1" ;;
        esac
    done

    [ -f "$SWARM_ROOT/config" ] || die "not initialised; run: $0 init"
    # shellcheck disable=SC1091
    . "$SWARM_ROOT/config"

    [ -n "${ANTHROPIC_API_KEY:-}" ] || die "ANTHROPIC_API_KEY is not set.
  A swarm needs a programmatic credential: the OAuth token in
  ~/.claude/.credentials.json is short-lived and must be refreshed by writing
  back to the file, so sharing it across containers races to rotate one token
  and can break your host login."

    rm -f "$STOP_FILE"

    local deadline=0
    if [ "$hours" != "0" ]; then
        deadline=$(python3 -c "import time,sys;print(int(time.time()+float(sys.argv[1])*3600))" "$hours")
    fi

    echo "Starting $((pods * agents)) agent containers (model=$model)"
    [ "$deadline" != "0" ] && echo "  deadline: $(date -r "$deadline" '+%H:%M:%S')"
    [ "$max_iters" != "0" ] && echo "  cap: $max_iters iterations per agent"

    for p in $(seq 1 "$pods"); do
        local pod="pod-$p"
        for a in $(seq 1 "$agents"); do
            local agent name
            agent="agent-$(printf '%02d' "$a")"
            name="swarm-$pod-$agent"
            podman rm -f "$name" >/dev/null 2>&1

            podman run -d --name "$name" \
                --memory 1g --cpus 1.5 \
                -e "JJJ_USER=$pod/$agent" \
                -e "JJJ_POD=$pod" \
                -e "SWARM_AGENT=$agent" \
                -e "SWARM_REMOTE=/swarm/remote.git" \
                -e "SWARM_LOG=/swarm/jjj-invocations.jsonl" \
                -e "SWARM_STOP=/swarm/STOP" \
                -e "SWARM_DEADLINE=$deadline" \
                -e "SWARM_MAX_ITERS=$max_iters" \
                -e "SWARM_MODEL=$model" \
                -e "ANTHROPIC_API_KEY=$ANTHROPIC_API_KEY" \
                -v "$SWARM_ROOT:/swarm:rw" \
                "$IMAGE" >/dev/null || die "failed to start $name"
            echo "  started $name"
        done
    done

    echo "  watch: $0 status    stop: $0 stop"
}

# --- status / logs / stop / analyze -----------------------------------------

cmd_status() {
    [ -f "$SWARM_ROOT/config" ] || die "not initialised"
    # shellcheck disable=SC1091
    . "$SWARM_ROOT/config"

    echo "containers:"
    podman ps -a --filter "name=swarm-" \
        --format "  {{.Names}}  {{.Status}}" 2>/dev/null | sort
    [ -e "$STOP_FILE" ] && echo "  (kill switch SET)"

    if [ -d "$SEED" ]; then
        echo "seed score: $(cd "$SEED" && ./score.py 2>/dev/null)"
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
