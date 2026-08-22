#!/usr/bin/env python3
"""Score a SQL engine on latency under a budget, with SQLite as the oracle.

The correctness-only version of this target was solved to 99/100 in ten minutes:
"implement N features" saturates, because N is finite and a capable model can
write all of them at once. Latency does not saturate. There is always another
constant to shave, and the ceiling is set by what is achievable rather than by
a checklist someone wrote down.

A query scores only if it is **both correct and inside its budget**. Wrong and
fast is worth nothing, which is the property that keeps this from degenerating
into a benchmark you can win by returning [].

The budgets are set so that a straightforward implementation misses them. Over
these tables a full scan per query is roughly a second in pure Python, and a
nested-loop join is minutes — so the work is indexes, hash joins, predicate
pushdown, projection pruning, and not much else. That is deliberate: the tuning
that gets you from 40% to 80% here is the tuning a real engine does.

  harness.py gen   --out DIR [--seed N] [--scale F]
  harness.py score --data DIR --engine DIR
"""

import argparse
import csv
import hashlib
import json
import math
import pathlib
import random
import sqlite3
import subprocess
import sys
import time

from spec import (CLASSES, ROWS, SCHEMA, STATUS, CITIES, build_workload,
                  coerce, ddl, digest)

def cmd_gen(args):
    out = pathlib.Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    rng = random.Random(args.seed)
    counts = {t: max(100, int(n * args.scale)) for t, n in ROWS.items()}

    def write(table, rowgen):
        with (out / f"{table}.csv").open("w", newline="") as f:
            w = csv.writer(f)
            w.writerow([n for n, _ in SCHEMA[table]])
            for row in rowgen:
                w.writerow(["\\N" if v is None else v for v in row])

    n_users = counts["users"]
    write("users", (
        (i, f"u{rng.randrange(10_000)}",
         None if rng.random() < 0.05 else rng.randrange(18, 80),
         None if rng.random() < 0.05 else round(rng.uniform(0, 100), 3),
         rng.choice(CITIES))
        for i in range(1, n_users + 1)))

    n_orders = counts["orders"]
    write("orders", (
        (i,
         None if rng.random() < 0.02 else rng.randrange(1, n_users + 1),
         None if rng.random() < 0.03 else round(rng.uniform(0, 1000), 2),
         rng.choice(STATUS),
         rng.randrange(0, 12))
        for i in range(1, n_orders + 1)))

    n_items = counts["items"]
    write("items", (
        (i,
         None if rng.random() < 0.02 else rng.randrange(1, n_orders + 1),
         f"sku-{rng.randrange(5000)}",
         None if rng.random() < 0.02 else round(rng.uniform(0, 500), 2))
        for i in range(1, n_items + 1)))

    (out / "manifest.json").write_text(json.dumps(
        {"seed": args.seed, "scale": args.scale, "counts": counts,
         "schema": {t: SCHEMA[t] for t in SCHEMA}}, indent=1) + "\n")
    print("generated " + ", ".join(f"{t}={n}" for t, n in counts.items()))


# Columns the oracle is indexed on. The oracle's speed is not what is being
# measured — the budgets are absolute — so an unindexed reference just makes
# every scoring run slower for nothing. Indexed, the reference answers the
# workload in milliseconds instead of eight seconds, and the score is unchanged
# because the rows are.
ORACLE_INDEXES = [
    "CREATE INDEX IF NOT EXISTS ix_users_id ON users(id)",
    "CREATE INDEX IF NOT EXISTS ix_users_city ON users(city)",
    "CREATE INDEX IF NOT EXISTS ix_users_age ON users(age)",
    "CREATE INDEX IF NOT EXISTS ix_orders_id ON orders(id)",
    "CREATE INDEX IF NOT EXISTS ix_orders_user ON orders(user_id)",
    "CREATE INDEX IF NOT EXISTS ix_orders_status ON orders(status)",
    "CREATE INDEX IF NOT EXISTS ix_orders_amount ON orders(amount)",
    "CREATE INDEX IF NOT EXISTS ix_items_id ON items(id)",
    "CREATE INDEX IF NOT EXISTS ix_items_order ON items(order_id)",
    "CREATE INDEX IF NOT EXISTS ix_items_sku ON items(sku)",
]


def load_sqlite(data):
    """Open the reference database, building it once and caching it on disk.

    Reloading 1.5M rows from CSV on every scoring run cost about ten seconds of
    every twenty-six — a third of a run's wall clock spent rebuilding an
    identical reference. Six agents score at least twice a turn, so it was the
    single largest avoidable cost in the loop.

    The cache is keyed on nothing because the data never changes within a
    workbench; delete `data/oracle.db` to rebuild it.
    """
    cached = pathlib.Path(data) / "oracle.db"
    if cached.exists():
        return sqlite3.connect(str(cached))

    tmp = cached.with_suffix(".db.tmp")
    tmp.unlink(missing_ok=True)
    conn = sqlite3.connect(str(tmp))
    for t in SCHEMA:
        conn.execute(ddl(t))
        with (pathlib.Path(data) / f"{t}.csv").open() as f:
            r = csv.reader(f)
            next(r)
            conn.executemany(
                f"INSERT INTO {t} VALUES ({','.join('?' * len(SCHEMA[t]))})",
                (tuple(coerce(v, ty) for v, (_, ty) in zip(row, SCHEMA[t])) for row in r))
    for stmt in ORACLE_INDEXES:
        conn.execute(stmt)
    conn.commit()
    conn.close()
    # Rename last, so a run interrupted mid-build does not leave a partial
    # database that every later run would happily open.
    tmp.rename(cached)
    return sqlite3.connect(str(cached))


