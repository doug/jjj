#!/usr/bin/env python3
"""Turn a run into measurements, reading only what jjj itself records.

Usage:
    analyze.py <swarm-root>        a trial: coordination from jjj, plus harness sections
    analyze.py <jjj-repo>          any jjj repository: coordination sections only

Why this reads jjj and not the shim
-----------------------------------

Every misreading in six trials came through a side channel — the invocation log,
container logs, an agent-local score:

  * "0 failures" reported during a 90%-failure outage, from a field that
    happened to read zero
  * three agents reported at 0 when they were at 73, from turn-*opening* scores
  * "137% of solutions withdrawn", from counting withdrawal *calls* against
    successful *creations* — two different bases
  * "40 lost a race, 1 refuted" when it was 5 and 35, from a classifier too
    coarse to tell duplication from selection on merit

So the coordination figures now come from jjj entities and the event log, which
is the same thing any participant sees. Two consequences, both wanted:

  1. It runs against a plain jjj repository. If a figure needs the shim, it is
     not a fact about the work — it is a fact about the harness, and it is
     printed in a section that says so.
  2. Where a question cannot be answered from jjj, that is a gap in jjj's model
     rather than a reason to reach for the shim. Those are listed at the end
     instead of being quietly approximated.

Every ratio states its numerator and its denominator basis. A ratio over 100% is
a measurement bug announcing itself; most are quieter than that.
"""

import collections
import json
import os
import re
import subprocess
import sys
from pathlib import Path


# --------------------------------------------------------------------------
# Reading a jjj repository
# --------------------------------------------------------------------------

class Repo:
    """A jjj repository, read only through jjj's own JSON output.

    Deliberately a subprocess boundary rather than parsing markdown: an analysis
    that reimplements jjj's reading rules can disagree with jjj, and then the
    numbers describe a repository nobody else can see.
    """

    def __init__(self, path: Path, jjj: str):
        self.path = path
        self.jjj = jjj
        self._cache = {}

    def _json(self, args, default):
        key = tuple(args)
        if key in self._cache:
            return self._cache[key]
        try:
            out = subprocess.run(
                [self.jjj, *args, "--json"],
                cwd=str(self.path), capture_output=True, text=True, timeout=120,
            )
            value = json.loads(out.stdout) if out.stdout.strip() else default
        except Exception:
            value = default
        if value is None:
            value = default
        self._cache[key] = value
        return value

    def problems(self):
        return self._json(["problem", "list"], [])

    def solutions(self):
        return self._json(["solution", "list"], [])

    def critiques(self):
        return self._json(["critique", "list"], [])

    def findings(self):
        return self._json(["finding", "list"], [])

    def escalations(self):
        return self._json(["escalate"], [])

    def events(self):
        # The default limit is 20, which would silently truncate a whole run
        # into "the last twenty things that happened".
        return self._json(["events", "--limit", "100000"], [])

    def available(self):
        return (self.path / ".jj" / "jjj-meta").exists()


def by_type(events):
    return collections.Counter(e.get("type", "?") for e in events)


def pct(n, d):
    return 0 if not d else 100 * n // d


def section(title):
    print(f"\n{title}\n{'-' * len(title)}")


GAPS = []


def gap(question, why):
    """Record something jjj cannot currently answer.

    Printed at the end rather than approximated from the shim. Each of these is
    a candidate change to the model, which is more useful than a number derived
    from a channel only the harness can see.
    """
    GAPS.append((question, why))


# --------------------------------------------------------------------------
# Coordination — derived from jjj, works on any jjj repository
# --------------------------------------------------------------------------

def analyze_participation(repo):
    section("Participation (from the event log)")
    events = repo.events()
    if not events:
        print("  no events recorded")
        return
    actors = collections.Counter(e.get("by", "") or "(unattributed)" for e in events)
    print(f"  {len(events)} events from {len(actors)} actors")
    for actor, n in actors.most_common(12):
        print(f"    {actor:24s} {n:5d}")
    unnamed = actors.get("(unattributed)", 0)
    if unnamed:
        print(f"  !! {unnamed} events with no actor — agents skipped `export JJJ_USER`")

    kinds = by_type(events)
    print("  by kind: " + ", ".join(f"{k}={n}" for k, n in kinds.most_common(8)))

    gap("Which agents contended for the same claim",
        "a claim is last-writer state, not an event; two agents claiming the "
        "same problem leaves one assignee and no record of the other")


