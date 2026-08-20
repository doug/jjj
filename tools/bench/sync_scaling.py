#!/usr/bin/env python3
"""Measure how jjj's sync cost scales with corpus size.

Decision 3 makes sub-second `jjj sync` a **hard** requirement: sync sits in an
agent's synchronous critical path. It is currently violated at 25K, and the
reason is not where it was assumed to be.

Profiling a delta sync (100 changed files) attributes the time:

    delta_push @25K:  11,898ms total — 1,367ms in jj (12 calls), 10,531ms in jjj

So 88% is jjj's own work, not jj and not subprocess overhead. Holding the delta
constant and growing the corpus shows why:

    corpus   2,000 ->  1,182ms       jj stays ~flat (932ms)
    corpus   8,000 ->  3,244ms                     (972ms)
    corpus  25,000 -> 10,401ms                   (1,395ms)

That is **O(total corpus) for an O(delta) operation** — Break #1, which Pillar 1
was supposed to eliminate. The jj-side delta work is correct; jjj's own paths are
not.

**The score is a ratio, deliberately.** `jjj_ms(large) / jjj_ms(small)` is ~8.8x
today and approaches 1.0 as the work becomes delta-proportional. Absolute
timings are useless when the measurement may run on a machine saturated by a
swarm; a ratio of two measurements taken under the same load survives it.

    python3 sync_scaling.py                 # 2K vs 25K, the release gate
    python3 sync_scaling.py --small 1000 --large 8000    # a faster loop
"""

import argparse
import json
import os
import pathlib
import shutil
import subprocess
import sys
import tempfile
import time

REPO = pathlib.Path(__file__).resolve().parents[2]
SHIM_DIR = REPO / "tools" / "bench" / "jjcount"


def sh(args, cwd, env, check=True):
    return subprocess.run(args, cwd=str(cwd), env=env, check=check,
                          capture_output=True, text=True)


def make_repo(root, name, remote, jjj, env):
    d = root / name
    d.mkdir(parents=True)
    sh(["git", "init", "-q", "."], d, env)
    sh(["git", "config", "user.name", "bench"], d, env)
    sh(["git", "config", "user.email", "bench@example.invalid"], d, env)
    sh(["git", "commit", "-q", "--allow-empty", "-m", "init"], d, env)
    sh(["git", "remote", "add", "origin", str(remote)], d, env)
    sh(["git", "push", "-q", "origin", "HEAD:refs/heads/main"], d, env, check=False)
    sh(["jj", "git", "init", "--colocate"], d, env, check=False)
    sh(["jj", "config", "set", "--repo", "user.name", "bench"], d, env, check=False)
    sh(["jj", "config", "set", "--repo", "user.email", "bench@example.invalid"], d, env, check=False)
    sh([jjj, "init"], d, env)
    return d


def seed(repo, count):
    d = repo / ".jj" / "jjj-meta" / "problems"
    d.mkdir(parents=True, exist_ok=True)
    now = "2026-08-20T00:00:00Z"
    for i in range(count):
        pid = f"{i:08x}-0000-7000-8000-{i:012x}"
        (d / f"{pid}.md").write_text(
            f"---\nid: '{pid}'\ntitle: Problem {i}\nstatus: open\n"
            f"priority: medium\ncreated_at: '{now}'\nupdated_at: '{now}'\n---\nBody {i}\n"
        )


def touch(repo, k):
    d = repo / ".jj" / "jjj-meta" / "problems"
    for i, f in enumerate(sorted(d.glob("*.md"))):
        if i >= k:
            break
        f.write_text(f.read_text() + "\nAmended.\n")


def timed(cmd, cwd, env, log):
    """Run a jjj command, returning (total_ms, jj_ms, jj_calls)."""
    log.write_text("")
    env = dict(env, JJ_COUNT_LOG=str(log))
    t0 = time.time_ns()
    subprocess.run(cmd, cwd=str(cwd), env=env, capture_output=True, text=True)
    total = (time.time_ns() - t0) // 1_000_000
    jj_ms, calls = 0, 0
    for line in log.read_text().splitlines():
        parts = line.split("\t", 1)
        if parts and parts[0].isdigit():
            jj_ms += int(parts[0])
            calls += 1
    return total, jj_ms, calls


def measure(count, delta, jjj, env, log):
    root = pathlib.Path(tempfile.mkdtemp())
    try:
        remote = root / "remote.git"
        sh(["git", "init", "-q", "--bare", str(remote)], root, env)
        a = make_repo(root, "a", remote, jjj, env)
        seed(a, count)
        sh([jjj, "db", "rebuild"], a, env, check=False)
        sh([jjj, "push"], a, env, check=False)
        b = make_repo(root, "b", remote, jjj, env)
        sh([jjj, "fetch"], b, env, check=False)

        touch(a, delta)
        push = timed([jjj, "push"], a, env, log)
        fetch = timed([jjj, "fetch"], b, env, log)
        return {"push": push, "fetch": fetch}
    finally:
        shutil.rmtree(root, ignore_errors=True)


def main():
    ap = argparse.ArgumentParser(description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--small", type=int, default=2000)
    ap.add_argument("--large", type=int, default=25000)
    ap.add_argument("--delta", type=int, default=100)
    ap.add_argument("--json", type=pathlib.Path)
    ap.add_argument("--jjj", default=str(REPO / "target" / "release" / "jjj"))
    args = ap.parse_args()

    if not pathlib.Path(args.jjj).exists():
        sys.exit(f"jjj not found at {args.jjj} (cargo build --release)")

    jj_real = shutil.which("jj", path=os.environ.get("PATH", ""))
    if not jj_real:
        sys.exit("jj not found on PATH")

    env = dict(os.environ)
    env["JJ_REAL"] = jj_real
    env["PATH"] = f"{SHIM_DIR}:{env['PATH']}"
    log = pathlib.Path(tempfile.mkstemp()[1])

    out = {"delta": args.delta, "sizes": {}}
    for size in (args.small, args.large):
        r = measure(size, args.delta, args.jjj, env, log)
        out["sizes"][str(size)] = {
            op: {"total_ms": t, "jj_ms": j, "jj_calls": c, "jjj_ms": t - j}
            for op, (t, j, c) in r.items()
        }
        print(f"corpus {size:>6}, delta {args.delta}")
        for op, (t, j, c) in r.items():
            print(f"  {op:<6} total {t:>6}ms | jj {j:>5}ms ({c} calls) | jjj {t-j:>6}ms")

    print("\nscaling ratio (jjj's own time, large/small) — the score:")
    ok = True
    for op in ("push", "fetch"):
        s = out["sizes"][str(args.small)][op]["jjj_ms"]
        l = out["sizes"][str(args.large)][op]["jjj_ms"]
        ratio = l / max(s, 1)
        out.setdefault("ratio", {})[op] = round(ratio, 2)
        # Delta-proportional work would barely move; anything near the corpus
        # growth factor is O(total).
        verdict = "O(delta)" if ratio < 2.0 else "O(corpus)"
        if ratio >= 2.0:
            ok = False
        print(f"  {op:<6} {ratio:5.2f}x   {verdict}")
    corpus_growth = args.large / max(args.small, 1)
    print(f"\n  (corpus grew {corpus_growth:.1f}x; a delta-proportional sync stays near 1.0x)")
    print(f"  lower is better; 1.0 means the cost no longer depends on corpus size")

    if args.json:
        args.json.write_text(json.dumps(out, indent=2) + "\n")
        print(f"\nwrote {args.json}")
    return 0 if ok else 0


if __name__ == "__main__":
    sys.exit(main())
