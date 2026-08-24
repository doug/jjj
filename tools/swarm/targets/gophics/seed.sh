#!/usr/bin/env bash
#
# Seed a swarm workbench for the gophics frame-cost target.
#
# The workbench is a **clone with no origin**. Nothing an agent does can reach
# the real repository, and whatever survives is merge-gated by hand. The source
# is only ever read: cloned, never written, never checked out into.
#
# Note that a clone carries committed state only. Uncommitted work in the source
# tree is deliberately not included — the swarm should start from a commit
# someone can name, not from whatever happened to be open in an editor.
#
# Usage: seed.sh <workbench-dir> <repo-root> <jjj-binary>

set -euo pipefail

ROOT="${1:?workbench directory required}"
REPO="${2:?repo root required}"
JJJ="${3:?jjj binary required}"
HERE="$(cd "$(dirname "$0")" && pwd)"
SRC="${GOPHICS_SRC:-$HOME/src/gophics}"

[ -d "$SRC/.git" ] || { echo "no gophics checkout at $SRC" >&2; exit 1; }

rm -rf "$ROOT"
git clone -q --no-hardlinks "$SRC" "$ROOT"
cd "$ROOT"
git remote remove origin 2>/dev/null || true
git config user.name "swarm-seed"
git config user.email "swarm-seed@example.invalid"

cp "$HERE/score.sh" "$HERE/verify.sh" "$ROOT/"
chmod +x "$ROOT/score.sh" "$ROOT/verify.sh"

jj git init --colocate >/dev/null 2>&1 || true
jj config set --repo user.name "swarm-seed" >/dev/null 2>&1 || true
jj config set --repo user.email "swarm-seed@example.invalid" >/dev/null 2>&1 || true
"$JJJ" init >/dev/null

new_problem() {
    "$JJJ" problem new "$1" --priority "$2" --tags "$3" --body "$4" --force --json \
        | python3 -c 'import sys,json; print(json.load(sys.stdin)["id"])'
}

new_problem "A full repaint does not fit in a 60fps frame" critical "latency,paint,keystone" \
"\`BenchmarkFrameFullRepaint\` takes about 27ms. A 60fps frame is 16.67ms, so a
full repaint drops a frame every time it happens.

This is the one target with an external standard rather than a
beat-the-status-quo budget: 16.67ms is what a frame *is*. Until it is met the
latency class scores zero on that benchmark, and that is accurate." >/dev/null

new_problem "A frame allocates 5MB even when nothing changed" critical "allocations,gc,keystone" \
"\`BenchmarkFrameUnchanged\` allocates 5.12MB per frame across only 2
allocations — so it is a small number of very large buffers, not death by a
thousand cuts. Find out what is being allocated per frame that could be
retained, pooled, or sized to the damage rather than the surface.

Allocation drives GC and GC drives jank, so this is upstream of the latency
problem rather than separate from it." >/dev/null

new_problem "Find what a repaint actually spends its time on" high "latency,investigation" \
"Before optimising, measure. \`go test ./app -run '^\$' -bench BenchmarkFrame
-cpuprofile\` and report where the time goes. A profile that refutes an
assumption is worth more than a change that helps a little." >/dev/null

new_problem "A localized change repaints more than it needs to" high "latency,paint" \
"\`BenchmarkFrameLocalizedChange\` allocates as much as a full repaint and
takes 299 allocations to it. If one widget changed, the work should be
proportional to that widget, not to the scene." >/dev/null

new_problem "Reduce allocations on the layout path" medium "allocations,layout" \
"Single-pass constraint layout should be able to run without allocating per
node. Look for slices and maps rebuilt each frame that could be reused across
frames." >/dev/null

new_problem "Reduce allocations on the paint/record path" medium "allocations,paint" \
"Scene recording builds a display list every frame. Look for what could be
retained between frames when the scene is unchanged." >/dev/null

cat > "$ROOT/SWARM.md" <<'BRIEF'
# The target

Make a gophics frame cheaper: faster, and with less garbage.

This is a real UI toolkit — 166 packages, ~800K lines — and no agent can hold
it all. Read what you need, change as little as you can, and say what you
measured.

## The score

`./score.sh` prints `<score> 100` and a per-class breakdown. Three classes,
equally weighted, so no single win carries the run and latency cannot be bought
with allocations or the reverse:

| class | what it measures |
|---|---|
| latency | ns/op against a frame budget |
| bytes | B/op — bytes allocated per frame |
| allocs | allocs/op — allocation count per frame |

Baseline is about 51.

**Two kinds of budget, deliberately.** Latency has a real external bar: 16.67ms
is a 60fps frame, and a full repaint takes ~27ms, so that reads zero until it is
fixed. Allocation has no such standard, so its budget is what the code does
today — every mark there comes from beating the status quo, and full marks need
to be **eight times** under it. There is no point at which the score stops
paying for an improvement.

**The allocation counters are the honest ones.** `B/op` and `allocs/op` are
counted, not timed, so six agents sharing a machine cannot move them. Timings
can drift with load; compare a before and after from the same turn.

## Correctness gates everything

A fast frame that draws the wrong pixels scores zero. The golden-image tests are
what say so, and they are strict — an optimisation that skips a paint, reuses a
buffer it should have cleared, or reorders a layer will still compile and still
be fast. Run `./verify.sh` before you push; it is the cheap check.

Two packages are excluded because they cannot run here, not because they may
break: `internal/gfx/gg/internal/gpu` needs a real GPU device, and
`internal/gfx/wgpu/hal/metal` is macOS-only. **GPU work is out of scope for this
trial** — it cannot be verified in a container, and a number that cannot be
verified is worse than no number.

## How work lands

Nothing reaches the shared branch except through review:

    jjj solution new "..." --body "what and why"  --problem <id>
    jjj solution attach <id>      # link your jj change
    jjj solution submit <id>      # publishes the diff for a reviewer
    -> a critic reviews the real diff and approves, or critiques it
    -> approved work is merged to main automatically

## Rules

- Measure before and after, in the same turn, and put the numbers in the body.
- A critique must cite a benchmark line or a failing test, not an opinion.
- Prefer the smallest change that moves a number. This codebase is someone's
  working project, not a scratchpad.
- A change that helps one class and hurts another is a trade, not a win. Report
  both.
BRIEF

score="$(./score.sh 2>/dev/null | tail -1)"
echo "baseline: $score"
echo "problems: $("$JJJ" problem list --json | python3 -c 'import sys,json;print(len(json.load(sys.stdin)))')"
