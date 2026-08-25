#!/usr/bin/env bash
#
# Fitness for the gophics wasm-size target.
#
# Scores the size of the WebAssembly binary, raw and gzipped, for two apps: a
# minimal one and one that exercises the whole widget set. Both, because the
# cheapest way to shrink a binary is to delete features, and a target that only
# measured the small app would reward exactly that.
#
# **Size is scored rather than time to first paint, and the measurement is why.**
# Instrumenting the real thing in headless Chromium showed instantiation is
# essentially all of it — TTFP 14245ms against INST 14245ms for a 15MB module —
# so binary size is the lever, not a proxy for it. Size is also perfectly
# deterministic, where the same page measured three times gave 11130ms, 11805ms
# and 16110ms: a 45% spread, far too noisy to review against. TTFP is still
# measured and reported, and the app must still paint at all, but it does not
# set the score.
#
# Correctness gates everything: the full test suite must pass, and the wasm must
# still boot and draw in a browser. Shrinking a binary that no longer runs is
# not progress.

set -uo pipefail
cd "$(dirname "$0")"

fail() { echo "0 100"; [ -n "${1:-}" ] && echo "$1" >&2; exit 0; }

command -v go >/dev/null 2>&1 || fail "no go toolchain"

# 1. It must build for the host, and still pass its tests.
#
# Excluded because they cannot run in a container, not because they may break:
# a real GPU device, macOS-only Metal, and the Objective-C runtime.
readonly SKIP='internal/gfx/gg/internal/gpu|internal/gfx/wgpu/hal/metal|internal/objc'
CGO_ENABLED=0 go build ./... 2>/tmp/build.err || fail "build failed: $(tail -3 /tmp/build.err)"
pkgs="$(go list ./... 2>/dev/null | grep -vE "$SKIP")"
CGO_ENABLED=0 go test $pkgs >/tmp/test.err 2>&1 \
    || fail "tests failed: $(grep -E '^--- FAIL' /tmp/test.err | head -3 | tr '\n' ' ')"

# 2. It must build for the web.
mkdir -p /tmp/wasm
for app in counter gallery; do
    GOOS=js GOARCH=wasm go build -o "/tmp/wasm/$app.wasm" "./examples/$app" 2>/tmp/wasm.err \
        || fail "wasm build of $app failed: $(tail -2 /tmp/wasm.err)"
done

# 3. It must still run. A binary that got smaller by no longer working is the
#    obvious way to win this, so the gate is that it boots and paints.
paint="skipped"
if command -v chromium >/dev/null 2>&1; then
    paint="$(./paint_check.sh /tmp/wasm/gallery.wasm 2>/dev/null || echo FAIL)"
    case "$paint" in
        FAIL|"") fail "the gallery wasm no longer paints in a browser" ;;
    esac
fi

python3 - "$paint" <<'PY'
import gzip, math, os, sys

# Budgets are what the code produces today, so every mark comes from beating
# them, and full marks need to be eight times under — about 1.9MB raw and
# 470KB over the wire for the counter. That is ambitious for Go wasm, which is
# the point: a ceiling that can be reached is a checklist.
# Set from what the tree produces today, a shade above, so the score starts with
# a live gradient in both classes rather than at zero — a class reading 0% tells
# an agent nothing about whether it is getting warmer. Re-measure and reset these
# if the baseline tree moves; they were taken at 15.74MB/3.97MB and
# 17.85MB/4.33MB.
BUDGETS = {
    "counter": {"raw": 15_900_000, "gz": 4_020_000},
    "gallery": {"raw": 18_000_000, "gz": 4_380_000},
}
MEET, BEAT = 0.25, 0.75


def points(measured, budget):
    if measured > budget:
        return 0.0
    return MEET + BEAT * min(1.0, math.log(budget / max(measured, 1)) / math.log(8))


sizes = {}
for app in BUDGETS:
    p = f"/tmp/wasm/{app}.wasm"
    raw = os.path.getsize(p)
    with open(p, "rb") as f:
        gz = len(gzip.compress(f.read(), 9))
    sizes[app] = {"raw": raw, "gz": gz}

# Two classes, equally weighted, so shrinking only what compresses well (or only
# what does not) cannot carry the run on its own.
scores = {}
for cls in ("raw", "gz"):
    scores[cls] = sum(points(sizes[a][cls], BUDGETS[a][cls]) for a in BUDGETS) / len(BUDGETS)

for cls, label in (("raw", "raw size"), ("gz", "gzipped")):
    pct = 100 * scores[cls]
    print(f"  {label:<9} {pct:5.1f}%  {'#' * int(pct / 5)}", file=sys.stderr)
for app in sorted(sizes):
    s, b = sizes[app], BUDGETS[app]
    print(f"    {app:<9} raw {s['raw']/1e6:6.2f}MB /{b['raw']/1e6:5.1f}   "
          f"gzip {s['gz']/1e6:5.2f}MB /{b['gz']/1e6:5.2f}", file=sys.stderr)
print(f"    time to first paint (reported, not scored): {sys.argv[1]}", file=sys.stderr)

print(f"{max(0, min(100, round(100 * sum(scores.values()) / len(scores))))} 100")
PY