def analyze_problem_design(repo):
    """How good is the fleet at framing work, not just doing it?

    The interesting claim about a swarm is not that it can grind a fitness
    function — one agent with more turns does that — but that it can decompose a
    problem well and tell which parts matter.
    """
    section("Problem design and ranking")
    problems = repo.problems()
    events = repo.events()

    created = [e for e in events if e.get("type") == "problem_created"]
    seeders = {e.get("by") for e in created if "seed" in (e.get("by") or "")}
    by_agent = [e for e in created if e.get("by") not in seeders]
    subs = [p for p in problems if p.get("parent_id")]

    print(f"  {len(problems)} problems exist; {len(created)} problem_created events, "
          f"{len(by_agent)} of them by non-seed actors")
    print(f"  {len(subs)} of {len(problems)} problems are sub-problems "
          f"({pct(len(subs), len(problems))}% — denominator: all problems)")

    dissolved = [p for p in problems if p.get("status") == "dissolved"]
    print(f"  {len(dissolved)} dissolved as misconceived or duplicate")
    for p in dissolved[:3]:
        why = (p.get("dissolved_reason") or "").strip().replace("\n", " ")
        print(f"    {p['id'][:8]} {why[:70]}")

    # Ranking lives in per-user files rather than entities, so read it per
    # milestone through the command that computes it.
    milestones = {p.get("milestone_id") for p in problems if p.get("milestone_id")}
    ranked_any = False
    for m in sorted(filter(None, milestones)):
        rows = repo._json(["rank", "show", m], [])
        if not isinstance(rows, list):
            rows = rows.get("ranking", [])
        # `voters` is a count per row, not a list: the number of actors whose
        # ordering placed that problem. The maximum across rows is how many
        # people ranked anything in this milestone at all.
        voters = max((int(r.get("voters", 0) or 0) for r in rows), default=0)
        if rows and voters:
            ranked_any = True
            print(f"  milestone {m[:8]}: {len(rows)} problems ranked by "
                  f"up to {voters} actor{'' if voters == 1 else 's'}")
    if not ranked_any:
        print("  !! no rankings authored — nobody said which problems matter most")

    if not milestones:
        print("  !! no problem belongs to a milestone, so ranking has nowhere to live")


def analyze_rivalry(repo):
    """Rival conjectures are what jjj exists for. A pile-up is not rivalry."""
    section("Rivalry and discarded work")
    solutions = repo.solutions()
    events = repo.events()

    per_problem = collections.defaultdict(set)
    authors = {}
    for e in events:
        if e.get("type") == "solution_created":
            problem = (e.get("problem") or "")
            if problem:
                per_problem[problem].add(e.get("by", ""))
            authors[e.get("entity")] = e.get("by", "")

    rivals = {k: v for k, v in per_problem.items() if len(v) > 1}
    print(f"  {len(solutions)} solutions across {len(per_problem)} "
          f"problem{'' if len(per_problem) == 1 else 's'}")
    if rivals:
        worst = max(rivals.items(), key=lambda kv: len(kv[1]))
        print(f"  {len(rivals)} problem{'' if len(rivals) == 1 else 's'} drew "
              f"solutions from more than one actor "
              f"(worst: {len(worst[1])} distinct actors on {worst[0][:8]})")
    else:
        print("  no problem drew a solution from a second actor")

    # Withdrawal reasons, from the event rationale rather than the argv that
    # produced it. Distinct solutions, not calls: counting withdrawal *calls*
    # against successful *creations* once produced "33 of 24 withdrawn (137%)".
    withdrawn_ids = {e.get("entity") for e in events
                     if e.get("type") == "solution_withdrawn"}
    reasons = {}
    for e in events:
        if e.get("type") == "solution_withdrawn":
            reasons[e.get("entity")] = (e.get("rationale") or "").lower()

    duplicated, on_merits, refuted, unexplained = 0, 0, 0, 0
    for why in reasons.values():
        superseded = any(k in why for k in ("supersed", "duplicate", "same as",
                                            "already", "redundant", "covered by",
                                            "equivalent"))
        # Two different reasons hide under "superseded" and only one is waste.
        # "Submitted first with an equivalent fix" is duplicated effort. "Theirs
        # reaches 200,004 ops against my 280,004" is the method working. A
        # rationale citing competing numbers is comparing, not conceding a race.
        compared = superseded and (
            re.search(r"\d[\d,]{3,}", why) is not None
            or any(k in why for k in ("better", "further", "vs ", "than mine",
                                      "outperform", "goes further"))
        )
        if compared:
            on_merits += 1
        elif superseded:
            duplicated += 1
        elif any(k in why for k in ("wrong", "incorrect", "breaks", "fails",
                                    "regress", "critique")):
            refuted += 1
        else:
            unexplained += 1

    created_ids = {e.get("entity") for e in events
                   if e.get("type") == "solution_created"}
    basis = len(created_ids) or len(solutions)
    if withdrawn_ids:
        print(f"  {len(withdrawn_ids)} distinct solutions withdrawn of {basis} created "
              f"({pct(len(withdrawn_ids), basis)}% — numerator: distinct solution ids "
              f"with a solution_withdrawn event; denominator: distinct solution_created events)")
        print(f"  withdrawn because: {duplicated} duplicated another, "
              f"{on_merits} lost on the merits, {refuted} refuted, "
              f"{unexplained} unexplained")
        if duplicated > (on_merits + refuted):
            print("    -> the waste is duplicated effort, not selection working")
        elif on_merits + refuted:
            print("    -> most discarded work lost to a better idea or an objection, "
                  "which is the method")
    else:
        print("  nothing withdrawn")


