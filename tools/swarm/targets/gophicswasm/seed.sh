#!/usr/bin/env bash
#
# Seed a swarm workbench for the gophics wasm-size target.
#
# The workbench is a **clone with no origin**. Nothing an agent does can reach
# the real repository, and whatever survives is merge-gated by hand. A clone
# carries committed state only, so uncommitted work in the source tree is
# deliberately excluded — the swarm starts from a commit someone can name.
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

cp "$HERE/score.sh" "$HERE/verify.sh" "$HERE/paint_check.sh" "$ROOT/"
chmod +x "$ROOT/score.sh" "$ROOT/verify.sh" "$ROOT/paint_check.sh"

jj git init --colocate >/dev/null 2>&1 || true
jj config set --repo user.name "swarm-seed" >/dev/null 2>&1 || true
jj config set --repo user.email "swarm-seed@example.invalid" >/dev/null 2>&1 || true
"$JJJ" init >/dev/null

new_problem() {
    "$JJJ" problem new "$1" --priority "$2" --tags "$3" --body "$4" --force --json \
        | python3 -c 'import sys,json; print(json.load(sys.stdin)["id"])'
}

new_problem "Find out what is actually in the 15MB" critical "investigation,keystone" \
"Before cutting anything, measure. \`go tool nm -size -sort size\` on the wasm,
or \`go build -ldflags=-dumpdep\`, will say which packages and symbols dominate.
Report the top twenty with sizes.

An answer that refutes an assumption — 'reflection is not the problem, X is' —
is worth more to the others than a change that saves 2%. Nobody knows the shape
of this yet." >/dev/null

new_problem "Reflection and the type metadata it forces the linker to keep" critical "size,reflect" \
"Go's linker cannot prove a type is unused if anything reaches it through
\`reflect\`, so one reflective path can pin the metadata for a large slice of
the program. \`encoding/json\` is the usual culprit; there may be others here.

Removing a reflective dependency can be worth megabytes, and costs nothing at
runtime." >/dev/null

new_problem "Dependencies that the web build does not need" high "size,deps" \
"A wasm build should not carry desktop or mobile providers, native menus, or
anything behind a build tag it cannot use. Check what \`GOOS=js\` actually links
in — build tags that look exclusive sometimes are not." >/dev/null

new_problem "Cut the gallery without cutting features" high "size,deps" \
"The gallery is 17MB because it exercises everything, which is the point: it is
the proof that shrinking did not come from deleting functionality. If it can be
made smaller while still passing its tests and still painting, that is a real
win." >/dev/null

new_problem "Fonts and other embedded assets" medium "size,assets" \
"Embedded TTFs and any other \`go:embed\` data sit in the binary uncompressed.
Subsetting, compressing, or loading them at runtime are all options — the last
one trades size for a network round trip, so measure before assuming." >/dev/null

new_problem "Can TinyGo build gophics, and what would it take?" critical "size,tinygo,keystone" \
"TinyGo produces far smaller WebAssembly than the standard toolchain — an order
of magnitude, not a few percent — so it is the largest single lever available
and the first thing worth settling.

It does not work today, and the reason is narrower than it looks. TinyGo 0.40
(installed; run \\`tinygo version\\`) supports Go 1.19 through 1.25, and gophics'
go.mod requires >= 1.26.5:

    tinygo build -target wasm ./examples/counter
    -> requires go version 1.19 through 1.25, got go1.26

That is a version floor, not a language barrier. The questions, in order:

  1. What in gophics actually needs Go 1.26? If nothing does, lowering the
     go.mod directive to 1.25 costs nothing and unblocks the experiment.
  2. With the floor lowered, does TinyGo build it — and if not, what breaks?
     TinyGo's reflection support and standard-library coverage are narrower
     than the standard toolchain, and gophics may rely on both.
  3. If it builds, does it still pass the tests and still paint?

**Report the answer even if it is no.** 'TinyGo cannot work because X' is worth
more to everyone than another 2% shaved elsewhere, and it stops five other
agents investigating the same thing. If it is a dead end, say precisely where it
dead-ends." >/dev/null

new_problem "Build flags and linker options worth having" medium "size,build" \
"\`-ldflags=\"-s -w\"\` saves about 2.6% here, which is small — the bulk is real
code, not symbols. Look for anything else the toolchain offers, and record what
does *not* help so nobody tries it twice." >/dev/null

cat > "$ROOT/SWARM.md" <<'BRIEF'
# The target

Make the gophics WebAssembly binary smaller, without making gophics smaller.

## Where it stands

| app | raw | gzipped |
|---|---|---|
| counter (minimal) | 14.99 MB | 3.78 MB |
| gallery (everything) | 16.98 MB | 4.12 MB |

Baseline is about 25. `-ldflags="-s -w"` is worth 2.6%, so the easy win is not
there: the bulk is real code.

**TinyGo is installed, it builds gophics, and it is 3.3x smaller.** Measured on
this tree: `tinygo -no-debug` gives 3.79MB raw / 1.38MB gzipped against standard
Go `-s -w` at 12.48MB / 3.19MB. A real browser renders both pixel-identically
and both reach first paint at about 168ms.

Two traps, both of which have already caught someone:

- **Always pass `-no-debug`.** Without it TinyGo carries DWARF and measures
  11.27MB — 19% smaller, which looks like a rounding error rather than the
  answer. One flag is the whole difference.
- **TinyGo checks the installed toolchain, not go.mod.** It shells out to
  `go version`; lowering the go.mod directive is necessary but not sufficient.
  A Go 1.25 toolchain must be on PATH for the tinygo invocation.

What TinyGo does *not* buy is first paint. It instantiates about 34ms faster,
inside roughly 168ms — the remaining ~150ms is app init (fonts, harfbuzz, GPU
pipeline setup) and is identical under both toolchains. If first paint is the
goal, that init is the target.

## The score

`./score.sh` prints `<score> 100`. Two classes, equally weighted:

| class | what it measures |
|---|---|
| raw size | the bytes the runtime must parse and instantiate |
| gzipped | the bytes that go over the wire |

Both apps count. Measuring only the minimal one would reward deleting features,
which is the cheapest way to win this and the least useful.

Budgets are what the code produces today, so every mark comes from beating them
and full marks need to be **eight times** under. There is no point at which the
score stops paying for a smaller binary.

## Why size, and why first paint is not measured here

Size this environment measures exactly. First paint it cannot measure at all,
and the harness no longer pretends to: headless, GPU-less and virtual-timed, it
reported 11-27 seconds for an app that a real browser with WebGPU paints in
168ms — wrong by a factor of 67 or more, because gophics falls back to software
rasterization without a GPU and virtual time decouples the clock.

A conclusion was built on those numbers once and was wrong twice over. Do not
rebuild it. `./paint_check.sh` now answers only "does it still paint".

## Correctness gates everything

- The full test suite must pass.
- The wasm must still **boot and paint**: `./paint_check.sh` loads it in headless
  Chromium, waits for gophics to size its canvas, and reads the canvas back as a
  PNG. A blank canvas compresses to a few hundred bytes, so the gate is that
  several kilobytes of real pixels arrive. A binary that got smaller by no longer
  working scores zero.

`./verify.sh` is the cheap pre-push check; `./score.sh` runs the full gate.

Three packages are excluded because they cannot run in a container, not because
they may break: `internal/gfx/gg/internal/gpu` needs a real GPU device,
`internal/gfx/wgpu/hal/metal` is macOS-only, and `internal/objc` needs the
Objective-C runtime. **GPU work is out of scope** — it cannot be verified here,
and a number that cannot be verified is worse than no number.

## How work lands

Nothing reaches the shared branch except through review:

    jjj solution new "..." --body "what and why"  --problem <id>
    jjj solution attach <id>      # link your jj change
    jjj solution submit <id>      # publishes the diff for a reviewer
    -> a critic reviews the real diff and approves, or critiques it
    -> approved work is merged to main automatically

## Rules

- Measure before and after, in the same turn, and put the byte counts in the body.
- A critique must cite a size or a failing test, not an opinion.
- Record what did **not** work. A dead end someone else will otherwise repeat is
  worth writing down; this codebase is large and nobody can hold all of it.
- A change that shrinks the binary and breaks a platform is not a win. Say so.
BRIEF

score="$(./score.sh 2>/dev/null | tail -1)"
echo "baseline: $score"
echo "problems: $("$JJJ" problem list --json | python3 -c 'import sys,json;print(len(json.load(sys.stdin)))')"
