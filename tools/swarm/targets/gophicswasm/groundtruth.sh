#!/usr/bin/env bash
#
# Does the scorer measure what it claims to?
#
# This is the check nobody ran, and its absence cost more than any bug in the
# agents' code. A fitness function is the one artifact the swarm cannot
# critique: agents optimise against it and reviewers verify against it, so an
# error inside it is invisible to the whole process and is amplified by it.
# Two errors got through that way — TinyGo measured carrying DWARF and reported
# as "19% smaller, not worth it" when stripped it is 3.3x, and a headless
# browser reporting 11-27 seconds to first paint for an app that paints in
# 168ms — and both produced confident, wrong conclusions that survived review.
#
# So: assert facts about the measurement itself, against ground truth taken a
# different way. Run by preflight before any trial.

set -uo pipefail
cd "$(dirname "$0")"

pass=0; fail=0
ok()  { pass=$((pass+1)); printf '    ok   %s\n' "$1"; }
bad() { fail=$((fail+1)); printf '    FAIL %s\n' "$1"; [ -n "${2:-}" ] && printf '         %s\n' "$2"; }

TMP="$(mktemp -d)"; trap 'rm -rf "$TMP"' EXIT

# 1. The size the scorer reports must be the size on disk.
#
# Trivial, and exactly the kind of thing that is never checked until a unit
# conversion or a stale path makes it silently false.
GOOS=js GOARCH=wasm go build -ldflags="-s -w" -o "$TMP/a.wasm" ./examples/counter 2>/dev/null
disk=$(wc -c < "$TMP/a.wasm" | tr -d ' ')
py=$(python3 -c "import os;print(os.path.getsize('$TMP/a.wasm'))")
[ "$disk" = "$py" ] && ok "reported size matches the file on disk ($disk bytes)" \
                    || bad "size disagrees with the file" "wc=$disk python=$py"

# 2. Stripping debug information must make the binary smaller.
#
# The assertion that would have caught the TinyGo error. If a build that should
# be smaller is not, the flags are not reaching the compiler.
GOOS=js GOARCH=wasm go build -o "$TMP/fat.wasm" ./examples/counter 2>/dev/null
fat=$(wc -c < "$TMP/fat.wasm" | tr -d ' ')
if [ "$disk" -lt "$fat" ]; then
    ok "stripped build is smaller than unstripped ($((fat/1000000))MB -> $((disk/1000000))MB)"
else
    bad "stripping did not shrink the binary" "the -ldflags are not being applied"
fi

# 3. Both toolchains must be compared with debug information stripped.
#
# TinyGo carrying DWARF measures 11.27MB and looks like a 19% option; with
# -no-debug it is 3.79MB against standard Go's 12.48MB. If a run ever reports
# TinyGo as a marginal win, this is why.
if command -v tinygo >/dev/null 2>&1 && [ -d /tmp/go125root ]; then
    export GOROOT=/tmp/go125root PATH="/tmp/go125root/bin:$PATH"
    if tinygo build -target wasm -no-debug -o "$TMP/t.wasm" ./examples/counter 2>/dev/null; then
        t=$(wc -c < "$TMP/t.wasm" | tr -d ' ')
        [ "$t" -lt "$disk" ] && ok "tinygo -no-debug beats standard go ($((t/100000))00KB vs $((disk/1000000))MB)" \
                             || bad "tinygo is not smaller than standard go" "check -no-debug is being passed"
    else
        ok "tinygo present but did not build (reported, not fatal)"
    fi
else
    ok "tinygo comparison skipped (no go1.25 toolchain staged)"
fi

# 4. The score must move the right way.
#
# A scorer that reports a stable number regardless of the code is worse than no
# scorer, because it looks like it is working. Make the binary bigger on purpose
# and require the score to fall.
base="$(./score.sh 2>/dev/null | tail -1 | cut -d' ' -f1)"
cat > examples/counter/_bloat.go <<'GO'
//go:build ignore_me_bloat

package main
GO
# Real bloat: a large embedded blob the linker cannot drop.
python3 - <<'PY'
import pathlib
p = pathlib.Path("examples/counter/bloat_gen.go")
blob = ",".join(str(i % 251) for i in range(400000))
p.write_text("package main\n\nvar bloatBlob = [...]byte{" + blob + "}\n\nfunc init() { _ = bloatBlob[0] }\n")
PY
bloated="$(./score.sh 2>/dev/null | tail -1 | cut -d' ' -f1)"
rm -f examples/counter/bloat_gen.go examples/counter/_bloat.go
if [ -n "$base" ] && [ -n "$bloated" ] && [ "$bloated" -lt "$base" ]; then
    ok "score falls when the binary grows ($base -> $bloated)"
else
    bad "score did not fall for a deliberately larger binary ($base -> $bloated)" \
        "the scorer may not be measuring the build it thinks it is"
fi

printf '  ground truth: %d passed, %d failed\n' "$pass" "$fail"
[ "$fail" -eq 0 ]