def cmd_score(args):
    # Fresh literals every run, so an engine cannot precompute the answers to a
    # known query list. The parent draws them and hands the concrete workload to
    # the runner; both sides must see exactly the same queries.
    counts = json.loads((pathlib.Path(args.data) / "manifest.json").read_text())["counts"]
    workload = build_workload(random.Random(), counts)

    conn = load_sqlite(args.data)
    expected, oracle_ms = {}, {}
    for name, sql, budget, cls in workload:
        t0 = time.perf_counter()
        rows = conn.execute(sql).fetchall()
        oracle_ms[name] = (time.perf_counter() - t0) * 1000
        expected[name] = digest(rows, " order by " in sql.lower())

    runner = pathlib.Path(__file__).with_name("runner.py")
    # The whole run is bounded too: budgets sum to well under a minute, and a
    # load that never finishes must not hang an agent's turn.
    cap = sum(b for _, _, b, _ in workload) + 180
    wl = pathlib.Path(args.data) / ".workload.json"
    wl.write_text(json.dumps(workload))
    try:
        proc = subprocess.run(
            [sys.executable, str(runner), str(pathlib.Path(args.data).resolve()),
             str(wl.resolve())],
            capture_output=True, text=True, cwd=args.engine, timeout=cap)
    except subprocess.TimeoutExpired:
        print("engine exceeded the overall time cap (load or a query hung)", file=sys.stderr)
        print("0 100")
        return
    if proc.returncode != 0 or not proc.stdout.strip():
        print(f"engine runner failed: {proc.stderr.strip()[-400:]}", file=sys.stderr)
        print("0 100")
        return
    got = json.loads(proc.stdout)
    if "__error__" in got:
        print(f"engine error: {got['__error__']}", file=sys.stderr)
        print("0 100")
        return

    per = {c: [0, 0] for c in CLASSES}
    lines = []
    for name, sql, budget, cls in workload:
        per[cls][1] += 1
        r = got.get(name) or {}
        ok = r.get("digest") == expected[name]
        ms = r.get("ms")
        in_budget = ms is not None and ms <= budget * 1000
        if ok and in_budget:
            # Meeting the budget is worth half. The other half arrives
            # logarithmically as you get further under it, and only a query
            # eight times inside its budget scores full marks.
            #
            # This is the part the correctness target got wrong: a score you can
            # max out is a checklist, and a fleet that can finish a checklist
            # finishes it in an afternoon. There is always another constant to
            # shave, so the scale should keep paying for it.
            speed = (budget * 1000) / max(ms, 1e-6)
            pts = 0.5 + 0.5 * min(1.0, math.log(speed) / math.log(8))
            per[cls][0] += pts
            verdict = "ok" if pts < 0.99 else "FAST"
        elif not ok:
            verdict = "WRONG" if ms is not None else f"ERR {r.get('error', '?')}"
        else:
            verdict = "SLOW"
        lines.append(f"    {name:<16} {verdict:<10} "
                     f"{'-' if ms is None else format(ms, '8.1f')}ms / {budget*1000:6.0f}ms budget"
                     f"   (sqlite {oracle_ms[name]:.1f}ms)")

    frac = sum((p / n if n else 0) for p, n in per.values()) / len(per)
    score = max(0, min(100, round(frac * 100)))
    for c in CLASSES:
        p, n = per[c]
        pct = 100 * p / max(n, 1)
        bar = "#" * int(pct / 5)
        print(f"  {c:<8} {p:>5.1f}/{n:<2} {pct:>5.1f}%  {bar}", file=sys.stderr)
    if args.verbose:
        print("\n".join(lines), file=sys.stderr)
    print(f"{score} 100")


def main():
    ap = argparse.ArgumentParser(description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = ap.add_subparsers(dest="cmd", required=True)
    g = sub.add_parser("gen"); g.set_defaults(fn=cmd_gen)
    g.add_argument("--out", required=True)
    g.add_argument("--seed", type=int, default=20260821)
    g.add_argument("--scale", type=float, default=1.0)
    s = sub.add_parser("score"); s.set_defaults(fn=cmd_score)
    s.add_argument("--data", required=True)
    s.add_argument("--engine", required=True)
    s.add_argument("-v", "--verbose", action="store_true")
    args = ap.parse_args()
    args.fn(args)


if __name__ == "__main__":
    main()
