#!/usr/bin/env python3
"""Turn a swarm run into measurements.

The trial is only worth running if it produces numbers that can falsify
something. Each section below answers one question the design's locked decisions
assert but never verified:

  * Did agents actually contend? (decision 4 — claims are advisory, not locks)
  * Did the critique gate hold, or did anything approve past an objection?
  * How often did entities conflict, and did agents resolve them? (decision 10)
  * Did per-pod bookmarks keep pushes from serialising? (decision 5 / Break #5)
  * Did agents follow the skill — identity set, ids not titles, no --force?
  * Did the fitness actually climb?

Usage: analyze.py <swarm-root>
"""

import collections
import json
import subprocess
import sys
from pathlib import Path


def load(log: Path):
    records = []
    if not log.exists():
        return records
    for line in log.read_text(errors="replace").splitlines():
        try:
            records.append(json.loads(line))
        except Exception:
            continue  # a torn line is a finding, not a crash
    return records


def section(title):
    print(f"\n{title}\n{'-' * len(title)}")


def analyze_participation(records):
    section("Participation")
    by_actor = collections.Counter(r["actor"] for r in records)
    by_pod = collections.Counter(r["pod"] for r in records)
    print(f"  {len(records)} jjj invocations from {len(by_actor)} actors "
          f"across {len(by_pod)} pods")
    for actor, n in sorted(by_actor.items()):
        print(f"    {actor:24s} {n:5d}")
    unnamed = by_actor.get("", 0)
    if unnamed:
        print(f"  !! {unnamed} invocations with NO identity — agents skipped `export JJJ_USER`")


def analyze_claims(records):
    section("Claim contention (decision 4: advisory, not a lock)")
    claims = [r for r in records if r["cmd"].startswith("next") and "--claim" in r["argv"]]
    print(f"  {len(claims)} claim attempts")

    # A collision is two actors claiming the same entity. The claimed id is not
    # in the argv, so recover it from stdout.
    claimed = collections.defaultdict(set)
    for r in claims:
        out = r.get("stdout", "")
        for token in out.replace('"', " ").replace(",", " ").split():
            if len(token) == 36 and token.count("-") == 4:
                claimed[token].add(r["actor"])
                break
    collisions = {k: v for k, v in claimed.items() if len(v) > 1}
    print(f"  {len(claimed)} distinct entities claimed, {len(collisions)} claimed by >1 agent")
    for entity, actors in list(collisions.items())[:5]:
        print(f"    {entity[:8]} claimed by {', '.join(sorted(actors))}")
    if not collisions and len(claims) > 5:
        print("  (no collisions — either the work partitioned cleanly or contention was too low)")


