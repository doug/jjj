#!/usr/bin/env bash
#
# Rehearse the whole loop with one scripted agent before a real trial starts.
#
# Every serious failure in a day of trials was silent. The fleet stayed up, the
# sampler kept writing rows, the scores looked plausible, and the run produced
# nothing — a too-old git so `jjj fetch` failed and six agents idled politely;
# review branches named inside the metadata glob so 84% of pushes died; a
# hardcoded `cargo build` gate on a Python workbench so no work ever merged.
# Each cost hours and each is a one-line assertion here.
#
# This is deliberately NOT a unit test of the pieces. It exercises the path the
# pieces have to form: clone -> fetch -> claim -> edit -> score -> submit ->
# review -> approve -> merge. Every one of those bugs lived in a seam, not in a
# component.
#
# No model calls, so it costs about two minutes and no tokens.
#
# Usage: preflight.sh [--target toy|sql|sqlperf|jjj] [--keep]

set -uo pipefail

SWARM_DIR="$(cd "$(dirname "$0")" && pwd)"
TARGET="sqlperf"
KEEP=0

while [ $# -gt 0 ]; do
    case "$1" in
        --target) TARGET="$2"; shift 2 ;;
        --keep) KEEP=1; shift ;;
        *) echo "preflight: unknown option $1" >&2; exit 2 ;;
    esac
done

ROOT="${SWARM_PREFLIGHT_ROOT:-$HOME/.jjj-swarm-preflight}"
IMAGE="${SWARM_IMAGE:-jjj-swarm-agent:0.5.1}"

pass=0; fail=0
ok()   { pass=$((pass+1)); printf '  \033[32mok\033[0m   %s\n' "$1"; }
bad()  { fail=$((fail+1)); printf '  \033[31mFAIL\033[0m %s\n' "$1"; [ -n "${2:-}" ] && printf '       %s\n' "$2"; }
step() { printf '\n\033[1m%s\033[0m\n' "$1"; }

cleanup() {
    podman rm -f preflight-agent >/dev/null 2>&1
    [ "$KEEP" = 1 ] || rm -rf "$ROOT"
}
trap cleanup EXIT

step "image"
podman image exists "$IMAGE" || { bad "image $IMAGE is not built (run: ./swarm.sh build)"; exit 1; }
ok "image present"

# jj needs git >= 2.41 for `git fetch --porcelain`. On an older git every
# `jjj fetch` fails, agents see an empty repository, and a fleet idles for
# twenty minutes looking perfectly healthy.
gitver="$(podman run --rm --entrypoint git "$IMAGE" --version 2>/dev/null | awk '{print $3}')"
if [ -z "$gitver" ]; then
    bad "no git in the image"
else
    maj="${gitver%%.*}"; rest="${gitver#*.}"; min="${rest%%.*}"
    if [ "$maj" -gt 2 ] || { [ "$maj" -eq 2 ] && [ "$min" -ge 41 ]; }; then
        ok "git $gitver (jj needs >= 2.41)"
    else
        bad "git $gitver is too old for jj — every \`jjj fetch\` will fail"
    fi
fi

step "seed"
rm -rf "$ROOT"
SWARM_ROOT="$ROOT" "$SWARM_DIR/swarm.sh" init --target "$TARGET" >"${TMPDIR:-/tmp}/preflight-seed.log" 2>&1 \
    && ok "seeded a $TARGET workbench" \
    || { bad "seeding failed" "$(tail -3 "${TMPDIR:-/tmp}/preflight-seed.log")"; exit 1; }

base="$(grep -oE '^  baseline: [0-9]+' "${TMPDIR:-/tmp}/preflight-seed.log" | tail -1 | awk '{print $2}')"
if [ -n "$base" ] && [ "$base" -gt 0 ]; then
    ok "baseline scores $base (non-zero, so a broken tree is distinguishable)"
else
    bad "baseline is ${base:-unset}" "a target that starts at 0 cannot tell 'minimal' from 'broken'"
fi

# A shared bare repo written by containers whose uids differ from the host's.
# Without this every concurrent push risks "unable to open loose object:
# Permission denied" — 69 of 478 pushes in one run.
if [ "$(git --git-dir="$ROOT/remote.git" config core.sharedRepository 2>/dev/null)" = "0666" ]; then
    ok "remote is group-writable (concurrent pushes will not trip on permissions)"
else
    bad "remote lacks core.sharedRepository" "concurrent pushes will intermittently fail"
fi

step "the loop, with one scripted agent"