def analyze_critique_gate(repo):
    section("Critique gate")
    solutions = repo.solutions()
    critiques = repo.critiques()
    events = repo.events()

    approved = [s for s in solutions if s.get("status") == "approved"]
    forced = [s for s in approved if s.get("force_approved")]
    by_status = collections.Counter(c.get("status", "?") for c in critiques)

    print(f"  {len(critiques)} critiques by status: " +
          ", ".join(f"{k}={n}" for k, n in sorted(by_status.items())))
    print(f"  {len(approved)} of {len(solutions)} solutions approved "
          f"({pct(len(approved), len(solutions))}% — denominator: all solutions, "
          f"including proposed and withdrawn)")

    # The gate's whole claim, checked against final state rather than against
    # what a command printed: no approved solution may carry an unresolved
    # objection.
    open_by_solution = collections.defaultdict(list)
    for c in critiques:
        if c.get("status") in ("open", "valid"):
            open_by_solution[c.get("solution_id")].append(c)
    breaches = [s for s in approved if open_by_solution.get(s["id"])]
    if breaches:
        print(f"  !! {len(breaches)} approved solutions still carry an open or "
              f"upheld critique — the gate did not hold")
        for s in breaches[:5]:
            print(f"     {s['id'][:8]} {s.get('title', '')[:50]}")
    else:
        print("  no approved solution carries an unresolved critique — the gate held")

    if forced:
        print(f"  !! {len(forced)} approved with --force, bypassing the gate")
    else:
        print("  no --force bypasses")

    # Self-critique means the fleet is not actually reviewing each other.
    authors = {}
    for e in events:
        if e.get("type") == "solution_created":
            authors[e.get("entity")] = e.get("by")
    self_critiques = [c for c in critiques
                      if c.get("author") and authors.get(c.get("solution_id")) == c.get("author")]
    if critiques:
        print(f"  {len(self_critiques)} of {len(critiques)} critiques were on the "
              f"author's own solution ({pct(len(self_critiques), len(critiques))}%)")

    gap("Whether an approval was ever attempted and refused",
        "a blocked approval changes nothing, so it leaves no entity and no "
        "event — only the command's exit code, which only the caller sees")


