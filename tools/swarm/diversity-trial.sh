#!/usr/bin/env bash
#
# The pre-registered A/B trial from docs/design/swarm-diversity-trial.md.
#
# The argument for running several agents is not parallel effort — one agent
# with more turns does that, more cheaply. It is parallel *perspective*: several
# agents attacking from genuinely different directions should get stuck less
# often, because only one of them has to find a way through.
#
# That has never been tested here. `SWARM_STRATEGIES` exists, gives each builder
# a different prior, and has not been switched on in a single trial — so every
# run to date measured six copies of one search.
#
# Arms differ in exactly one thing:
#
#   control   one shared builder brief   (SWARM_STRATEGIES unset)
#   diverse   measure / structure / algorithm, one each   (SWARM_STRATEGIES=1)
#
# Runs are sequential on an otherwise idle machine — both arms measure a fitness
# that is counted rather than timed, but the agents themselves contend for CPU,
# and two fleets at once would make each arm's turn count a function of the
# other's.
#
# Usage:
#   diversity-trial.sh run    [--runs N] [--hours H] [--model M]
#   diversity-trial.sh report
#
# Results land in $TRIAL_ROOT (default ~/.jjj-diversity-trial), one directory
# per run, with results.tsv accumulating the pre-registered metrics.

set -uo pipefail

SWARM_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SWARM_DIR/../.." && pwd)"
TRIAL_ROOT="${TRIAL_ROOT:-$HOME/.jjj-diversity-trial}"
RESULTS="$TRIAL_ROOT/results.tsv"
JJJ_BIN="${JJJ_BIN:-$REPO_ROOT/target/release/jjj}"

info() { printf '\n=== %s\n' "$*"; }
die()  { printf 'diversity-trial: %s\n' "$*" >&2; exit 1; }

# ---------------------------------------------------------------------------

# Score the shared branch independently, rather than trusting any agent's
# report. Under the merge gate an agent's work sits on its own branch until
# approved, so an agent-reported score reads as progress before anything has
# landed — which is how a run of six private trees once passed for a healthy
# fleet with scores climbing apart from 18 to 55.
score_main() {
    local root="$1" work
    work="$(mktemp -d)"
    if ! git clone -q "$root/remote.git" "$work/repo" 2>/dev/null; then
        chmod -R u+w "$work" 2>/dev/null; rm -rf "$work"
        echo "0"; return
    fi
    local scorer="./score.py"
    [ -x "$work/repo/score.sh" ] && scorer="./score.sh"
    (cd "$work/repo" && $scorer 2>/dev/null | tail -1 | awk '{print $1}') || echo "0"
    chmod -R u+w "$work" 2>/dev/null; rm -rf "$work"
}

# Metric 3: how many of the pipeline's cost sites moved.
#
# Six copies of one search should concentrate on whatever the first agent found;
# different priors should spread. Reported as sites whose op count differs from
# the seeded baseline.
class_coverage() {
    local root="$1" work
    work="$(mktemp -d)"
    if ! git clone -q "$root/remote.git" "$work/repo" 2>/dev/null; then
        chmod -R u+w "$work" 2>/dev/null; rm -rf "$work"
        echo "0"; return
    fi
    ( cd "$work/repo/fixture" 2>/dev/null && python3 measure.py 2>/dev/null ) \
        | python3 -c "
import json, sys
try:
    d = json.load(sys.stdin)
except Exception:
    print(0); raise SystemExit
base = {}
try:
    base = json.load(open('$root/baseline_sites.json'))
except Exception:
    pass
sites = d.get('by_site', {})
if not base:
    print(len(sites)); raise SystemExit
moved = sum(1 for k, v in sites.items() if base.get(k) != v)
print(moved)
" 2>/dev/null || echo 0
    chmod -R u+w "$work" 2>/dev/null; rm -rf "$work"
}