# Everything below runs inside a container as the agent user, against the real
# seeded remote — the same path a real agent takes, minus the model.
run_in_agent() {
    podman run --rm -u swarm \
        -e JJJ_USER="preflight/agent-01" -e JJJ_POD="preflight" \
        -v "$ROOT:/swarm:rw" --entrypoint /bin/bash "$IMAGE" -c "$1" 2>&1
}

out="$(run_in_agent '
set -uo pipefail
say() { printf "STEP %s %s\n" "$1" "$2"; }

git clone -q /swarm/remote.git /work/r 2>/dev/null || { say clone fail; exit 0; }
cd /work/r
git config user.name "$JJJ_USER"; git config user.email "pf@swarm.invalid"
jj git init --colocate >/dev/null 2>&1
jj config set --repo user.name "$JJJ_USER" >/dev/null 2>&1
jj config set --repo user.email "pf@swarm.invalid" >/dev/null 2>&1
say clone ok

jjj.real fetch >/dev/null 2>&1 && say fetch ok || say fetch fail

n=$(jjj.real problem list 2>/dev/null | tail -n +3 | grep -c .)
[ "${n:-0}" -gt 0 ] && say problems "$n" || say problems 0

jjj.real next --claim --json >/tmp/n.json 2>/dev/null
pid=$(python3 -c "
import json
d = json.load(open(\"/tmp/n.json\")) or {}
# next --json names the work entity_id, not id.
print(d.get(\"entity_id\") or d.get(\"id\") or \"\")" 2>/dev/null)
[ -n "$pid" ] && say claim ok || say claim fail

# A real edit, so the merge path is exercised with actual content.
printf "\n# preflight touched this\n" >> "$(ls *.py 2>/dev/null | head -1 || echo README.md)"
[ -x ./verify.sh ] && { ./verify.sh >/dev/null 2>&1 && say verify ok || say verify fail; } || say verify absent
[ -x ./score.sh ] && { s=$(./score.sh 2>/dev/null | tail -1 | cut -d" " -f1); say score "${s:-none}"; } || say score absent

git add -A >/dev/null 2>&1; git commit -q -m "preflight: a real change" >/dev/null 2>&1
ch=$(jj log -r @- --no-graph -T "change_id" 2>/dev/null | head -1)

jjj.real solution new "Preflight rehearsal solution" --problem "$pid" \
    --body "A scripted change, to prove the loop carries code end to end." \
    --force --json >/tmp/s.json 2>/dev/null
sid=$(python3 -c "import json;print(json.load(open(\"/tmp/s.json\"))[\"id\"])" 2>/dev/null)
[ -n "$sid" ] && say solution ok || { say solution fail; exit 0; }

jjj.real solution attach "$sid" >/dev/null 2>&1 && say attach ok || say attach fail
sub=$(jjj.real solution submit "$sid" 2>&1)
case "$sub" in *published*) say publish ok ;; *) say publish fail ;; esac

b=$(git ls-remote origin 2>/dev/null | grep -c "review-s-$sid")
[ "${b:-0}" -gt 0 ] && say branch ok || say branch fail

# The author must not be able to sign off their own work. Checked BEFORE anyone
# approves it: once a solution is Approved, lgtm refuses for a different reason
# and a broken self-check would look like it passed.
own=$(jjj.real solution lgtm "$sid" --approve 2>&1)
case "$own" in *"you wrote"*|*"own"*|*"your own"*) say selfblock ok ;; *) say selfblock fail ;; esac

# Then a second identity reviews and approves it.
export JJJ_USER="preflight/critic-01"
self=$(jjj.real solution lgtm "$sid" --approve --rationale "rehearsal" 2>&1)
case "$self" in *Approved*) say approve ok ;; *) say approve fail ;; esac
export JJJ_USER="preflight/agent-01"

# Push LAST, so the approval itself reaches the remote. Pushing before it means
# the integrator finds no approved work and the whole merge path goes untested.
jjj.real push >/dev/null 2>&1 && say push ok || say push fail
')"

check() { echo "$out" | grep -q "^STEP $1 $2$"; }
for s in clone fetch claim attach push; do
    check "$s" ok && ok "$s" || bad "$s" "$(echo "$out" | grep "^STEP $s " | head -1)"
done
check problems 0 && bad "problems visible" "fetch succeeded but the workbench looks empty" \
                 || ok "problems visible ($(echo "$out" | grep '^STEP problems ' | awk '{print $3}'))"
