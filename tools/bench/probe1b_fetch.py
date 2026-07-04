#!/usr/bin/env python3
"""M0 Probe 1b — delta content-fetch strategy (refines Pillar 1).

Probe 1 found `jj file show -r REV path` costs ~300ms/file at 100K (it re-resolves
the tree each call), so Pillar 1's per-changed-file loop is a scaled-down Break #1:
a 200-file delta would be ~60s. This compares three ways to fetch the *content* of
K changed files at a revision, on a 100K corpus:

  A. loop:  K x `jj file show -r D <path>`            (one subprocess per file)
  B. batch: `jj file show -r D <path1> ... <pathK>`   (one subprocess, many paths)
  C. diff:  `jj diff --from C --to D --git`           (one subprocess, full delta)

Usage: probe1b_fetch.py [corpus_count] [K ...]   (default: 100000 50 200)
"""
import os
import shutil
import statistics
import subprocess
import sys
import tempfile
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
GEN = HERE / "gen_corpus"
REPEATS = 3


def sh(args, cwd, env):
    r = subprocess.run(args, cwd=cwd, env=env, capture_output=True, text=True)
    if r.returncode != 0:
        raise RuntimeError(f"{args} failed:\n{r.stderr}")
    return r.stdout


def timed(args, cwd, env):
    t0 = time.perf_counter()
    out = sh(args, cwd, env)
    return time.perf_counter() - t0, out


def main():
    argv = sys.argv[1:]
    count = int(argv[0]) if argv else 100000
    ks = [int(x) for x in argv[1:]] or [50, 200]
    if not GEN.exists():
        sys.exit(f"build generator first: rustc -O -o {GEN} {GEN}.rs")

    root = Path(tempfile.mkdtemp(prefix="jjj_probe1b_"))
    rows = []
    try:
        home = root / "home"
        home.mkdir()
        env = dict(os.environ)
        env["HOME"] = str(home)
        env["JJ_CONFIG"] = str(home / "config.toml")
        (home / "config.toml").write_text(
            '[user]\nname = "probe"\nemail = "probe@example.com"\n'
        )
        repo = root / "repo"
        repo.mkdir()
        sh(["jj", "git", "init", "."], repo, env)
        print(f"... generating {count} corpus", flush=True)
        sh([str(GEN), str(repo), str(count), "flat"], repo, env)
        sh(["jj", "commit", "-m", "corpus"], repo, env)
        C = sh(["jj", "log", "--no-graph", "-r", "@-", "-T", "commit_id"], repo, env).strip()

        all_files = []
        for p in (repo / "problems").rglob("*.md"):
            all_files.append(p.relative_to(repo).as_posix())
            if len(all_files) >= max(ks):
                break

        for k in ks:
            paths = all_files[:k]
            sh(["jj", "new", C], repo, env)
            for rp in paths:
                with open(repo / rp, "a") as f:
                    f.write("\nedited\n")
            sh(["jj", "commit", "-m", f"delta{k}"], repo, env)
            D = sh(["jj", "log", "--no-graph", "-r", "@-", "-T", "commit_id"], repo, env).strip()

            # A. one file show per path
            a_times = []
            for _ in range(REPEATS):
                t0 = time.perf_counter()
                for rp in paths:
                    sh(["jj", "file", "show", "-r", D, rp], repo, env)
                a_times.append(time.perf_counter() - t0)

            # B. one batched file show with all paths
            b_times = [timed(["jj", "file", "show", "-r", D, *paths], repo, env)[0]
                       for _ in range(REPEATS)]

            # C. one diff --git over the delta
            c_times = [timed(["jj", "diff", "--from", C, "--to", D, "--git"], repo, env)[0]
                       for _ in range(REPEATS)]

            rows.append((k, statistics.median(a_times), statistics.median(b_times),
                         statistics.median(c_times)))
    finally:
        shutil.rmtree(root, ignore_errors=True)

    print(f"\n=== Probe 1b: delta content-fetch on {count} corpus ===")
    hdr = f"{'K':>5} {'A_loop_s':>10} {'B_batch_s':>11} {'C_diffgit_s':>12} {'speedup_A/B':>12}"
    print(hdr)
    print("-" * len(hdr))
    for k, a, b, c in rows:
        print(f"{k:>5} {a:>10.3f} {b:>11.3f} {c:>12.3f} {a/b:>11.1f}x")
    print("\nIf B/C are ~flat in K while A grows linearly => Pillar 1 must batch the fetch,")
    print("not loop `jj file show`.")


if __name__ == "__main__":
    main()
