#!/usr/bin/env python3
"""Repeatable end-to-end benchmark harness for jjj (the release gate).

Where the M0 probes measured raw jj primitives to validate the design, this
harness times real jjj commands against a generated corpus so regressions in
the shipped read/write/sync paths show up before a release. Run it on a quiet
machine and compare against the last recorded numbers (see README).

Covers the bench matrix from docs/design/scaling-for-agent-swarms.md:
  - cold list (no DB, FS walk)         - db rebuild
  - warm list / status / next          - FTS search
  - events listing (cold ingest + warm DB-primary)
  - write throughput under concurrent per-pod writers
  - sync: cold push, cold fetch, warm delta-fetch (100-file delta)

Usage:
  python3 bench.py                       # quick run (2K corpus, with sync)
  python3 bench.py --count 25000         # release-gate scale
  python3 bench.py --skip-sync          # skip the remote/push/fetch cases
  python3 bench.py --json results.json  # machine-readable output

The jjj binary defaults to ../../target/release/jjj (build with
`cargo build --release`); override with --jjj or $JJJ_BIN.
"""
import argparse
import json
import os
import statistics
import subprocess
import sys
import tempfile
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO_ROOT = HERE.parent.parent

BODY = (
    "This problem concerns a sub-component of the larger investigation. "
    "It requires careful analysis of the available evidence and a conjecture "
    "that can be subjected to criticism. Linked changes hold the artifacts."
)


def sh(args, cwd, env, check=True, timeout=600):
    r = subprocess.run(
        args, cwd=cwd, env=env, capture_output=True, text=True, timeout=timeout
    )
    if check and r.returncode != 0:
        raise RuntimeError(
            f"cmd {args} failed in {cwd}:\nstdout: {r.stdout}\nstderr: {r.stderr}"
        )
    return r


def timed(args, cwd, env):
    t0 = time.perf_counter()
    sh(args, cwd, env)
    return time.perf_counter() - t0


def jj_env(home: Path):
    """Isolated jj/git identity so runs don't touch or depend on user config."""
    env = dict(os.environ)
    env["HOME"] = str(home)
    env["JJ_CONFIG"] = str(home / "jjconfig.toml")
    (home / "jjconfig.toml").write_text(
        '[user]\nname = "bench"\nemail = "bench@example.com"\n'
    )
    # git for the bare remote; identity comes from HOME/.gitconfig
    (home / ".gitconfig").write_text(
        "[user]\n\tname = bench\n\temail = bench@example.com\n"
    )
    return env


def problem_id(i: int) -> str:
    return f"0195{i:08x}-{i % 0xFFFF:04x}-7def-8c3a-{i:012x}"


def gen_problems(meta_dir: Path, count: int, start: int = 0):
    """Write `count` problem entity files straight into the meta checkout —
    the same shape `jjj problem new` writes, minus one subprocess per entity."""
    problems = meta_dir / "problems"
    problems.mkdir(parents=True, exist_ok=True)
    for i in range(start, start + count):
        pid = problem_id(i)
        (problems / f"{pid}.md").write_text(
            f"---\n"
            f"id: {pid}\n"
            f'title: "Investigate facet {i} of the problem space"\n'
            f"status: open\n"
            f"priority: medium\n"
            f"created_at: 2026-06-19T12:00:00Z\n"
            f"updated_at: 2026-06-19T12:00:00Z\n"
            f"tags:\n- area:facet{i % 50}\n- size:M\n"
            f"---\n\n{BODY}\n"
        )


def gen_event_shards(meta_dir: Path, count: int, shards: int = 4):
    """Synthetic per-pod event shards referencing real corpus problems."""
    events_dir = meta_dir / "events"
    events_dir.mkdir(parents=True, exist_ok=True)
    for s in range(shards):
        lines = []
        for i in range(s, count, shards):
            lines.append(
                json.dumps(
                    {
                        "when": f"2026-06-19T12:{(i // 60) % 60:02d}:{i % 60:02d}Z",
                        "type": "problem_created",
                        "entity": problem_id(i),
                        "by": f"pod{s}",
                    }
                )
            )
        (events_dir / f"pod{s}.jsonl").write_text("\n".join(lines) + "\n")