def analyze_evidence(repo):
    """Did the fleet record what it measured, and did anything use it?

    M1's success criterion. A finding nobody cites is a filing cabinet.
    """
    section("Evidence")
    findings = repo.findings()
    solutions = repo.solutions()
    critiques = repo.critiques()

    if not findings:
        print("  no findings recorded")
        print("  (if investigations are being filed as solutions and then withdrawn,")
        print("   that is the gap `jjj finding` exists to close)")
        return

    current = [f for f in findings if f.get("status") == "current"]
    superseded = [f for f in findings if f.get("status") == "superseded"]
    authors = collections.Counter(f.get("author") or "(none)" for f in findings)
    with_method = [f for f in findings if (f.get("method") or "").strip()]

    print(f"  {len(findings)} findings: {len(current)} current, "
          f"{len(superseded)} superseded by a better measurement")
    print(f"  {len(with_method)} of {len(findings)} say how they were measured "
          f"({pct(len(with_method), len(findings))}%) — a number nobody can "
          f"reproduce is a rumour")
    print(f"  recorded by: " + ", ".join(f"{a}={n}" for a, n in authors.most_common(6)))

    cited = collections.Counter()
    for s in solutions:
        for fid in s.get("cites", []) or []:
            cited[fid] += 1
    for c in critiques:
        for fid in c.get("cites", []) or []:
            cited[fid] += 1

    used = len(cited)
    citing_solutions = sum(1 for s in solutions if s.get("cites"))
    print(f"  {used} of {len(findings)} findings are cited by later work "
          f"({pct(used, len(findings))}% — denominator: all findings, current and "
          f"superseded)")
    print(f"  {citing_solutions} of {len(solutions)} solutions cite evidence "
          f"({pct(citing_solutions, len(solutions))}%)")
    if used == 0:
        print("  !! nothing cites a finding — evidence is being recorded and ignored")


def analyze_escalations(repo):
    section("Escalations")
    events = repo.events()
    raised = [e for e in events if e.get("type") == "escalation_raised"]
    cleared = {e.get("entity") for e in events
               if e.get("type") == "escalation_cleared"}
    open_now = repo.escalations()

    if not raised:
        print("  none raised — the fleet was never blocked on a person, "
              "or could not say so")
        return

    print(f"  {len(raised)} raised, {len(cleared)} cleared, {len(open_now)} still open")
    for e in raised[:8]:
        state = "cleared" if e.get("entity") in cleared else "OPEN"
        why = (e.get("rationale") or "").replace("\n", " ")[:60]
        print(f"    [{state:7s}] {e.get('by', '?'):16s} {why}")

    if len(raised) > 3 * max(len(cleared), 1):
        print("  !! far more raised than cleared — either nobody is watching, or "
              "the blacklist is not holding and routine things are being escalated")


def analyze_conflicts(repo):
    section("Conflicts")
    events = repo.events()
    resolved = [e for e in events if e.get("type") == "conflict_resolved"]
    print(f"  {len(resolved)} conflicts resolved and recorded")
    gap("How many conflicts occurred but were abandoned",
        "an unresolved conflict is a file state, not an event; only resolution "
        "is recorded, so the denominator is invisible")


# --------------------------------------------------------------------------
# Harness — NOT derived from jjj. Facts about the rig, not about the work.
# --------------------------------------------------------------------------

def load_shim(log: Path):
    records = []
    if not log.exists():
        return records
    for line in log.read_text(errors="replace").splitlines():
        try:
            records.append(json.loads(line))
        except Exception:
            continue  # a torn line is a finding, not a crash
    return records


def analyze_harness_latency(records):
    section("Harness: command latency (invocation shim, not jjj)")
    if not records:
        print("  no invocations logged")
        return
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
    print("  (wall-clock here is contended by design; a contention signal, not")
    print("   the sub-second measurement, which needs a quiet machine)")


def analyze_harness_adherence(records):
    section("Harness: skill adherence (invocation shim, not jjj)")
    total = len(records)
    if not total:
        print("  no invocations logged")
        return
    json_calls = sum(1 for r in records if "--json" in r["argv"])
    print(f"  {json_calls} of {total} invocations used --json ({pct(json_calls, total)}%)")

    id_args = title_args = 0
    for r in records:
        for a in r["argv"][1:]:
            if len(a) == 36 and a.count("-") == 4:
                id_args += 1
            elif " " in a and not a.startswith("-"):
                title_args += 1
    print(f"  {id_args} id arguments vs {title_args} phrase arguments")
    if title_args > id_args:
        print("  !! agents mostly passed titles, not ids — the skill's rule did not stick")

    failures = collections.Counter(r["cmd"] for r in records if r["exit"] != 0)
    if failures:
        print("  failing commands:")
        for cmd, n in failures.most_common(8):
            print(f"    {n:4d}  {cmd}")


