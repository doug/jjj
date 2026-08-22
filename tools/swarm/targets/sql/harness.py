#!/usr/bin/env python3
"""Differential-test a SQL engine against SQLite.

The target has no fixed ceiling on purpose. A swarm solved the 31-operation toy
in fifteen minutes, because "implement N things" saturates the moment N things
exist. Here truth is defined by an oracle — `sqlite3` from the standard library
— over a deterministically generated corpus, so the frontier is always another
failing case rather than a checklist.

Two processes, deliberately:

  * this one holds the oracle and never loads the engine
  * `runner.py` loads the engine with `sqlite3` blocked, and never sees an
    expected result

Expected outputs are computed at score time and never written into the
workbench. An engine that could reach the oracle would score 100% without
implementing anything, and an agent optimising against a gate it can defeat
will defeat it — twice today a swarm "improved" a benchmark by removing the
correctness check it was measured against.

  harness.py gen   --out corpus.json [--seed N] [--cases N]
  harness.py score --corpus corpus.json --engine DIR [--timeout S]
"""

import argparse
import json
import pathlib
import random
import sqlite3
import subprocess
import sys

SCHEMA = [
    "CREATE TABLE users (id INTEGER, name TEXT, age INTEGER, score REAL, city TEXT)",
    "CREATE TABLE orders (id INTEGER, user_id INTEGER, amount REAL, status TEXT, qty INTEGER)",
    "CREATE TABLE items (id INTEGER, order_id INTEGER, sku TEXT, price REAL)",
]

CITIES = ["porto", "oslo", "lima", "cairo", None, "kyoto"]
NAMES = ["ada", "brin", "cleo", "dara", "esa", None, "fen", "gus"]
STATUS = ["open", "shipped", "cancelled", None, "held"]
SKUS = ["a-1", "b-2", "c-3", "d-4", None]

# Every tier's weight in the final score. Tier 1 alone cannot carry a run, and
# the hard tiers are where the twenty-fourth hour goes.
TIERS = {1: "projection & filters", 2: "logic, ordering, limits",
         3: "aggregates & grouping", 4: "joins",
         5: "NULL semantics, types, subqueries"}


def build_data(rng):
    rows = []
    for i in range(1, 41):
        rows.append("INSERT INTO users VALUES (%d, %s, %s, %s, %s)" % (
            i, lit(rng.choice(NAMES)), lit(rng.choice([None, *range(18, 70)])),
            lit(rng.choice([None, *[round(rng.uniform(0, 100), 2) for _ in range(5)]])),
            lit(rng.choice(CITIES))))
    for i in range(1, 61):
        rows.append("INSERT INTO orders VALUES (%d, %s, %s, %s, %s)" % (
            i, lit(rng.choice([None, *range(1, 45)])),
            lit(rng.choice([None, *[round(rng.uniform(0, 500), 2) for _ in range(6)]])),
            lit(rng.choice(STATUS)), lit(rng.choice([None, *range(0, 9)]))))
    for i in range(1, 81):
        rows.append("INSERT INTO items VALUES (%d, %s, %s, %s)" % (
            i, lit(rng.choice([None, *range(1, 65)])), lit(rng.choice(SKUS)),
            lit(rng.choice([None, *[round(rng.uniform(0, 99), 2) for _ in range(5)]]))))
    return rows


def lit(v):
    if v is None:
        return "NULL"
    if isinstance(v, str):
        return "'" + v.replace("'", "''") + "'"
    return repr(v)


NUMCOL = {"users": ["id", "age", "score"], "orders": ["id", "user_id", "amount", "qty"],
          "items": ["id", "order_id", "price"]}
TXTCOL = {"users": ["name", "city"], "orders": ["status"], "items": ["sku"]}
ALLCOL = {t: NUMCOL[t] + TXTCOL[t] for t in NUMCOL}


def gen_tier1(rng):
    t = rng.choice(list(ALLCOL))
    cols = rng.choice(["*", ", ".join(rng.sample(ALLCOL[t], rng.randint(1, 3)))])
    if rng.random() < 0.35:
        return f"SELECT {cols} FROM {t}"
    c = rng.choice(NUMCOL[t])
    op = rng.choice(["=", "<", ">", "<=", ">=", "<>"])
    return f"SELECT {cols} FROM {t} WHERE {c} {op} {rng.randint(0, 90)}"