def analyze_problem_design(records):
    """How good is the fleet at framing work, not just doing it?

    The interesting claim about a swarm is not that it can grind a fitness
    function — one agent with more turns does that — but that it can decompose a
    problem well and tell which parts matter. None of that was measured, and
    four trials turned out to use none of the machinery for it: zero rankings,
    zero subproblems, zero duplicate detection. Some of that was agents not
    trying; most of it was `jjj rank` having no way to author an ordering
    outside the TUI.
    """
    section("Problem design and ranking")

    seeded = [r for r in records if r["cmd"] == "problem new"]
    by_agent = [r for r in seeded if r["actor"] and "seed" not in r["actor"]]
    subs = [r for r in seeded if "--parent" in r["argv"]]
    print(f"  {len(seeded)} problems created, {len(by_agent)} of them by agents")
    print(f"  {len(subs)} were sub-problems (--parent)")

    ranked = [r for r in records if r["cmd"].startswith("rank set")
              or r["cmd"].startswith("rank move")]
    gaps = [r for r in ranked if "--gap" in r["argv"]]
    if ranked:
        print(f"  {len(ranked)} ranking edits, {len(gaps)} expressing a priority cliff")
        print(f"  rankers: {dict(collections.Counter(r['actor'] for r in ranked).most_common(5))}")
    else:
        print("  !! no rankings authored — nobody said which problems matter most")

    dups = [r for r in records if r["cmd"].startswith("problem duplicate")]
    diss = [r for r in records if r["cmd"].startswith("problem dissolve")]
    print(f"  {len(dups)} marked duplicate, {len(diss)} dissolved as misconceived")

    # Rival conjectures are what jjj exists to support — but a pile-up is not
    # rivalry. Counting solutions per problem cannot tell the two apart, so
    # count distinct *authors* too: one agent posting four attempts is
    # iterating, six agents posting seven is either genuine competition or an
    # uncoordinated stampede, and the difference shows in how the losers ended.
    per_problem = collections.defaultdict(list)
    for r in records:
        if r["cmd"] == "solution new" and r["exit"] == 0:
            for i, a in enumerate(r["argv"]):
                if a == "--problem" and i + 1 < len(r["argv"]):
                    per_problem[r["argv"][i + 1]].append(r["actor"])
    rivals = {k: v for k, v in per_problem.items() if len(set(v)) > 1}
    iterated = {k: v for k, v in per_problem.items()
                if len(v) > 1 and len(set(v)) == 1}
    # Distinct solutions, and successes only. Counting *calls* against
    # *successful creations* produced "33 of 24 withdrawn (137%)" — two
    # different bases and a repeated withdrawal counted twice. A ratio over 100%
    # is a measurement bug announcing itself; most are quieter than that.
    def _ids(cmd):
        out = set()
        for r in records:
            if r["cmd"] == cmd and r["exit"] == 0:
                for a in r["argv"]:
                    if len(a) > 8 and a[0].isalnum() and "-" in a:
                        out.add(a)
                        break
        return out

    withdrawn = len(_ids("solution withdraw"))
    created = sum(1 for r in records if r["cmd"] == "solution new" and r["exit"] == 0)

    if rivals:
        worst = max(rivals.items(), key=lambda kv: len(set(kv[1])))
        print(f"  {len(rivals)} problems drew solutions from more than one agent "
              f"(worst: {len(worst[1])} solutions, {len(set(worst[1]))} agents)")
        print(f"  {len(iterated)} problems saw one agent iterate")
    else:
        print("  no problem drew a solution from a second agent")

    # Why work was thrown away, which is a different question from how much.
    # Criticism eliminating a conjecture is the method working; identical effort
    # racing and losing is pure waste. One run withdrew 47 solutions, of which
    # 40 were "superseded, submitted first with an equivalent fix" and exactly
    # one was refuted.
    lost, refuted, other = 0, 0, 0
    for r in records:
        if r["cmd"] != "solution withdraw" or r["exit"] != 0:
            continue
        argv = r["argv"]
        why = ""
        for i, a in enumerate(argv):
            if a == "--rationale" and i + 1 < len(argv):
                why = argv[i + 1].lower()
        if any(k in why for k in ("supersed", "duplicate", "same as", "already",
                                  "redundant", "covered by", "equivalent")):
            lost += 1
        elif any(k in why for k in ("wrong", "incorrect", "breaks", "fails",
                                    "regress", "critique")):
            refuted += 1
        else:
            other += 1
    if lost or refuted:
        print(f"  withdrawn because: {lost} lost a race, {refuted} refuted, "
              f"{other} unexplained")
        if lost > refuted * 3:
            print("    -> the waste is duplicated effort, not criticism working")

    if created:
        pct = 100 * withdrawn // created
        note = ("— concentration, not competition: agents piled onto the same "
                "problems while others went untouched" if pct > 35 else "")
        print(f"  {withdrawn} of {created} solutions withdrawn ({pct}%) {note}")


def analyze_critique_gate(records):
    section("Critique gate")
    # Two paths reach Approved, and counting only the explicit one badly
    # understates the gate: a run with 23 approvals reported 2, because 21 came
    # through `lgtm --approve`, which is the path the agent guidance actually
    # recommends.
    lgtm_approve = [r for r in records
                    if r["cmd"] == "solution lgtm" and "--approve" in r["argv"]]
    approvals = [r for r in records if r["cmd"] == "solution approve"] + lgtm_approve
    blocked = [r for r in approvals
               if r["exit"] != 0 and "critique" in r.get("stderr", "").lower()]
    # `lgtm --approve` refuses in-band rather than by exiting non-zero, so a
    # blocked one is visible only in what it printed.
    blocked += [r for r in lgtm_approve
                if r["exit"] == 0 and "still open" in r.get("stdout", "")]
    forced = [r for r in approvals if "--force" in r["argv"]]
    landed = sum(1 for r in approvals
                 if r["exit"] == 0 and "still open" not in r.get("stdout", ""))
    print(f"  {len(approvals)} approval attempts, {landed} approved, "
          f"{len(blocked)} blocked by an open critique")
    print(f"    (via `lgtm --approve`: {len(lgtm_approve)}; "
          f"via `solution approve`: {len(approvals) - len(lgtm_approve)})")
    if forced:
        print(f"  !! {len(forced)} used --force, bypassing the gate:")
        for r in forced[:5]:
            print(f"     {r['actor']}: {' '.join(r['argv'][:4])}")
    else:
        print("  no --force bypasses — the gate held")

    critiques = [r for r in records if r["cmd"] == "critique new"]
    lgtms = [r for r in records if r["cmd"] == "solution lgtm"]
    print(f"  {len(critiques)} critiques raised, {len(lgtms)} sign-offs")

    # Self-critique would mean the swarm is not actually reviewing each other.
    print(f"  critiques by actor: "
          f"{dict(collections.Counter(r['actor'] for r in critiques).most_common(5))}")