def make_repo(root: Path, name: str, env, remote: Path | None, jjj: str) -> Path:
    repo = root / name
    repo.mkdir()
    sh(["jj", "git", "init", "--colocate"], repo, env)
    if remote is not None:
        sh(
            ["jj", "git", "remote", "add", "origin", f"file://{remote}"],
            repo,
            env,
        )
    sh([jjj, "init"], repo, env)
    return repo


class Bench:
    def __init__(self, reps: int):
        self.reps = reps
        self.results = {}

    def run(self, name: str, args, cwd, env, reps=None, setup=None):
        times = []
        for _ in range(reps or self.reps):
            if setup:
                setup()
            times.append(timed(args, cwd, env))
        med = statistics.median(times)
        self.results[name] = {
            "median_s": round(med, 4),
            "min_s": round(min(times), 4),
            "runs": len(times),
        }
        print(f"  {name:<28} median {med:8.3f}s   min {min(times):8.3f}s")
        return med

    def record(self, name: str, seconds: float, **extra):
        self.results[name] = {"median_s": round(seconds, 4), "runs": 1, **extra}
        print(f"  {name:<28} {'':>7} {seconds:8.3f}s")


def compare_to_baseline(out, baseline_path, tolerance):
    """Report benches slower than `tolerance` x their recorded median.

    Returns True if everything is within tolerance. A bench missing from the
    baseline is reported and skipped rather than failing: adding a bench should
    not break the build before anyone has recorded a number for it.
    """
    try:
        baseline = json.loads(Path(baseline_path).read_text())
    except (OSError, json.JSONDecodeError) as e:
        print(f"!! cannot read baseline {baseline_path}: {e}")
        return False

    if baseline.get("corpus") != out.get("corpus"):
        print(
            f"!! baseline corpus {baseline.get('corpus')} != this run "
            f"{out.get('corpus')} — timings are not comparable"
        )
        return False

    base_results = baseline.get("results", {})
    regressions, missing = [], []

    print(f"\n— comparing against {baseline_path} (tolerance {tolerance}x) —")
    for name, result in out["results"].items():
        if name not in base_results:
            missing.append(name)
            continue
        got = result.get("median_s")
        want = base_results[name].get("median_s")
        if got is None or not want:
            continue
        ratio = got / want
        flag = "REGRESSION" if ratio > tolerance else "ok"
        print(f"  {name:44s} {got:7.3f}s vs {want:7.3f}s  ({ratio:4.2f}x) {flag}")
        if ratio > tolerance:
            regressions.append((name, got, want, ratio))

    for name in missing:
        print(f"  {name:44s} (not in baseline — skipped)")

    if regressions:
        print(f"\n!! {len(regressions)} bench(es) regressed beyond {tolerance}x:")
        for name, got, want, ratio in regressions:
            print(f"   {name}: {got:.3f}s vs {want:.3f}s ({ratio:.2f}x)")
        return False

    print("\nno regressions beyond tolerance.")
    return True


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--count", type=int, default=2000, help="corpus size (default 2000; release gate 25000)")
    ap.add_argument("--reps", type=int, default=3)
    ap.add_argument("--writers", type=int, default=10, help="concurrent writer pods")
    ap.add_argument("--writes-per-writer", type=int, default=5)
    ap.add_argument("--delta", type=int, default=100, help="files modified for warm delta-fetch")
    ap.add_argument("--skip-sync", action="store_true")
    ap.add_argument("--json", type=Path, default=None)
    ap.add_argument("--jjj", default=os.environ.get("JJJ_BIN", str(REPO_ROOT / "target" / "release" / "jjj")))
    ap.add_argument(
        "--check-against",
        type=Path,
        default=None,
        help="compare medians against a recorded baseline JSON and exit non-zero "
             "if any bench is slower than --tolerance x its baseline",
    )
    ap.add_argument(
        "--tolerance",
        type=float,
        default=3.0,
        help="allowed slowdown factor for --check-against (default 3.0). Shared "
             "CI runners are noisy, so this catches algorithmic regressions, not drift",
    )
    args = ap.parse_args()

    jjj = args.jjj
    if not Path(jjj).exists():
        sys.exit(f"jjj binary not found at {jjj} — run `cargo build --release` (or set --jjj/$JJJ_BIN)")

    bench = Bench(args.reps)
    print(f"jjj bench — corpus {args.count}, reps {args.reps}, binary {jjj}")

    with tempfile.TemporaryDirectory(prefix="jjj-bench-") as tmp:
        root = Path(tmp)
        env = jj_env(root)

        remote = None
        if not args.skip_sync:
            remote = root / "remote.git"
            remote.mkdir()
            sh(["git", "init", "--bare"], remote, env)

        repo = make_repo(root, "repo_a", env, remote, jjj)
        meta = repo / ".jj" / "jjj-meta"

        print(f"generating corpus ({args.count} problems)…")
        gen_problems(meta, args.count)
        gen_event_shards(meta, args.count)

        # jjj init created an empty (clean) DB before the corpus existed; the
        # corpus was written behind its back, so drop it. Cold reads measure
        # the FS walk; db rebuild then builds the cache for the warm reads.
        db_file = repo / ".jj" / "jjj.db"

        print("— reads —")
        bench.run(
            "cold_list (no DB, FS walk)",
            [jjj, "problem", "list"],
            repo, env,
            setup=lambda: db_file.exists() and db_file.unlink(),
        )
        bench.record("db_rebuild", timed([jjj, "db", "rebuild"], repo, env))
        bench.run("warm_list (DB)", [jjj, "problem", "list"], repo, env)
        bench.run("status", [jjj, "status"], repo, env)
        bench.run("next_top5", [jjj, "next", "--top", "5"], repo, env)
        bench.run("search_fts", [jjj, "search", "facet"], repo, env)

        # events: first listing after rebuild ingests nothing new (rebuild
        # fast-forwards offsets); measure a genuinely cold ingest by wiping
        # the offset index, then the warm DB-primary path.
        offsets = meta / ".events_offsets.json"
        if offsets.exists():
            offsets.unlink()
        bench.record("events_cold_ingest", timed([jjj, "events", "--limit", "50"], repo, env))
        bench.run("events_warm (DB-primary)", [jjj, "events", "--limit", "50"], repo, env)

        print("— writes —")
        t0 = time.perf_counter()
        procs = []
        for w in range(args.writers):
            wenv = dict(env)
            wenv["JJJ_POD"] = f"bench-pod-{w}"
            for j in range(args.writes_per_writer):
                procs.append(
                    subprocess.Popen(
                        [jjj, "problem", "new", f"Concurrent write {w}-{j}"],
                        cwd=repo, env=wenv,
                        stdout=subprocess.DEVNULL, stderr=subprocess.PIPE, text=True,
                    )
                )
        failures = 0
        for p in procs:
            _, err = p.communicate(timeout=600)
            if p.returncode != 0:
                failures += 1
                print(f"    writer failed: {err.strip().splitlines()[-1] if err.strip() else '?'}")
        wall = time.perf_counter() - t0
        ops = len(procs) - failures
        bench.record(
            f"write_throughput ({args.writers}x{args.writes_per_writer})",
            wall, ops_per_s=round(ops / wall, 2), failures=failures,
        )
        if failures:
            print(f"    WARNING: {failures}/{len(procs)} concurrent writes failed")

        if not args.skip_sync:
            print("— sync —")
            bench.record("cold_push (full corpus)", timed([jjj, "push"], repo, env))

            repo_b = make_repo(root, "repo_b", env, remote, jjj)
            bench.record("cold_fetch (full corpus)", timed([jjj, "fetch"], repo_b, env))

            # warm delta: modify `--delta` entities in A, push, fetch in B
            for i in range(min(args.delta, args.count)):
                p = meta / "problems" / f"{problem_id(i)}.md"
                p.write_text(p.read_text() + "\nAmended with new evidence.\n")
            bench.record(f"delta_push ({args.delta} files)", timed([jjj, "push"], repo, env))
            bench.record(f"warm_delta_fetch ({args.delta} files)", timed([jjj, "fetch"], repo_b, env))

        out = {
            "corpus": args.count,
            "reps": args.reps,
            "jjj_rev": sh(["git", "rev-parse", "--short", "HEAD"], REPO_ROOT, dict(os.environ), check=False).stdout.strip(),
            "results": bench.results,
        }
        if args.json:
            args.json.write_text(json.dumps(out, indent=2) + "\n")
            print(f"wrote {args.json}")

        if args.check_against:
            if not compare_to_baseline(out, args.check_against, args.tolerance):
                sys.exit(1)

    print("done.")


if __name__ == "__main__":
    main()