def gen_tier2(rng):
    t = rng.choice(list(ALLCOL))
    n1, n2 = rng.sample(NUMCOL[t], 2)
    body = rng.choice([
        f"SELECT * FROM {t} WHERE {n1} > {rng.randint(0,50)} AND {n2} < {rng.randint(50,300)}",
        f"SELECT * FROM {t} WHERE {n1} < {rng.randint(0,40)} OR {n2} >= {rng.randint(10,200)}",
        f"SELECT * FROM {t} WHERE NOT {n1} > {rng.randint(0,60)}",
        f"SELECT {n1}, {n2} FROM {t} WHERE {n1} IS NULL",
        f"SELECT {n1}, {n2} FROM {t} WHERE {n2} IS NOT NULL",
        f"SELECT {n1} + {n2} FROM {t}",
        f"SELECT {n1} * 2 - 1 FROM {t} WHERE {n1} IS NOT NULL",
    ])
    if rng.random() < 0.55:
        body += f" ORDER BY {rng.choice(ALLCOL[t])}" + rng.choice(["", " DESC", " ASC"])
    if rng.random() < 0.4:
        body += f" LIMIT {rng.randint(1,7)}"
        if rng.random() < 0.3:
            body += f" OFFSET {rng.randint(0,4)}"
    return body


def gen_tier3(rng):
    t = rng.choice(list(ALLCOL))
    n = rng.choice(NUMCOL[t])
    g = rng.choice(ALLCOL[t])
    agg = rng.choice(["COUNT(*)", f"COUNT({n})", f"SUM({n})", f"AVG({n})",
                      f"MIN({n})", f"MAX({n})"])
    choice = rng.random()
    if choice < 0.25:
        return f"SELECT {agg} FROM {t}"
    if choice < 0.45:
        return f"SELECT DISTINCT {g} FROM {t}"
    q = f"SELECT {g}, {agg} FROM {t} GROUP BY {g}"
    if rng.random() < 0.4:
        q += f" HAVING {agg} > {rng.randint(0,4)}"
    if rng.random() < 0.5:
        q += f" ORDER BY {g}"
    return q


def gen_tier4(rng):
    kind = rng.choice(["INNER JOIN", "LEFT JOIN"])
    pair = rng.choice([("users", "orders", "users.id", "orders.user_id"),
                       ("orders", "items", "orders.id", "items.order_id")])
    a, b, ka, kb = pair
    cols = rng.choice([f"{a}.id, {b}.id", f"{a}.*", f"{b}.*",
                       f"{a}.id, {rng.choice(NUMCOL[b])}"])
    q = f"SELECT {cols} FROM {a} {kind} {b} ON {ka} = {kb}"
    if rng.random() < 0.4:
        q += f" WHERE {rng.choice(NUMCOL[a])} > {rng.randint(0,30)}"
    if rng.random() < 0.35:
        q += f" ORDER BY {ka}"
    if rng.random() < 0.3:
        q += f" LIMIT {rng.randint(2,8)}"
    return q


def gen_tier5(rng):
    t = rng.choice(list(ALLCOL))
    n = rng.choice(NUMCOL[t])
    s = rng.choice(TXTCOL[t])
    return rng.choice([
        f"SELECT {s} FROM {t} WHERE {s} LIKE '{rng.choice(['a%','%o','%a%','_-2'])}'",
        f"SELECT {n} FROM {t} WHERE {n} IN ({rng.randint(1,9)}, {rng.randint(10,40)}, NULL)",
        f"SELECT {n} FROM {t} WHERE {n} NOT IN ({rng.randint(1,9)}, {rng.randint(10,40)})",
        f"SELECT CASE WHEN {n} > {rng.randint(1,50)} THEN 'hi' WHEN {n} IS NULL THEN 'none' ELSE 'lo' END FROM {t}",
        f"SELECT COUNT(*) FROM {t} WHERE {s} IS NULL",
        f"SELECT {n} FROM {t} WHERE {n} = NULL",
        f"SELECT {n} FROM {t} WHERE ({n} > 5) IS NULL",
        f"SELECT * FROM users WHERE id IN (SELECT user_id FROM orders WHERE amount > {rng.randint(50,300)})",
        f"SELECT * FROM orders WHERE user_id NOT IN (SELECT id FROM users WHERE age IS NOT NULL)",
        f"SELECT {n} + NULL FROM {t}",
        f"SELECT {s} || 'x' FROM {t}",
        f"SELECT {n} FROM {t} WHERE {n} BETWEEN {rng.randint(0,20)} AND {rng.randint(21,90)}",
        f"SELECT {n} FROM {t} ORDER BY {n} DESC",
    ])


GENS = {1: gen_tier1, 2: gen_tier2, 3: gen_tier3, 4: gen_tier4, 5: gen_tier5}


def oracle_conn(schema, data):
    conn = sqlite3.connect(":memory:")
    for s in schema:
        conn.execute(s)
    for d in data:
        conn.execute(d)
    conn.commit()
    return conn


