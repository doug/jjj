#!/usr/bin/env python3
"""M0 Probe 1 — jj tree-diff cost at scale (validates Pillar 1 keystone).

Question: is `jj diff --from A --to B --name-only` sub-second between two
commits whose trees hold 25K / 100K entity files but differ by a *small* delta?
If not, the delta-fetch keystone is in doubt. Also compares flat vs fan-out
directory layout, and measures `jj file show -r REV path` (the per-changed-file
fetch cost Pillar 1 pays in its loop).

Method (isolates pure tree-diff from working-copy snapshot cost):
  1. generate corpus into @, `jj commit` -> corpus commit C, @ becomes empty
  2. `jj new C`, modify K files, `jj commit` -> delta commit D, @ empty again
  3. with @ empty (cheap snapshot), time `jj diff --from C --to D --name-only`

Usage: probe1_treediff.py [counts...]   (default: 1000 25000 100000)
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
DELTA_FILES = 5
REPEATS = 3


def sh(args, cwd, env=None):
    r = subprocess.run(args, cwd=cwd, env=env, capture_output=True, text=True)
    if r.returncode != 0:
        raise RuntimeError(f"cmd {args} failed in {cwd}:\n{r.stderr}")
    return r.stdout


def timed(args, cwd, env):
    t0 = time.perf_counter()
    out = sh(args, cwd, env)
    return time.perf_counter() - t0, out


def jj_env(home):
    # Isolated jj/git identity so commits don't depend on the user's config.
    env = dict(os.environ)
    env["HOME"] = str(home)
    env["JJ_CONFIG"] = str(home / "config.toml")
    (home / "config.toml").write_text(
        '[user]\nname = "probe"\nemail = "probe@example.com"\n'
    )
    return env


def commit_id(cwd, env, rev):
    return sh(["jj", "log", "--no-graph", "-r", rev, "-T", "commit_id"], cwd, env).strip()


def run_case(count, layout, root):
    repo = root / f"{layout}_{count}"
    home = root / f"home_{layout}_{count}"
    home.mkdir(parents=True)
    env = jj_env(home)
    repo.mkdir()
    sh(["jj", "git", "init", "."], repo, env)

    work = repo  # entity files live in the working copy for the probe
    t_gen0 = time.perf_counter()
    sh([str(GEN), str(work), str(count), layout], repo, env)
    t_gen = time.perf_counter() - t_gen0

    # corpus commit C
    t_snap, _ = timed(["jj", "commit", "-m", "corpus"], repo, env)
    C = commit_id(repo, env, "@-")

    # build delta commit D on top of C
    sh(["jj", "new", C], repo, env)
    files = []
    for p in (work / "problems").rglob("*.md"):
        files.append(p)
        if len(files) >= DELTA_FILES:
            break
    for p in files:
        with open(p, "a") as f:
            f.write("\nedited for delta probe\n")
    sh(["jj", "commit", "-m", "delta"], repo, env)
    D = commit_id(repo, env, "@-")

    # @ is now empty -> snapshot is trivial; measure pure tree-diff
    diff_times = []
    nchanged = None
    for _ in range(REPEATS):
        dt, out = timed(
            ["jj", "diff", "--from", C, "--to", D, "--name-only"], repo, env
        )
        diff_times.append(dt)
        nchanged = len([l for l in out.splitlines() if l.strip()])

    one = files[0].relative_to(work).as_posix()
    show_times = []
    for _ in range(REPEATS):
        st, _ = timed(["jj", "file", "show", "-r", C, one], repo, env)
        show_times.append(st)

    return {
        "count": count,
        "layout": layout,
        "gen_s": t_gen,
        "snapshot_s": t_snap,
        "treediff_s": statistics.median(diff_times),
        "treediff_min_s": min(diff_times),
        "nchanged": nchanged,
        "fileshow_ms": statistics.median(show_times) * 1000,
    }


def main():
    counts = [int(x) for x in sys.argv[1:]] or [1000, 25000, 100000]
    if not GEN.exists():
        sys.exit(f"generator not built: {GEN}\n  build: rustc -O -o {GEN} {GEN}.rs")
    rows = []
    root = Path(tempfile.mkdtemp(prefix="jjj_probe1_"))
    try:
        for count in counts:
            for layout in ("flat", "fanout"):
                print(f"... {layout:6} {count:>7} (generating + committing)", flush=True)
                rows.append(run_case(count, layout, root))
    finally:
        shutil.rmtree(root, ignore_errors=True)

    print("\n=== Probe 1: jj tree-diff at scale (delta = %d files) ===" % DELTA_FILES)
    hdr = f"{'count':>7} {'layout':>7} {'gen_s':>7} {'snapshot_s':>11} {'treediff_s':>11} {'treediff_min':>13} {'changed':>8} {'fileshow_ms':>12}"
    print(hdr)
    print("-" * len(hdr))
    for r in rows:
        print(
            f"{r['count']:>7} {r['layout']:>7} {r['gen_s']:>7.2f} {r['snapshot_s']:>11.2f} "
            f"{r['treediff_s']:>11.3f} {r['treediff_min_s']:>13.3f} {r['nchanged']:>8} {r['fileshow_ms']:>12.1f}"
        )
    print("\nKeystone gate: treediff_s must be < 1.0s at 25K and < ~1.0s at 100K.")


if __name__ == "__main__":
    main()