def analyze_harness_fitness(root: Path):
    section("Harness: fitness (container logs)")
    logs = sorted((root / "logs").glob("*.log")) if (root / "logs").exists() else []
    trajectory = []
    for log in logs:
        for line in log.read_text(errors="replace").splitlines():
            # End-of-turn scores only. Reading turn-*opening* scores reported
            # three of six agents at 0 while they were in fact at 73: a turn can
            # legitimately open at zero after a merge and be fixed within it.
            if "end score=" in line:
                try:
                    trajectory.append(int(line.split("end score=")[1].split()[0]))
                except Exception:
                    pass
    if trajectory:
        print(f"  end-of-turn scores ranged {min(trajectory)} -> {max(trajectory)} "
              f"across {len(trajectory)} completed turns")
    else:
        print("  no end-of-turn scores in the logs")


def analyze_harness_health(root: Path):
    section("Harness: agent health (container logs)")
    logdir = root / "logs"
    if not logdir.exists():
        print("  no logs directory")
        return
    for log in sorted(logdir.glob("pod-*.log")):
        text = log.read_text(errors="replace")
        lines = text.splitlines()
        iters = len([l for l in lines if " begin" in l])
        failed = len([l for l in lines if "FAILED" in l])
        last = lines[-1] if lines else "(empty)"
        print(f"  {log.stem:20s} {iters:4d} iters, {failed:3d} failed | {last[:60]}")


# --------------------------------------------------------------------------

def clone_remote(root: Path, jjj: str):
    """Materialise the fleet's shared metadata as a readable jjj repo.

    The bare remote is the fleet's shared truth; a supervisor reading it sees
    exactly what an agent sees after a fetch, which is the point of M3.
    """
    import shutil
    import tempfile

    remote = root / "remote.git"
    if not remote.exists():
        return None
    work = Path(tempfile.mkdtemp(prefix="jjj-analyze-"))
    repo = work / "repo"
    try:
        subprocess.run(["git", "clone", "-q", str(remote), str(repo)],
                       check=True, capture_output=True, timeout=300)
        for cmd in (["jj", "git", "init", "--colocate"],
                    ["jj", "config", "set", "--repo", "user.name", "analyze"],
                    ["jj", "config", "set", "--repo", "user.email", "analyze@invalid"]):
            subprocess.run(cmd, cwd=str(repo), capture_output=True, timeout=120)
        subprocess.run([jjj, "fetch"], cwd=str(repo), capture_output=True, timeout=300)
        return repo
    except Exception:
        shutil.rmtree(work, ignore_errors=True)
        return None


def main():
    if len(sys.argv) < 2:
        sys.exit(__doc__)
    root = Path(sys.argv[1]).resolve()
    jjj = os.environ.get("JJJ_BIN") or str(
        Path(__file__).resolve().parents[2] / "target" / "release" / "jjj"
    )
    if not Path(jjj).exists():
        jjj = "jjj"

    is_swarm = (root / "config").exists() or (root / "remote.git").exists()
    print(f"{'Swarm trial' if is_swarm else 'jjj repository'}: {root}")

    if is_swarm:
        repo_path = clone_remote(root, jjj)
        if repo_path is None:
            print("\n!! could not read the shared metadata from remote.git")
            print("   coordination sections need it; harness sections follow anyway")
    else:
        repo_path = root

    if repo_path is not None:
        repo = Repo(repo_path, jjj)
        if not repo.available():
            print(f"\n!! {repo_path} is not a jjj repository (no .jj/jjj-meta)")
        else:
            analyze_participation(repo)
            analyze_problem_design(repo)
            analyze_rivalry(repo)
            analyze_critique_gate(repo)
            analyze_evidence(repo)
            analyze_escalations(repo)
            analyze_conflicts(repo)

    if is_swarm:
        records = load_shim(root / "jjj-invocations.jsonl")
        analyze_harness_latency(records)
        analyze_harness_adherence(records)
        analyze_harness_fitness(root)
        analyze_harness_health(root)

    if GAPS:
        section("Not answerable from jjj")
        print("  Each of these is a candidate change to the model, not a reason to")
        print("  reach for the invocation log.\n")
        for question, why in GAPS:
            print(f"  * {question}")
            print(f"    {why}")

    print()


if __name__ == "__main__":
    main()