def canon(rows, ordered):
    """Normalise a result set for comparison.

    Rows are compared as a multiset unless the query has an ORDER BY: SQL does
    not promise an order without one, so demanding a particular order would
    mark a correct engine wrong.
    """
    out = []
    for r in rows:
        vals = []
        for v in r:
            if isinstance(v, bool):
                v = int(v)
            if isinstance(v, float):
                # sqlite may hand back 3.0 where an engine computes 3; and
                # float arithmetic differs in the last bits either way.
                v = round(v, 9)
                if v == int(v):
                    v = int(v)
            elif isinstance(v, int):
                pass
            elif v is not None and not isinstance(v, str):
                v = str(v)
            vals.append(v)
        out.append(tuple(vals))
    if ordered:
        return out
    return sorted(out, key=lambda t: [(x is None, str(type(x)), str(x)) for x in t])


def cmd_gen(args):
    rng = random.Random(args.seed)
    data = build_data(rng)
    conn = oracle_conn(SCHEMA, data)

    # Generation is unfiltered, so some queries are invalid. Keeping only the
    # ones the oracle accepts means a failing case is always the engine's
    # fault, never the generator's.
    want = {1: 0.22, 2: 0.22, 3: 0.20, 4: 0.16, 5: 0.20}
    cases, seen = [], set()
    for tier, frac in want.items():
        target = max(1, int(args.cases * frac))
        tries = 0
        while sum(1 for c in cases if c["tier"] == tier) < target and tries < target * 60:
            tries += 1
            sql = GENS[tier](rng)
            if sql in seen:
                continue
            try:
                conn.execute(sql).fetchall()
            except Exception:
                continue
            seen.add(sql)
            cases.append({"id": len(cases), "tier": tier, "sql": sql})

    corpus = {"seed": args.seed, "schema": SCHEMA, "data": data, "cases": cases}
    pathlib.Path(args.out).write_text(json.dumps(corpus, indent=1) + "\n")
    per = {t: sum(1 for c in cases if c["tier"] == t) for t in TIERS}
    print(f"wrote {len(cases)} cases to {args.out}: " +
          ", ".join(f"tier{t}={n}" for t, n in per.items()))


def cmd_score(args):
    corpus = json.loads(pathlib.Path(args.corpus).read_text())
    cases = corpus["cases"]

    conn = oracle_conn(corpus["schema"], corpus["data"])
    expected = {}
    for c in cases:
        try:
            expected[c["id"]] = conn.execute(c["sql"]).fetchall()
        except Exception:
            expected[c["id"]] = None

    runner = pathlib.Path(__file__).with_name("runner.py")
    proc = subprocess.run(
        [sys.executable, str(runner), args.corpus, str(args.timeout)],
        capture_output=True, text=True, cwd=args.engine,
        timeout=args.timeout * len(cases) + 120)
    if proc.returncode != 0:
        print(f"engine runner failed: {proc.stderr.strip()[-400:]}", file=sys.stderr)
        print("0 100")
        return
    try:
        got = json.loads(proc.stdout)
    except Exception:
        print(f"engine runner produced no result: {proc.stdout[-300:]}", file=sys.stderr)
        print("0 100")
        return

    per = {t: [0, 0] for t in TIERS}
    for c in cases:
        t = c["tier"]
        per[t][1] += 1
        exp = expected[c["id"]]
        if exp is None:
            per[t][0] += 1  # oracle could not run it either; do not punish
            continue
        r = got.get(str(c["id"]))
        if r is None or r.get("error"):
            continue
        ordered = " order by " in c["sql"].lower()
        try:
            if canon([tuple(x) for x in r["rows"]], ordered) == canon(exp, ordered):
                per[t][0] += 1
        except Exception:
            pass

    # Equal weight per tier, not per case: otherwise a fleet maximises the
    # score by grinding the easy tier, and the hard semantics that make this a
    # 24-hour target never get touched.
    frac = sum((p / n if n else 0.0) for p, n in per.values()) / len(per)
    score = max(0, min(100, round(frac * 100)))
    for t, (p, n) in sorted(per.items()):
        bar = "#" * int(20 * p / n) if n else ""
        print(f"  tier{t} {p:>4}/{n:<4} {100*p//max(n,1):>3}%  {bar:<20} {TIERS[t]}",
              file=sys.stderr)
    print(f"{score} 100")


def main():
    ap = argparse.ArgumentParser(description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = ap.add_subparsers(dest="cmd", required=True)
    g = sub.add_parser("gen"); g.set_defaults(fn=cmd_gen)
    g.add_argument("--out", required=True)
    g.add_argument("--seed", type=int, default=20260821)
    g.add_argument("--cases", type=int, default=2000)
    s = sub.add_parser("score"); s.set_defaults(fn=cmd_score)
    s.add_argument("--corpus", required=True)
    s.add_argument("--engine", required=True)
    s.add_argument("--timeout", type=float, default=2.0)
    args = ap.parse_args()
    args.fn(args)


if __name__ == "__main__":
    main()