# Metrics 2 and 4, from the fleet's own record rather than from container logs:
# the longest stretch of the trajectory with no improvement, and how many
# solutions were withdrawn as redundant with work already landed.
#
# The withdrawal classifier is the one from analyze.py, and it makes the
# distinction that a coarser one got wrong: "submitted first with an equivalent
# fix" is duplicated effort, while "theirs reaches 200,004 against my 280,004"
# is rival conjectures being selected between, which is the method working.
plateau_and_duplicates() {
    local root="$1"
    python3 - "$root" "$JJJ_BIN" <<'PY'
import json, re, subprocess, sys, tempfile, shutil, os
from pathlib import Path

root = Path(sys.argv[1]); jjj = sys.argv[2]

# Longest plateau, in samples, from the sampler's trajectory of the SHARED
# branch. An agent-local score would measure its private tree.
longest = 0
traj = root / "trajectory.tsv"
if traj.exists():
    run = 0
    best = None
    for line in traj.read_text(errors="replace").splitlines()[1:]:
        cols = line.split("\t")
        if len(cols) < 3:
            continue
        try:
            approved = int(cols[5])
        except (ValueError, IndexError):
            continue
        # "Improvement" on the shared branch means something landed. The score
        # column is an agent's own tree, which moves without anything merging.
        if best is None or approved > best:
            best = approved; run = 0
        else:
            run += 1
            longest = max(longest, run)

# Duplicate withdrawals, from the event log.
duplicates = 0
work = tempfile.mkdtemp()
try:
    repo = os.path.join(work, "repo")
    if subprocess.run(["git", "clone", "-q", str(root / "remote.git"), repo],
                      capture_output=True).returncode == 0:
        for cmd in (["jj", "git", "init", "--colocate"],
                    ["jj", "config", "set", "--repo", "user.name", "trial"],
                    ["jj", "config", "set", "--repo", "user.email", "t@invalid"]):
            subprocess.run(cmd, cwd=repo, capture_output=True)
        subprocess.run([jjj, "fetch"], cwd=repo, capture_output=True)
        out = subprocess.run([jjj, "events", "--limit", "100000", "--json"],
                             cwd=repo, capture_output=True, text=True)
        try:
            events = json.loads(out.stdout or "[]")
        except Exception:
            events = []
        for e in events:
            if e.get("type") != "solution_withdrawn":
                continue
            why = (e.get("rationale") or "").lower()
            superseded = any(k in why for k in
                             ("supersed", "duplicate", "same as", "already",
                              "redundant", "covered by", "equivalent"))
            compared = superseded and (
                re.search(r"\d[\d,]{3,}", why) is not None
                or any(k in why for k in ("better", "further", "vs ", "than mine",
                                          "outperform", "goes further")))
            if superseded and not compared:
                duplicates += 1
finally:
    shutil.rmtree(work, ignore_errors=True)

print(f"{longest}\t{duplicates}")
PY
}

# ---------------------------------------------------------------------------

one_run() {
    local arm="$1" n="$2" hours="$3" model="$4"
    local root="$TRIAL_ROOT/${arm}-${n}"
    local ns="div${arm}${n}"

    info "$arm run $n — $root"
    rm -rf "$root"; mkdir -p "$root"

    local strategies=0
    [ "$arm" = "diverse" ] && strategies=1

    # Same target, seed, duration and fleet shape in both arms. The only
    # difference is SWARM_STRATEGIES — anything else varying here would make the
    # comparison meaningless.
    SWARM_ROOT="$root" SWARM_NS="$ns" \
        "$SWARM_DIR/swarm.sh" init --target synth --pods 2 --agents 3 --critics 1 \
        || die "init failed for $arm run $n"

    # The baseline op counts, before any agent has touched anything: class
    # coverage is "how many sites moved", which needs a before.
    ( cd "$root/seed/fixture" 2>/dev/null && python3 measure.py 2>/dev/null ) \
        | python3 -c "
import json, sys
try:
    json.dump(json.load(sys.stdin).get('by_site', {}), open('$root/baseline_sites.json', 'w'))
except Exception:
    pass" 2>/dev/null

    local before
    before="$(score_main "$root")"
    echo "  baseline score on main: ${before:-0}"

    SWARM_ROOT="$root" SWARM_NS="$ns" SWARM_STRATEGIES="$strategies" \
        "$SWARM_DIR/swarm.sh" start --hours "$hours" --model "$model" --stop-when-done \
        || die "start failed for $arm run $n"

    # Wait for the fleet to finish: either the watchdog trips (STOP appears) or
    # the deadline empties the container list.
    local waited=0 limit
    limit=$(python3 -c "import sys;print(int(float(sys.argv[1])*3600)+900)" "$hours")
    while [ "$waited" -lt "$limit" ]; do
        sleep 60; waited=$((waited + 60))
        if [ -e "$root/STOP" ]; then
            echo "  watchdog stopped the fleet at ${waited}s"
            break
        fi
        if [ -z "$(podman ps -q --filter "name=^${ns}-pod-" 2>/dev/null)" ]; then
            echo "  no containers left at ${waited}s"
            break
        fi
    done

    SWARM_ROOT="$root" SWARM_NS="$ns" "$SWARM_DIR/swarm.sh" stop >/dev/null 2>&1
    sleep 20

    local after coverage pd plateau duplicates
    after="$(score_main "$root")"
    coverage="$(class_coverage "$root")"
    pd="$(plateau_and_duplicates "$root")"
    plateau="$(printf '%s' "$pd" | cut -f1)"
    duplicates="$(printf '%s' "$pd" | cut -f2)"

    printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\n' \
        "$arm" "$n" "${before:-0}" "${after:-0}" "${plateau:-0}" \
        "${coverage:-0}" "${duplicates:-0}" >> "$RESULTS"

    echo "  final=${after} plateau=${plateau} coverage=${coverage} duplicates=${duplicates}"
}

