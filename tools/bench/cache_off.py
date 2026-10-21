#!/usr/bin/env python3
"""Cache-on vs cache-off latency for `prime`, `query` and `next` (plan unit A2).

The question: the SQLite cache in `src/db/` is described as a disposable index
over the canonical markdown. If the markdown walk is fast enough at every
corpus size anyone will see, the cache can be deleted. This measures that.

Cache-off is taken with `JJJ_NO_DB=1` (see `crate::db::bypassed`) *and* the
`.jj/jjj.db` file removed, so the arm is a faithful picture of a world where
`src/db/` does not exist rather than a world where it merely sat idle.

The corpus is deliberately not 25,000 clones of one stub: body lengths,
tag counts, statuses, assignees and a parent/child forest are all varied from a
seeded RNG, because a flat corpus of identical short entities makes a markdown
walk look better than it is (same parse cost, perfect page locality, no deep
`parent_id` chains for the ready-front computation to walk).

Usage:
  python3 cache_off.py                            # 50/500/5000/25000, 100 reps
  python3 cache_off.py --counts 50 500 --reps 30  # a faster loop
  python3 cache_off.py --json a2-results.json

Kill criterion (plan §2 A2): cache-off `prime` p50 above 500ms at 5,000
entities blocks the deletion. The script prints the verdict.
"""
import argparse
import json
import math
import os
import random
import statistics
import subprocess
import sys
import tempfile
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO_ROOT = HERE.parent.parent
sys.path.insert(0, str(HERE))

from bench import jj_env, make_repo, sh  # noqa: E402  (shares the harness setup)

# ---------------------------------------------------------------------------
# Corpus shape
# ---------------------------------------------------------------------------

# Rough shape of a tracker that has been used: most entities are problems, a
# minority carry attempts, fewer still carry criticism.
MIX = [("problem", 0.55), ("solution", 0.25), ("critique", 0.12),
       ("finding", 0.05), ("milestone", 0.03)]

TAG_VOCAB = [
    "area:auth", "area:storage", "area:sync", "area:tui", "area:cli",
    "area:perf", "size:S", "size:M", "size:L", "size:XL", "security",
    "regression", "flaky", "needs-design", "swarm", "lane:measure",
    "lane:substrate", "lane:verbs", "p0", "tech-debt",
]
ACTORS = ["alice", "bob", "carol", "dave", "pod-01", "pod-02", "pod-03", "agent-ae5"]
PROBLEM_STATUS = ["open"] * 6 + ["in_progress"] * 3 + ["solved"] * 2 + ["dissolved"]
SOLUTION_STATUS = ["proposed"] * 4 + ["submitted"] * 3 + ["approved"] * 2 + ["withdrawn"]
CRITIQUE_STATUS = ["open"] * 4 + ["addressed"] * 2 + ["valid"] + ["dismissed"]
PRIORITY = ["low", "medium", "medium", "high", "critical"]
SEVERITY = ["low", "medium", "high", "critical"]

WORDS = (
    "the storage layer walks every entity file on a cold read which is fine at "
    "fifty and not obviously fine at twenty five thousand so the question is "
    "where the knee sits and whether a rebuildable index earns its keep against "
    "the cost it imposes on every model field that has to be mirrored through "
    "schema migration upsert and a positional decoder before it can ship"
).split()


