#!/usr/bin/env python3
"""M0 Probe 2 — remote ref contention (validates Break #5 / Pillar 1 retry loop).

Question: when N pods push concurrently to the single `jjj` bookmark, how often
is a push rejected (non-fast-forward), how many fetch-merge-push retries does it
take to drain everyone, and what's the wall time? This sets the sync-latency
floor under contention.

This is a *ref-race* experiment at the git level (jj git push has identical
non-fast-forward semantics). It uses a local bare remote, so it isolates the
contention/serialization cost from WAN round-trip latency (which is separately
additive). jjj's own three-way content merge replaces `git rebase` here and is
cheap for a small delta, so retry *count* is the quantity of interest.

Usage: probe2_push_contention.py [N ...]   (default: 5 10 20)
"""
import os
import random
import shutil
import statistics
import subprocess
import sys
import tempfile
import threading
import time
from pathlib import Path

MAX_ATTEMPTS = 50
BRANCH = "jjj"


def git(args, cwd, check=True):
    r = subprocess.run(["git", *args], cwd=cwd, capture_output=True, text=True)
    if check and r.returncode != 0:
        raise RuntimeError(f"git {args} in {cwd} failed:\n{r.stderr}")
    return r


def setup_remote(root):
    remote = root / "remote.git"
    git(["init", "--bare", "-b", BRANCH, str(remote)], root)
    # seed an initial commit on the branch via a scratch clone
    seed = root / "seed"
    git(["clone", str(remote), str(seed)], root)
    cfg(seed)
    (seed / "events").mkdir()
    (seed / "seed.txt").write_text("seed\n")
    git(["add", "-A"], seed)
    git(["commit", "-m", "seed"], seed)
    git(["push", "origin", BRANCH], seed)
    shutil.rmtree(seed)
    return remote


def cfg(repo):
    git(["config", "user.name", "probe"], repo)
    git(["config", "user.email", "probe@example.com"], repo)


def worker(wid, remote, root, barrier, results):
    repo = root / f"w{wid}"
    git(["clone", str(remote), str(repo)], root)
    cfg(repo)
    # each worker authors a distinct event shard (single-writer file, like the design)
    shard = repo / "events" / f"agent-{wid}.jsonl"
    shard.parent.mkdir(parents=True, exist_ok=True)
    shard.write_text(f'{{"user":"agent-{wid}","n":1}}\n')
    git(["add", "-A"], repo)
    git(["commit", "-m", f"w{wid} append"], repo)

    barrier.wait()  # all workers race to push at once
    attempts = 0
    t0 = time.perf_counter()
    while attempts < MAX_ATTEMPTS:
        attempts += 1
        push = git(["push", "origin", f"HEAD:{BRANCH}"], repo, check=False)
        if push.returncode == 0:
            break
        # rejected: fetch latest and replay our commit on top (jjj would 3-way merge instead)
        git(["fetch", "origin", BRANCH], repo)
        rb = git(["rebase", f"origin/{BRANCH}"], repo, check=False)
        if rb.returncode != 0:
            git(["rebase", "--abort"], repo, check=False)
            # fall back to reset+reapply (our file is unique, so no real conflict)
            git(["reset", "--hard", f"origin/{BRANCH}"], repo)
            shard.write_text(f'{{"user":"agent-{wid}","n":1}}\n')
            git(["add", "-A"], repo)
            git(["commit", "-m", f"w{wid} append"], repo)
        time.sleep(random.uniform(0.005, 0.03) * attempts)  # backoff
    results[wid] = (attempts, time.perf_counter() - t0)


def run_case(n, root):
    case = root / f"n{n}"
    case.mkdir()
    remote = setup_remote(case)
    barrier = threading.Barrier(n)
    results = {}
    threads = [
        threading.Thread(target=worker, args=(i, remote, case, barrier, results))
        for i in range(n)
    ]
    t0 = time.perf_counter()
    for t in threads:
        t.start()
    for t in threads:
        t.join()
    wall = time.perf_counter() - t0

    # verify all N shards landed on the remote
    check = case / "verify"
    git(["clone", str(remote), str(check)], case)
    landed = len(list((check / "events").glob("agent-*.jsonl")))

    attempts = [a for a, _ in results.values()]
    return {
        "n": n,
        "wall_s": wall,
        "landed": landed,
        "max_attempts": max(attempts),
        "mean_attempts": statistics.mean(attempts),
        "first_try": sum(1 for a in attempts if a == 1),
    }


def main():
    ns = [int(x) for x in sys.argv[1:]] or [5, 10, 20]
    rows = []
    root = Path(tempfile.mkdtemp(prefix="jjj_probe2_"))
    try:
        for n in ns:
            print(f"... contention N={n}", flush=True)
            rows.append(run_case(n, root))
    finally:
        shutil.rmtree(root, ignore_errors=True)

    print("\n=== Probe 2: remote ref contention (local remote, no WAN latency) ===")
    hdr = f"{'N':>4} {'wall_s':>8} {'landed':>7} {'first_try':>10} {'mean_attempts':>14} {'max_attempts':>13}"
    print(hdr)
    print("-" * len(hdr))
    for r in rows:
        ok = "OK" if r["landed"] == r["n"] else f"LOST {r['n']-r['landed']}"
        print(
            f"{r['n']:>4} {r['wall_s']:>8.3f} {r['landed']:>7} {r['first_try']:>10} "
            f"{r['mean_attempts']:>14.2f} {r['max_attempts']:>13}   {ok}"
        )
    print("\nRead: mean_attempts ~1 => contention negligible; high max_attempts or")
    print("wall growing super-linearly in N => Break #5 needs care (backoff/jitter).")


if __name__ == "__main__":
    main()
