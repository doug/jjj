#!/usr/bin/env bash
#
# Does this wasm still boot and draw?
#
# The cheapest way to shrink a binary is to break it, so size is only meaningful
# next to proof the thing still runs. Loads the module in headless Chromium,
# waits for gophics to create and size its canvas, and reads back both the time
# and the length of the canvas as a PNG — a blank canvas compresses to almost
# nothing, so a few KB of PNG is evidence that pixels actually landed.
#
# Prints "PAINTED elapsed=<ms> ... PNGLEN=<n>" on success, or nothing and a
# non-zero exit. `elapsed` is diagnostic only — see the note below.
#
# Usage: paint_check.sh <path-to.wasm>

set -uo pipefail
WASM="${1:?wasm path required}"
DIR="$(mktemp -d)"
trap 'rm -rf "$DIR"; [ -n "${SRV:-}" ] && kill "$SRV" 2>/dev/null' EXIT

cp "$WASM" "$DIR/app.wasm"
cp "$(go env GOROOT)/lib/wasm/wasm_exec.js" "$DIR/" 2>/dev/null \
    || cp "$(go env GOROOT)/misc/wasm/wasm_exec.js" "$DIR/" 2>/dev/null \
    || { echo "no wasm_exec.js in GOROOT" >&2; exit 1; }

cat > "$DIR/index.html" <<'HTML'
<!doctype html>
<html><head><meta charset="utf-8">
<style>html,body{margin:0;height:100%;background:#fff}</style>
<script src="wasm_exec.js"></script>
<script>
var t0 = performance.now(), inst = -1;
function M(){ return document.getElementById("__m"); }
(async function(){
  try {
    var buf = await (await fetch("app.wasm")).arrayBuffer();
    var go = new Go();
    var r = await WebAssembly.instantiate(buf, go.importObject);
    inst = performance.now() - t0;
    go.run(r.instance);
  } catch (e) { M().textContent = "ERR " + e; }
})();
var poll = setInterval(function(){
  var c = document.querySelector('canvas');
  if (c && c.width > 0) {
    clearInterval(poll);
    var t = performance.now() - t0, px = "na";
    try { px = String(c.toDataURL().length); } catch (e) { px = "err"; }
    M().textContent = "PAINTED elapsed=" + t.toFixed(0) + " inst=" + inst.toFixed(0)
                    + " CANVAS=" + c.width + "x" + c.height + " PNGLEN=" + px;
  }
}, 5);
</script></head><body><div id="__m" style="display:none">PENDING</div></body></html>
HTML

cd "$DIR"
python3 -m http.server 8099 >/dev/null 2>&1 &
SRV=$!
sleep 1

# A gate, not a measurement.
#
# This answers "does it still paint" and nothing else. The elapsed number it
# prints is not a time to first paint and must not be read as one: measured
# here it came out around 11-27 seconds for an app that a real browser paints in
# 168ms — off by a factor of 67 or more. Three things conspire, and no flag
# fixes them, because they are what this environment *is*:
#
#   - `--disable-gpu`, so gophics falls back to its software rasterizer and the
#     first frame is CPU-rasterized rather than drawn through WebGPU
#   - `--virtual-time-budget`, which decouples performance.now() from real time
#   - a CPU-capped container shared with five other agents
#
# A conclusion was once built on this number — "TinyGo is 5x faster to first
# paint" — and it was wrong: measured in a real browser with WebGPU, standard Go
# and TinyGo both reach first paint at about 168ms, and the 34ms TinyGo saves on
# instantiate sits inside ~150ms of app init (fonts, harfbuzz, pipeline setup)
# that is identical in both. Score size, which this environment *can* measure
# exactly, and measure first paint where there is a GPU.
timeout 180 chromium --headless --no-sandbox --disable-gpu --disable-dev-shm-usage \
    --run-all-compositor-stages-before-draw --virtual-time-budget=60000 \
    --dump-dom http://127.0.0.1:8099/index.html 2>/dev/null > "$DIR/dom.html"

python3 - "$DIR/dom.html" <<'PY'
import re, sys
s = open(sys.argv[1]).read()
m = re.search(r'id="__m"[^>]*>([^<]*)<', s)
v = m.group(1) if m else ""
# A blank canvas is a few hundred bytes of PNG; real content is kilobytes.
n = re.search(r"PNGLEN=(\d+)", v)
if not v.startswith("PAINTED") or not n or int(n.group(1)) < 2000:
    sys.exit(1)
print(v)
PY