def body_text(rng: random.Random) -> str:
    """A body whose length is drawn from a long-tailed distribution.

    Real bodies are mostly a paragraph and occasionally an essay; parse cost and
    page-cache behaviour both depend on that tail, so a fixed ~200-byte body
    would understate the walk.
    """
    # ~5% empty, median a short paragraph, tail to a few KB.
    roll = rng.random()
    if roll < 0.05:
        return ""
    n_words = int(rng.lognormvariate(math.log(60), 1.1))
    n_words = max(5, min(n_words, 900))
    out, i = [], 0
    while i < n_words:
        take = min(rng.randint(12, 28), n_words - i)
        out.append(" ".join(rng.choice(WORDS) for _ in range(take)) + ".")
        i += take
    # Fold into paragraphs, the way an authored body reads.
    paras, step = [], max(1, len(out) // max(1, rng.randint(1, 4)))
    for j in range(0, len(out), step):
        paras.append(" ".join(out[j:j + step]))
    return "\n\n".join(paras)


def tags(rng: random.Random) -> list[str]:
    # Tag count matters: `next --tag` and `query tag:` both scan them, and the
    # YAML sequence is a per-entity allocation on every parse.
    k = rng.choices([0, 1, 2, 3, 4, 5], weights=[12, 25, 28, 20, 10, 5])[0]
    return rng.sample(TAG_VOCAB, k)


def eid(kind: str, i: int) -> str:
    """A deterministic id with real UUID7 structure: 48-bit millisecond prefix,
    version nibble, then random.

    The structure is load-bearing for the measurement, not cosmetic. `prime` and
    `query` compute shortest-unique prefixes across every id they print
    (`display::truncated_prefixes`), so how early ids diverge decides how much
    work that is. An earlier version of this generator built ids from a counter
    and so shared the first eight hex digits across the whole corpus, which is
    maximally adversarial and nothing a real repo looks like. Spreading the
    timestamp prefix over a year and randomising the rest reproduces the real
    divergence: entities minutes apart share ~8 digits, months apart share ~5.
    """
    salt = {"problem": 0x1, "solution": 0x2, "critique": 0x3,
            "finding": 0x4, "milestone": 0x5}[kind]
    rng = random.Random((salt << 40) ^ (i * 0x9E3779B97F4A7C15) & 0xFFFFFFFFFFFF)
    # 2026-01-01 in epoch ms, plus up to ~a year, advancing with i so ids stay
    # time-ordered the way uuid7 is.
    base_ms = 1767225600000
    ms = base_ms + int(i * 1_200_000) + rng.randint(0, 600_000)
    hi, lo = ms >> 16, ms & 0xFFFF
    return (f"{hi:08x}-{lo:04x}-7{rng.randrange(0x1000):03x}-"
            f"{rng.randrange(0x8000, 0xC000):04x}-{rng.randrange(1 << 48):012x}")


def fm(lines: list[tuple[str, object]], seq: dict[str, list[str]], body: str) -> str:
    out = ["---"]
    for key, value in lines:
        if value is None:
            continue
        out.append(f"{key}: {value}")
    for key, values in seq.items():
        if values:
            out.append(f"{key}:")
            out.extend(f"- {v}" for v in values)
    out.append("---")
    out.append("")
    out.append(body)
    return "\n".join(out) + "\n"


def ts(rng: random.Random, base: int) -> tuple[str, str]:
    created = base + rng.randint(0, 86400 * 180)
    updated = created + rng.randint(0, 86400 * 30)
    fmt = "%Y-%m-%dT%H:%M:%SZ"
    return (time.strftime(fmt, time.gmtime(created)),
            time.strftime(fmt, time.gmtime(updated)))


def gen_corpus(meta: Path, total: int, seed: int = 20261019) -> dict[str, int]:
    """Write a mixed, nested, variable-sized corpus of `total` entities."""
    rng = random.Random(seed)
    counts = {k: max(1, int(total * w)) for k, w in MIX}
    # Put the rounding slack on problems.
    counts["problem"] += total - sum(counts.values())
    base = int(time.mktime(time.strptime("2026-01-01", "%Y-%m-%d")))

    for kind in counts:
        (meta / f"{kind}s").mkdir(parents=True, exist_ok=True)

    milestone_ids = [eid("milestone", i) for i in range(counts["milestone"])]
    for i, mid in enumerate(milestone_ids):
        c, u = ts(rng, base)
        (meta / "milestones" / f"{mid}.md").write_text(fm(
            [("id", mid), ("title", f'"Milestone {i}: {rng.choice(WORDS)} work"'),
             ("status", rng.choice(["planning", "active", "completed"])),
             ("created_at", c), ("updated_at", u),
             ("lamport", rng.randint(1, 40))],
            {"tags": tags(rng)}, body_text(rng)))

    # Problems form a forest, not a list. Each new problem either roots or hangs
    # off an existing one, with depth capped so the ready-front walk sees real
    # chains without degenerating into one 25,000-deep line.
    problem_ids: list[str] = []
    depth: dict[str, int] = {}
    for i in range(counts["problem"]):
        pid = eid("problem", i)
        parent = None
        if problem_ids and rng.random() < 0.42:
            cand = rng.choice(problem_ids[-min(len(problem_ids), 4000):])
            if depth[cand] < 4:
                parent = cand
        depth[pid] = 0 if parent is None else depth[parent] + 1
        problem_ids.append(pid)
        status = rng.choice(PROBLEM_STATUS)
        assignee = rng.choice(ACTORS) if rng.random() < 0.3 else None
        c, u = ts(rng, base)
        claimed = None
        if assignee and rng.random() < 0.5:
            claimed = u
        (meta / "problems" / f"{pid}.md").write_text(fm(
            [("id", pid),
             ("title", f'"Investigate {rng.choice(WORDS)} {rng.choice(WORDS)} '
                       f'in facet {i % 97}"'),
             ("parent_id", parent),
             ("status", status),
             ("priority", rng.choice(PRIORITY)),
             ("confidence", rng.choice([None, "green", "amber", "red"])),
             ("milestone_id", rng.choice(milestone_ids) if rng.random() < 0.35 else None),
             ("assignee", assignee),
             ("claimed_at", claimed),
             ("created_at", c), ("updated_at", u),
             ("dissolved_reason", '"superseded by a better framing"'
              if status == "dissolved" else None),
             ("lamport", rng.randint(1, 120))],
            {"tags": tags(rng)}, body_text(rng)))

    solution_ids: list[str] = []
    for i in range(counts["solution"]):
        sid = eid("solution", i)
        solution_ids.append(sid)
        c, u = ts(rng, base)
        (meta / "solutions" / f"{sid}.md").write_text(fm(
            [("id", sid),
             ("title", f'"Try {rng.choice(WORDS)} {rng.choice(WORDS)} for {i}"'),
             ("problem_id", rng.choice(problem_ids)),
             ("status", rng.choice(SOLUTION_STATUS)),
             ("assignee", rng.choice(ACTORS) if rng.random() < 0.6 else None),
             ("force_approved", "false"),
             ("created_at", c), ("updated_at", u),
             ("lamport", rng.randint(1, 120))],
            {"change_ids": [f"{rng.choice('kmnopqrstuvwxyz')}" +
                            "".join(rng.choice("klmnopqrstuvwxyz") for _ in range(31))
                            for _ in range(rng.choices([1, 1, 2, 3], weights=[6, 2, 2, 1])[0])],
             "tags": tags(rng)}, body_text(rng)))

    for i in range(counts["critique"]):
        cid = eid("critique", i)
        c, u = ts(rng, base)
        (meta / "critiques" / f"{cid}.md").write_text(fm(
            [("id", cid),
             ("title", f'"This misses {rng.choice(WORDS)} ({i})"'),
             ("solution_id", rng.choice(solution_ids)),
             ("status", rng.choice(CRITIQUE_STATUS)),
             ("severity", rng.choice(SEVERITY)),
             ("author", rng.choice(ACTORS)),
             ("created_at", c), ("updated_at", u),
             ("lamport", rng.randint(1, 120))],
            {"tags": tags(rng)}, body_text(rng)))

    for i in range(counts["finding"]):
        fid = eid("finding", i)
        c, u = ts(rng, base)
        (meta / "findings" / f"{fid}.md").write_text(fm(
            [("id", fid),
             ("title", f'"Measured {rng.choice(WORDS)} at {i}"'),
             ("problem_id", rng.choice(problem_ids)),
             ("status", rng.choice(["current"] * 4 + ["superseded"])),
             ("author", rng.choice(ACTORS)),
             ("method", f'"{rng.choice(WORDS)} harness, {rng.randint(3, 500)} reps"'),
             ("created_at", c), ("updated_at", u),
             ("lamport", rng.randint(1, 120))],
            {"tags": tags(rng)}, body_text(rng)))

    return counts


def gen_events(meta: Path, counts: dict[str, int], per_entity: float,
               shards: int = 4, seed: int = 7) -> int:
    """Per-pod event shards referencing the real corpus.

    `prime` and `next` both read the event log (escalations, solution
    authorship), and with the cache off that read is a full scan of these
    shards — so their size is part of the measurement, not setup noise.
    """
    rng = random.Random(seed)
    kinds = [(k, counts[k]) for k, _ in MIX]
    # Only real `EventType` variants: an unparseable line is dropped with a
    # warning, which would make the shards look bigger than the work they cause.
    types = {
        "problem": ["problem_created", "problem_solved", "problem_reopened",
                    "problem_dissolved", "claim_contested"],
        "solution": ["solution_created", "solution_submitted", "solution_approved",
                     "solution_withdrawn", "github_pr_created"],
        "critique": ["critique_raised", "critique_addressed", "critique_replied",
                     "critique_dismissed"],
        "finding": ["finding_recorded", "finding_superseded"],
        "milestone": ["milestone_created", "milestone_completed"],
    }
    events_dir = meta / "events"
    events_dir.mkdir(parents=True, exist_ok=True)
    buckets: list[list[str]] = [[] for _ in range(shards)]
    n = 0
    for kind, count in kinds:
        for i in range(count):
            for _ in range(max(0, int(round(per_entity)))):
                t = rng.choice(types[kind])
                when = time.strftime(
                    "%Y-%m-%dT%H:%M:%SZ",
                    time.gmtime(int(time.mktime(time.strptime("2026-01-01", "%Y-%m-%d")))
                                + rng.randint(0, 86400 * 200)))
                rec = {"when": when, "type": t, "entity": eid(kind, i),
                       "by": rng.choice(ACTORS)}
                buckets[n % shards].append(json.dumps(rec))
                n += 1

    # Escalations are deliberately a fixed handful rather than a fraction of the
    # corpus: the queue is bounded by how many things one person can be asked
    # about, not by how big the tracker got. Scaling them with the corpus put 132
    # of them in a 500-entity `prime`, which measures a listing nobody will see.
    for k in range(min(4, counts["problem"])):
        rec = {"when": "2026-09-0{}T09:00:00Z".format(k + 1),
               "type": "escalation_raised", "entity": eid("problem", k),
               "by": ACTORS[k % len(ACTORS)],
               "note": "needs a human decision on the trade-off"}
        buckets[k % shards].append(json.dumps(rec))
        n += 1

    for s, lines in enumerate(buckets):
        (events_dir / f"pod{s}.jsonl").write_text("\n".join(lines) + "\n")
    return n


# ---------------------------------------------------------------------------
# Timing
# ---------------------------------------------------------------------------

def pctl(values: list[float], q: float) -> float:
    """Nearest-rank percentile — no interpolation, so p99 of N samples is an
    observed measurement rather than a number nobody saw."""
    s = sorted(values)
    k = max(1, math.ceil(q * len(s)))
    return s[k - 1]


def time_cmd(args, cwd, env, reps: int) -> dict:
    times = []
    for _ in range(reps):
        t0 = time.perf_counter()
        r = subprocess.run(args, cwd=cwd, env=env, capture_output=True,
                           text=True, timeout=900)
        dt = time.perf_counter() - t0
        if r.returncode != 0:
            raise RuntimeError(f"{args} failed: {r.stderr[-800:]}")
        times.append(dt)
    return {
        "reps": reps,
        "p50_ms": round(statistics.median(times) * 1000, 1),
        "p99_ms": round(pctl(times, 0.99) * 1000, 1),
        "min_ms": round(min(times) * 1000, 1),
        "max_ms": round(max(times) * 1000, 1),
    }


CASES = {
    "prime": ["prime"],
    "query": ["query", "status:open sort:edit"],
    "next": ["next", "--top", "5"],
}


def main():
    ap = argparse.ArgumentParser(description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--counts", type=int, nargs="+",
                    default=[50, 500, 5000, 25000])
    ap.add_argument("--reps", type=int, default=100,
                    help="iterations per cell (p99 is nearest-rank, so this is "
                         "also the resolution of the p99)")
    ap.add_argument("--events-per-entity", type=float, default=3.0)
    ap.add_argument("--json", type=Path, default=None)
    ap.add_argument("--jjj", default=os.environ.get(
        "JJJ_BIN", str(REPO_ROOT / "target" / "release" / "jjj")))
    args = ap.parse_args()

    jjj = args.jjj
    if not Path(jjj).exists():
        sys.exit(f"jjj binary not found at {jjj} — run `cargo build --release`")

    rev = sh(["git", "rev-parse", "--short", "HEAD"], REPO_ROOT,
             dict(os.environ), check=False).stdout.strip()
    print(f"A2 cache-off bench — reps {args.reps}, rev {rev}, binary {jjj}")

    out = {"reps": args.reps, "jjj_rev": rev,
           "events_per_entity": args.events_per_entity, "cells": {}}

    for count in args.counts:
        with tempfile.TemporaryDirectory(prefix=f"jjj-a2-{count}-") as tmp:
            root = Path(tmp)
            env = jj_env(root)
            repo = make_repo(root, "repo", env, None, jjj)
            meta = repo / ".jj" / "jjj-meta"
            db_file = repo / ".jj" / "jjj.db"

            counts = gen_corpus(meta, count)
            n_events = gen_events(meta, counts, args.events_per_entity)
            bytes_on_disk = sum(f.stat().st_size for f in meta.rglob("*.md"))
            print(f"\n=== corpus {count} "
                  f"({', '.join(f'{k}:{v}' for k, v in counts.items())}; "
                  f"{n_events} events; {bytes_on_disk / 1e6:.1f} MB of markdown)")

            # --- cache off: no db file, and the bypass set so nothing can
            # lazily reintroduce one mid-measurement.
            if db_file.exists():
                db_file.unlink()
            for stray in (repo / ".jj").glob("jjj.db-*"):
                stray.unlink()
            off_env = dict(env)
            off_env["JJJ_NO_DB"] = "1"
            print("  -- cache OFF (JJJ_NO_DB=1, no .jj/jjj.db) --")
            for name, argv in CASES.items():
                r = time_cmd([jjj, *argv], repo, off_env, args.reps)
                out["cells"][f"{name}|{count}|off"] = r
                print(f"     {name:<6} p50 {r['p50_ms']:8.1f}ms   "
                      f"p99 {r['p99_ms']:8.1f}ms")
            if db_file.exists():
                print("     !! a cache-off run created .jj/jjj.db — bypass leaks")

            # --- cache on: build the index, then measure the same commands.
            t0 = time.perf_counter()
            sh([jjj, "db", "rebuild"], repo, env)
            rebuild_s = time.perf_counter() - t0
            out["cells"][f"db_rebuild|{count}|on"] = {
                "reps": 1, "p50_ms": round(rebuild_s * 1000, 1),
                "p99_ms": round(rebuild_s * 1000, 1)}
            print(f"  -- cache ON (db rebuild took {rebuild_s:.2f}s) --")
            for name, argv in CASES.items():
                r = time_cmd([jjj, *argv], repo, env, args.reps)
                out["cells"][f"{name}|{count}|on"] = r
                print(f"     {name:<6} p50 {r['p50_ms']:8.1f}ms   "
                      f"p99 {r['p99_ms']:8.1f}ms")

            out["cells"][f"corpus|{count}|meta"] = {
                "entities": sum(counts.values()), "by_kind": counts,
                "events": n_events, "markdown_bytes": bytes_on_disk}

    # --- table
    print("\n" + "=" * 78)
    print(f"{'command':<8}{'corpus':>8}  {'cache':<6}"
          f"{'p50 (ms)':>11}{'p99 (ms)':>11}   {'off/on p50':>11}")
    print("-" * 78)
    for name in CASES:
        for count in args.counts:
            on = out["cells"].get(f"{name}|{count}|on")
            off = out["cells"].get(f"{name}|{count}|off")
            if not (on and off):
                continue
            ratio = off["p50_ms"] / on["p50_ms"] if on["p50_ms"] else float("nan")
            print(f"{name:<8}{count:>8}  {'on':<6}{on['p50_ms']:>11.1f}"
                  f"{on['p99_ms']:>11.1f}")
            print(f"{'':<8}{'':>8}  {'off':<6}{off['p50_ms']:>11.1f}"
                  f"{off['p99_ms']:>11.1f}   {ratio:>10.2f}x")
    print("=" * 78)

    # --- the gate
    kill = out["cells"].get("prime|5000|off")
    if kill is None:
        print("KILL_CRITERION=not-evaluated (5000 not in --counts)")
    elif kill["p50_ms"] > 500:
        print(f"KILL_CRITERION=FIRED  cache-off prime p50 at 5,000 entities is "
              f"{kill['p50_ms']:.1f}ms > 500ms — B1 does not proceed.")
    else:
        print(f"KILL_CRITERION=not-fired  cache-off prime p50 at 5,000 entities "
              f"is {kill['p50_ms']:.1f}ms <= 500ms.")

    if args.json:
        args.json.write_text(json.dumps(out, indent=2) + "\n")
        print(f"wrote {args.json}")


if __name__ == "__main__":
    main()