cmd_run() {
    local runs=2 hours=1 model="sonnet"
    while [ $# -gt 0 ]; do
        case "$1" in
            --runs)  runs="$2";  shift 2 ;;
            --hours) hours="$2"; shift 2 ;;
            --model) model="$2"; shift 2 ;;
            *) die "unknown option $1" ;;
        esac
    done

    [ -x "$JJJ_BIN" ] || die "jjj not found at $JJJ_BIN (cargo build --release)"
    command -v podman >/dev/null || die "podman not found"

    mkdir -p "$TRIAL_ROOT"
    [ -s "$RESULTS" ] || printf 'arm\trun\tbaseline\tfinal\tplateau\tcoverage\tduplicates\n' > "$RESULTS"

    # Interleaved, not blocked: control-1, diverse-1, control-2, diverse-2. If
    # the machine gets slower over the afternoon — a background build, thermal
    # throttling — blocking would hand the whole penalty to one arm and the
    # difference would read as an effect of the briefs.
    local n=1
    while [ "$n" -le "$runs" ]; do
        one_run control "$n" "$hours" "$model"
        one_run diverse "$n" "$hours" "$model"
        n=$((n + 1))
    done

    cmd_report
}

cmd_report() {
    [ -s "$RESULTS" ] || die "no results yet at $RESULTS"
    python3 - "$RESULTS" <<'PY'
import statistics, sys
from pathlib import Path

results = Path(sys.argv[1])
root = results.parent

def minutes(arm, run):
    """Wall-clock the arm actually got, from the sampler's own elapsed column.

    The watchdog stops a fleet once nothing is open and nothing awaits review,
    so run length is a function of the arm's behaviour rather than a constant:
    control-1 converged at 21 minutes while diverse-1 still had two solutions in
    review and ran its full hour. Any per-run figure is therefore partly a
    measure of how long that run was allowed to last, and a comparison that does
    not show this reads a difference in duration as a difference in briefs.
    """
    traj = root / f"{arm}-{run}" / "trajectory.tsv"
    if not traj.exists():
        return 0
    last = 0
    for line in traj.read_text(errors="replace").splitlines()[1:]:
        c = line.split("\t")
        if len(c) > 1:
            try:
                last = max(last, int(c[1]))
            except ValueError:
                pass
    return last

rows = []
for line in results.read_text().splitlines()[1:]:
    c = line.split("\t")
    if len(c) < 7:
        continue
    rows.append({"arm": c[0], "run": c[1], "baseline": int(c[2]), "final": int(c[3]),
                 "plateau": int(c[4]), "coverage": int(c[5]), "duplicates": int(c[6]),
                 "minutes": minutes(c[0], c[1])})

if not rows:
    print("no completed runs")
    raise SystemExit

arms = {}
for r in rows:
    arms.setdefault(r["arm"], []).append(r)

print(f"\n{'':10s} {'n':>2s} {'final':>10s} {'plateau':>9s} {'coverage':>9s} "
      f"{'dupes':>7s} {'minutes':>8s}")
print("-" * 61)
means = {}
for arm in ("control", "diverse"):
    rs = arms.get(arm, [])
    if not rs:
        continue
    m = {k: statistics.mean(r[k] for r in rs)
         for k in ("final", "plateau", "coverage", "duplicates", "minutes")}
    means[arm] = m
    print(f"{arm:10s} {len(rs):2d} {m['final']:10.1f} {m['plateau']:9.1f} "
          f"{m['coverage']:9.1f} {m['duplicates']:7.1f} {m['minutes']:8.1f}")

if len(means) < 2:
    print("\nOnly one arm has completed. Both are needed for a comparison.")
    raise SystemExit

c, d = means["control"], means["diverse"]
n = min(len(arms["control"]), len(arms["diverse"]))

print("\nAgainst the pre-registered criteria:\n")

higher = d["final"] > c["final"]
shorter = d["plateau"] < c["plateau"]
print(f"1. final score      diverse {d['final']:.1f} vs control {c['final']:.1f}"
      f"  ({'diverse higher' if higher else 'control same or higher'})")
print(f"2. longest plateau  diverse {d['plateau']:.1f} vs control {c['plateau']:.1f}"
      f"  ({'diverse shorter' if shorter else 'not shorter'})")
print(f"3. class coverage   diverse {d['coverage']:.1f} vs control {c['coverage']:.1f}")
print(f"4. duplicates       diverse {d['duplicates']:.1f} vs control {c['duplicates']:.1f}")

print()

# Saturation guard, written before any comparative data existed.
#
# Control run 1 took the synth target from its 700,004-op baseline to 60,004 —
# the scorer's declared full-marks floor — in ten minutes. When both arms finish
# at the ceiling, "diverse scored no better" is a fact about the instrument, not
# about the briefs, and printing REFUTED would be the strongest claim in this
# report resting on a measurement that could not have come out any other way.
#
# This is the harness's own fitness-function rule turned on itself: a metric with
# a reachable ceiling stops discriminating once anyone reaches it.
CEILING = 100
saturated = c["final"] >= CEILING and d["final"] >= CEILING
if saturated:
    print("INCONCLUSIVE — the instrument saturated. Both arms finished at the")
    print(f"scoring ceiling ({CEILING}), so the final-score comparison could not have")
    print("come out any other way and says nothing about the briefs.")
    print()
    print("The target is solved, not merely hard: the fitness floor is reachable, and")
    print("both arms reach it. Re-run on a target whose optimum is out of reach within")
    print("the run, or the comparison is between two numbers that were fixed in")
    print("advance. The remaining figures below still describe how each arm got")
    print("there, and are worth reading as description rather than as a verdict:")
    print()
    print(f"   plateau   diverse {d['plateau']:.1f} vs control {c['plateau']:.1f}")
    print(f"   coverage  diverse {d['coverage']:.1f} vs control {c['coverage']:.1f}")
    print(f"   dupes     diverse {d['duplicates']:.1f} vs control {c['duplicates']:.1f}")
    print(f"   minutes   diverse {d['minutes']:.0f} vs control {c['minutes']:.0f}")
    print()
    if d["minutes"] and c["minutes"] and abs(d["minutes"] - c["minutes"]) > 0.25 * c["minutes"]:
        print("   ...and the arms did not run equally long, because the watchdog stops")
        print("   a fleet once its queue empties. Coverage and duplicates accumulate")
        print("   with time, so the longer arm is flattered on both.")
        print()
    print(f"n={n} per arm.")
    raise SystemExit

# The pre-registered refutation condition, applied rather than reinterpreted.
if not higher and not shorter:
    print("REFUTED as pre-registered: diverse scores no better AND has no shorter")
    print("plateau. The briefs are decoration; the advantage is parallelism.")
elif higher and d["coverage"] < c["coverage"]:
    print("Diverse scored higher without spreading — that is not the stated")
    print("mechanism, so something other than perspective explains it.")
elif d["coverage"] > c["coverage"] and not higher:
    print("Diverse spread across more sites but ended no higher: breadth traded")
    print("for depth. A real cost, and worth saying plainly rather than")
    print("presenting breadth as a win.")
else:
    print("Consistent with the hypothesis.")

print()
if means["control"]["minutes"] and means["diverse"]["minutes"]:
    ratio = means["diverse"]["minutes"] / max(means["control"]["minutes"], 1)
    if ratio > 1.25 or ratio < 0.8:
        print("CONFOUND: the arms did not run for the same length of time")
        print(f"  control {means['control']['minutes']:.0f} min vs diverse "
              f"{means['diverse']['minutes']:.0f} min.")
        print("  The watchdog stops a fleet once nothing is open and nothing awaits")
        print("  review, so duration is an outcome of the arm rather than a constant.")
        print("  Coverage and duplicates both accumulate with time, so the longer arm")
        print("  is flattered on them. Treat any difference in those two as unproven.")
        print()

print(f"n={n} per arm. Agent runs are noisy — the same configuration twice does")
print("not produce the same number — so only a large effect means anything here,")
print("and even then it is one target's worth of evidence, not a claim about")
print("swarms in general.")
PY
}

case "${1:-}" in
    run)    shift; cmd_run "$@" ;;
    report) shift; cmd_report "$@" ;;
    *) sed -n '2,30p' "$0" | sed 's/^# \{0,1\}//' ;;
esac
