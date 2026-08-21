#!/usr/bin/env bash
#
# Fitness for the jjj self-improvement target.
#
# Prints "<score> <ceiling>" like every other target, so the harness and
# watchdog need no special case. Here the score is a *scaling ratio* scaled to
# an integer: jjj's own time on a large corpus divided by its time on a small
# one, for a delta-sized sync.
#
#   score = 100 / ratio, so it never saturates and stays intuitive:
#     ratio 3.2x (today) -> score  31     cost still grows with the corpus
#     ratio 2.0x         -> score  50
#     ratio 1.0x (target)-> score 100     cost no longer depends on the corpus
#
# A ratio, not milliseconds, because this is measured on a machine the swarm
# itself saturates.
#
# **The ratio alone was not enough.** The first version timed wall-clock on the
# assumption that contention taxes both corpora equally. It does not: the large
# corpus holds more memory and does more I/O, so it loses disproportionately.
# Measured on an unmodified tree, six concurrent agents scored it 0, 6, 7, 8, 9
# and 28 — noise far larger than the improvement it exists to detect, which
# would have had reviewers accepting and rejecting on coin flips. The benchmark
# now measures **CPU time** (see tools/bench/sync_scaling.py), which a stolen
# timeslice does not inflate, and takes the minimum of repetitions.
#
# Repeatability within a load condition is ~3%. There is still a systematic
# offset *between* conditions — the same tree reads 3.2x idle and 2.15x with
# every core busy — so compare before against after within a turn, and do not
# compare a score to one from another run under different load.
#
# Correctness gates the score. A build that fails, or a suite that fails, scores
# zero however fast it is: making sync fast by breaking it is not progress.

set -uo pipefail
cd "$(dirname "$0")"

# 500 -> 5000 is a 10x corpus growth, which shows the effect clearly, and one
# scoring run costs about 20s — affordable every agent turn.
SMALL="${SYNC_SMALL:-500}"
LARGE="${SYNC_LARGE:-5000}"
DELTA="${SYNC_DELTA:-50}"

fail() { echo "0 100"; [ -n "${1:-}" ] && echo "$1" >&2; exit 0; }

# 1. It must build.
cargo build --release --quiet 2>/tmp/build.err || fail "build failed: $(tail -3 /tmp/build.err)"

# 2. It must still be correct. Fast tests only — the full suite runs at the
#    merge gate, but a broken build must never score above zero here.
if ! cargo test --release --quiet --lib >/tmp/test.err 2>&1; then
    fail "lib tests failed: $(grep -E '^test .* FAILED|panicked' /tmp/test.err | head -3)"
fi

# 3. Measure.
out=$(python3 tools/bench/sync_scaling.py \
        --small "$SMALL" --large "$LARGE" --delta "$DELTA" \
        --jjj "$(pwd)/target/release/jjj" 2>/dev/null) || fail "benchmark failed"

# Only the ratio block, not the per-corpus timing lines above it, which also
# begin "  push".
ratios=$(echo "$out" | sed -n '/scaling ratio/,$p')
push_ratio=$(echo "$ratios" | awk '/^  push /{print $2}' | tr -d 'x')
fetch_ratio=$(echo "$ratios" | awk '/^  fetch /{print $2}' | tr -d 'x')
[ -z "$push_ratio" ] && fail "could not parse the benchmark output"

# Score both operations: the worse one dominates, so neither can be ignored.
python3 - "$push_ratio" "$fetch_ratio" <<'PY'
import sys
push = float(sys.argv[1])
fetch = float(sys.argv[2]) if len(sys.argv) > 2 and sys.argv[2] else push
worst = max(push, fetch)
# 100/ratio: 1.0x -> 100, 2.0x -> 50, 4.0x -> 25. Never bottoms out at zero, so
# early progress is visible instead of being clamped away — a scale that reads 0
# both for "no progress" and "some progress" hides exactly what a trial needs.
score = max(1, min(100, round(100.0 / max(worst, 1.0))))
print(f"{score} 100")
print(f"  push {push:.2f}x  fetch {fetch:.2f}x  (1.0x = delta-proportional)", file=sys.stderr)
PY