check solution ok && ok "solution created with a body" || bad "solution new"
check publish ok  && ok "submit published a reviewable branch" \
                  || bad "submit did not publish" "reviewers would have no diff to read"
check branch ok   && ok "the review branch reached the remote" || bad "review branch missing from remote"
check verify ok   && ok "verify.sh passes on the seed" \
                  || { check verify absent && ok "no verify.sh (target defines none)" || bad "verify.sh rejects its own seed"; }
check selfblock ok && ok "an author cannot sign off their own solution" || bad "self-approval was allowed"
check approve ok  && ok "a reviewer can approve" || bad "approval failed" "nothing will ever merge"

sc="$(echo "$out" | grep '^STEP score ' | awk '{print $3}')"
if [ "$sc" = "none" ] || [ "$sc" = "absent" ]; then
    bad "score.sh produced nothing"
elif [ "$sc" = "0" ]; then
    bad "score.sh returned 0 on a seeded tree" "the gate rejects its own starting point"
else
    ok "score.sh returns $sc on a real edit"
fi

step "integration"

# The step that silently did nothing for a whole run, because it ran
# `cargo build` on a Python workbench. Approved work must actually reach main.
before="$(git --git-dir="$ROOT/remote.git" rev-list --count main 2>/dev/null || echo 0)"
merge_out="$(run_in_agent '
set -uo pipefail
git clone -q /swarm/remote.git /work/m 2>/dev/null || exit 0
cd /work/m
git config user.name int; git config user.email int@swarm.invalid
jj git init --colocate >/dev/null 2>&1
export JJJ_USER=integrator
jjj.real fetch >/dev/null 2>&1
sid=$(jjj.real solution list --status approved --json 2>/dev/null \
      | python3 -c "import json,sys;d=json.load(sys.stdin);print(d[0][\"id\"] if d else \"\")" 2>/dev/null)
[ -z "$sid" ] && { echo "STEP approved none"; exit 0; }
git fetch -q origin "review-s-$sid" 2>/dev/null || { echo "STEP fetchbranch fail"; exit 0; }
git merge -q --no-edit FETCH_HEAD >/dev/null 2>&1 || { echo "STEP merge fail"; exit 0; }
if [ -x ./verify.sh ]; then ./verify.sh >/dev/null 2>&1 || { echo "STEP gate reject"; exit 0; }; fi
git push -q origin HEAD:refs/heads/main >/dev/null 2>&1 && echo "STEP merged ok" || echo "STEP merged fail"
')"
after="$(git --git-dir="$ROOT/remote.git" rev-list --count main 2>/dev/null || echo 0)"

if echo "$merge_out" | grep -q "STEP merged ok" && [ "$after" -gt "$before" ]; then
    ok "approved work merged to shared main ($before -> $after commits)"
else
    bad "approved work did not reach main" "$(echo "$merge_out" | grep '^STEP' | tail -1) — this is the failure that produced a whole run of approvals and no merges"
fi

step "metric"

# A fitness function noisier than the effect it measures makes reviewers accept
# and reject on coin flips. Wall-clock timing scored an unmodified tree
# anywhere from 0 to 28 across six concurrent agents before this was checked.
if [ -x "$ROOT/seed/score.sh" ]; then
    a=$( (cd "$ROOT/seed" && ./score.sh 2>/dev/null | tail -1 | cut -d' ' -f1) )
    for i in 1 2 3 4 5 6 7 8; do (while :; do :; done) & done
    spin="$(jobs -p)"
    sleep 2
    b=$( (cd "$ROOT/seed" && ./score.sh 2>/dev/null | tail -1 | cut -d' ' -f1) )
    kill $spin 2>/dev/null; wait 2>/dev/null
    if [ -z "$a" ] || [ -z "$b" ]; then
        bad "could not score the seed twice"
    else
        d=$(( a > b ? a - b : b - a ))
        if [ "$d" -le 5 ]; then
            ok "score is load-stable (idle $a, saturated $b)"
        else
            bad "score moves $d points under load (idle $a, saturated $b)" \
                "the metric is measuring machine contention; use CPU time, not wall-clock"
        fi
    fi
else
    ok "no score.sh at the workbench root (target scores differently)"
fi

step "result"
printf '  %d passed, %d failed\n\n' "$pass" "$fail"
[ "$fail" -eq 0 ] || { echo "  Do not start a trial until these pass — each of these"; \
                       echo "  assertions exists because a silent version of it cost hours."; exit 1; }
echo "  The loop carries work end to end. Safe to start a trial."