def analyze_conflicts(records):
    section("Conflicts (decision 10: agent auto-resolves and re-pushes)")
    conflicts = [r for r in records if r["cmd"] == "conflicts"]
    resolves = [r for r in records if r["cmd"].startswith("resolve")]
    saw = [r for r in conflicts if "No unresolved" not in r.get("stdout", "")]
    print(f"  `jjj conflicts` run {len(conflicts)} times; {len(saw)} saw a real conflict")
    print(f"  `jjj resolve` run {len(resolves)} times")
    if saw and not resolves:
        print("  !! conflicts were observed but never resolved — agents abandoned them")


def analyze_sync(records):
    section("Sync (per-pod bookmarks; decision 3 wants sub-second)")
    for cmd in ("push", "fetch", "sync"):
        calls = [r for r in records if r["cmd"].split()[0] == cmd]
        if not calls:
            continue
        times = sorted(r["ms"] for r in calls)
        median = times[len(times) // 2]
        p95 = times[int(len(times) * 0.95)] if len(times) > 1 else times[0]
        failed = sum(1 for r in calls if r["exit"] != 0)
        print(f"  {cmd:6s} n={len(calls):4d}  median={median:8.0f}ms  "
              f"p95={p95:8.0f}ms  max={times[-1]:8.0f}ms  failed={failed}")
    print("  (wall-clock here is contended by design; treat as a contention signal,")
    print("   not as the sub-second measurement, which needs a quiet machine)")


def analyze_skill_adherence(records):
    section("Skill adherence")
    total = len(records)
    if not total:
        return
    json_calls = sum(1 for r in records if "--json" in r["argv"])
    print(f"  {json_calls}/{total} invocations used --json "
          f"({100 * json_calls // max(total, 1)}%)")

    # An agent following the skill passes ids. A 36-char UUID in argv is the
    # signal; a quoted phrase suggests a fuzzy title match.
    id_args = 0
    title_args = 0
    for r in records:
        for a in r["argv"][1:]:
            if len(a) == 36 and a.count("-") == 4:
                id_args += 1
            elif " " in a and not a.startswith("-"):
                title_args += 1
    print(f"  {id_args} id arguments vs {title_args} phrase arguments")
    if title_args > id_args:
        print("  !! agents mostly passed titles, not ids — the skill's rule did not stick")

    failures = collections.Counter(
        r["cmd"] for r in records if r["exit"] != 0
    )
    if failures:
        print("  failing commands:")
        for cmd, n in failures.most_common(8):
            print(f"    {n:4d}  {cmd}")


def analyze_progress(root: Path):
    section("Fitness")
    config = {}
    cfg = root / "config"
    if cfg.exists():
        for line in cfg.read_text().splitlines():
            if "=" in line:
                k, v = line.split("=", 1)
                config[k] = v
    pods = int(config.get("pods", 0))
    for p in range(1, pods + 1):
        score = root / f"pod-{p}" / "score.py"
        if score.exists():
            out = subprocess.run(
                [str(score)], cwd=str(score.parent), capture_output=True, text=True
            )
            print(f"  pod-{p}: {out.stdout.strip()} cases passing")

    # The agent logs record a score after every iteration, so the trajectory is
    # recoverable — a flat line means the swarm was busy but not productive.
    logs = sorted((root / "logs").glob("*.log")) if (root / "logs").exists() else []
    trajectory = []
    for log in logs:
        for line in log.read_text(errors="replace").splitlines():
            if "score=" in line:
                try:
                    trajectory.append(int(line.split("score=")[1].split()[0]))
                except Exception:
                    pass
    if trajectory:
        print(f"  observed scores ranged {min(trajectory)} -> {max(trajectory)} "
              f"across {len(trajectory)} iterations")


def analyze_agent_health(root: Path):
    section("Agent health")
    logdir = root / "logs"
    if not logdir.exists():
        return
    for log in sorted(logdir.glob("pod-*.log")):
        text = log.read_text(errors="replace")
        iters = text.count("iter") and len([l for l in text.splitlines() if " begin" in l])
        failed = len([l for l in text.splitlines() if "FAILED" in l])
        last = text.strip().splitlines()[-1] if text.strip() else "(empty)"
        print(f"  {log.stem:20s} {iters:4d} iters, {failed:3d} failed | {last[:60]}")


def main():
    if len(sys.argv) < 2:
        sys.exit(__doc__)
    root = Path(sys.argv[1]).resolve()
    records = load(root / "jjj-invocations.jsonl")

    print(f"Swarm trial: {root}")
    if not records:
        print("\nNo jjj invocations logged. Either the run has not started, or the")
        print("shim was not on PATH — check SWARM_LOG and tools/swarm/shim-bin.")

    analyze_participation(records)
    analyze_claims(records)
    analyze_problem_design(records)
    analyze_critique_gate(records)
    analyze_conflicts(records)
    analyze_sync(records)
    analyze_skill_adherence(records)
    analyze_progress(root)
    analyze_agent_health(root)
    print()


if __name__ == "__main__":
    main()
