#!/usr/bin/env bash
#
# Fitness for the gophics frame-cost target.
#
# Scores what a UI toolkit is actually judged on: whether a frame fits in its
# budget, and how much garbage it makes getting there. Both come from
# benchmarks the project already had.
#
# **Allocation counters carry most of the weight, deliberately.** `B/op` and
# `allocs/op` are counted, not timed, so six agents sharing a machine cannot
# move them — where wall-clock scored an unmodified tree anywhere from 0 to 28
# in an earlier trial. They are also causally upstream of the thing being
# optimised: allocation drives GC, GC drives jank. A frame that allocates 5MB
# while nothing on screen changed is the defect, and the millisecond count is
# the symptom.
#
# Latency is still scored, because the budget is the point, but it is measured
# as the minimum across repetitions and weighted alongside two counters that
# contention cannot touch.
#
# Correctness gates everything: a fast frame that draws the wrong pixels is
# worth nothing, and the golden-image suite is what says so.

set -uo pipefail
cd "$(dirname "$0")"

fail() { echo "0 100"; [ -n "${1:-}" ] && echo "$1" >&2; exit 0; }

command -v go >/dev/null 2>&1 || fail "no go toolchain"

# 1. It must build.
CGO_ENABLED=0 go build ./... 2>/tmp/build.err || fail "build failed: $(tail -3 /tmp/build.err)"

# 2. It must still draw the right pixels.
#
# Two packages are excluded because they cannot pass in a container, not
# because they are allowed to break: `internal/gfx/gg/internal/gpu` needs a
# real GPU device, and `internal/gfx/wgpu/hal/metal` is macOS-only and will not
# build on Linux at all. Everything else — including every golden-image test —
# passes here, verified before this target existed.
# `internal/objc` binds the Objective-C runtime and needs cgo, which this
# scorer disables everywhere for the sake of the zero-CGo guarantee. On Linux it
# is excluded by build constraints and never appears; on macOS it appears and
# fails with "NSString class not found", which made the score read 0 on the host
# while reading 51 in the container. A scorer that only works in one environment
# is a scorer somebody will misread.
readonly SKIP='github.com/doug/gophics/internal/gfx/gg/internal/gpu|github.com/doug/gophics/internal/gfx/wgpu/hal/metal|github.com/doug/gophics/internal/objc'
pkgs="$(go list ./... 2>/dev/null | grep -vE "$SKIP")"
if ! CGO_ENABLED=0 go test $pkgs >/tmp/test.err 2>&1; then
    fail "tests failed: $(grep -E '^--- FAIL' /tmp/test.err | head -3 | tr '\n' ' ')"
fi

# 3. Measure.
if ! CGO_ENABLED=0 go test ./app -run '^$' -bench 'BenchmarkFrame' \
        -benchtime "${GOPHICS_BENCHTIME:-30x}" -count "${GOPHICS_COUNT:-3}" \
        >/tmp/bench.out 2>&1; then
    fail "benchmarks failed: $(tail -3 /tmp/bench.out)"
fi

python3 - <<'PY'
import re, sys

# Budgets. Frame latency is the 60fps budget; a full repaint gets the whole
# frame, the incremental paths get a fraction of it because in a real app they
# share the frame with everything else.
#
# The two kinds of budget here are set differently, on purpose.
#
# Latency has a real external bar — 16.67ms is a 60fps frame, and a full
# repaint currently takes 27ms, so that class *should* read zero until it is
# fixed. That is the headline defect, not a scoring artefact.
#
# Allocation has no such external standard, so its budget is simply what the
# code does today, set just above the measured figure. Every mark in those
# classes therefore comes from beating the status quo, and full marks need to
# be eight times under it: 640KB and a handful of allocations for a frame in
# which nothing changed. Setting them any tighter would make the class read
# zero with no gradient, which tells an agent nothing about whether it is
# getting warmer.
BUDGETS = {
    # A frame in which nothing changed should cost almost nothing; one in which
    # a single widget changed should cost a fraction of a full repaint. The
    # first pair were 1ms and 4ms, loose enough that a localized change was
    # already eight times inside and could earn nothing more however much it
    # improved — a budget nobody can beat is not a budget.
    "BenchmarkFrameUnchanged":       {"ns": 500_000,    "b": 5_150_000, "allocs": 4},
    "BenchmarkFrameLocalizedChange": {"ns": 1_000_000,  "b": 5_180_000, "allocs": 320},
    "BenchmarkFrameFullRepaint":     {"ns": 16_670_000, "b": 5_250_000, "allocs": 620},
}

best = {}
for line in open("/tmp/bench.out"):
    m = re.match(r"(Benchmark\w+)(?:-\d+)?\s+\d+\s+([\d.]+) ns/op\s+(\d+) B/op\s+(\d+) allocs/op", line)
    if not m:
        continue
    name, ns, b, a = m.group(1), float(m.group(2)), int(m.group(3)), int(m.group(4))
    cur = best.setdefault(name, {"ns": ns, "b": b, "allocs": a})
    # Minimum across repetitions: noise only ever adds, so the floor is the
    # best estimate of the uncontended cost.
    cur["ns"] = min(cur["ns"], ns)
    cur["b"] = min(cur["b"], b)
    cur["allocs"] = min(cur["allocs"], a)

if len(best) != len(BUDGETS):
    print(f"expected {len(BUDGETS)} benchmarks, parsed {len(best)}", file=sys.stderr)
    print("0 100")
    raise SystemExit

MEET = 0.25   # meeting the budget is table stakes
BEAT = 0.75   # the rest is earned by going under it


def points(measured, budget):
    """A quarter for meeting the budget, the rest for beating it.

    Meeting is worth little on purpose. An earlier split awarded half, which put
    the starting score at 51 before anything had been improved — half the marks
    banked for the status quo, which reads as "half done" when nothing has been
    done. Most of the scale should be the part that has to be earned.

    Full marks need to be eight times inside, so the scale never stops paying
    for an improvement: a ceiling that can be reached is a checklist, and a
    fleet finishes a checklist in an afternoon.
    """
    import math
    if measured > budget:
        return 0.0
    ratio = budget / max(measured, 1e-9)
    return MEET + BEAT * min(1.0, math.log(ratio) / math.log(8))

# Three classes, equally weighted, so no single win carries the run — and in
# particular so latency cannot be bought with allocations or the reverse.
classes = {"latency": "ns", "bytes": "b", "allocs": "allocs"}
scores = {}
for cls, key in classes.items():
    got = sum(points(best[n][key], BUDGETS[n][key]) for n in BUDGETS)
    scores[cls] = got / len(BUDGETS)

for cls in classes:
    pct = 100 * scores[cls]
    print(f"  {cls:<8} {pct:5.1f}%  {'#' * int(pct / 5)}", file=sys.stderr)
for n in sorted(BUDGETS):
    v, bud = best[n], BUDGETS[n]
    print(f"    {n:<30} {v['ns']/1e6:8.2f}ms /{bud['ns']/1e6:6.1f}   "
          f"{v['b']/1e6:6.2f}MB /{bud['b']/1e6:5.1f}   "
          f"{v['allocs']:5d} allocs /{bud['allocs']:5d}", file=sys.stderr)

print(f"{max(0, min(100, round(100 * sum(scores.values()) / len(scores))))} 100")
PY
