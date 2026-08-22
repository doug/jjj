"""Shared definitions for the latency target.

Deliberately free of `sqlite3`. The runner imports this to know the schema and
the workload, and the runner also blocks the oracle — so anything both sides
need has to live somewhere the blocked process can safely import. Putting these
in the harness meant the runner could not start at all.
"""

import hashlib

SCHEMA = {
    "users": [("id", "INTEGER"), ("name", "TEXT"), ("age", "INTEGER"),
              ("score", "REAL"), ("city", "TEXT")],
    "orders": [("id", "INTEGER"), ("user_id", "INTEGER"), ("amount", "REAL"),
               ("status", "TEXT"), ("qty", "INTEGER")],
    "items": [("id", "INTEGER"), ("order_id", "INTEGER"), ("sku", "TEXT"),
              ("price", "REAL")],
}

# Rows at scale 1.0. Big enough that scanning costs real time in Python, small
# enough to load in a few seconds and sit in a 4GB container.
ROWS = {"users": 50_000, "orders": 500_000, "items": 1_000_000}

CITIES = ["porto", "oslo", "lima", "cairo", "kyoto", "quito", "bergen", "hue"]
STATUS = ["open", "shipped", "cancelled", "held", "refunded"]


def ddl(table):
    cols = ", ".join(f"{n} {t}" for n, t in SCHEMA[table])
    return f"CREATE TABLE {table} ({cols})"


# The workload. Each entry is (name, sql-template, budget_seconds, class), and
# the braces are filled with fresh values on every scoring run.
#
# The shapes are fixed — they have to be, or the budgets would mean nothing —
# but the literals are not. A fixed query list is a list of 26 answers that can
# be computed once at load time and looked up, which is the same hole that let a
# memorised corpus score on the correctness target. Drawing the values per run
# closes it without changing what is being measured.
#
# Budgets are absolute rather than a multiple of SQLite's time. SQLite is C with
# a B-tree; a multiple of its milliseconds would be either unreachable or
# meaningless depending on the query, whereas an absolute budget states the
# actual requirement.
WORKLOAD_TEMPLATES = [
    ("point_user",      "SELECT * FROM users WHERE id = {uid}", 0.05, "lookup"),
    ("point_order",     "SELECT * FROM orders WHERE id = {oid}", 0.05, "lookup"),
    ("point_item",      "SELECT * FROM items WHERE id = {iid}", 0.05, "lookup"),
    ("selective_sku",   "SELECT id, price FROM items WHERE sku = '{sku}'", 0.30, "lookup"),
    ("selective_city",  "SELECT id, name FROM users WHERE city = '{city}'", 0.30, "lookup"),
    ("range_amount",    "SELECT id FROM orders WHERE amount > {hi_amount}", 0.40, "scan"),
    ("range_age",       "SELECT id FROM users WHERE age >= {hi_age}", 0.20, "scan"),

    ("scan_count",      "SELECT COUNT(*) FROM items", 0.50, "scan"),
    ("scan_sum",        "SELECT SUM(price) FROM items", 0.80, "scan"),
    ("scan_filter",     "SELECT COUNT(*) FROM orders WHERE status = '{status}'", 0.60, "scan"),
    ("scan_null",       "SELECT COUNT(*) FROM orders WHERE amount IS NULL", 0.60, "scan"),

    ("group_city",      "SELECT city, COUNT(*) FROM users GROUP BY city", 0.30, "group"),
    ("group_status",    "SELECT status, COUNT(*), SUM(qty) FROM orders GROUP BY status", 1.00, "group"),
    ("group_sku",       "SELECT sku, COUNT(*) FROM items GROUP BY sku", 1.50, "group"),
    ("group_having",    "SELECT user_id, COUNT(*) FROM orders GROUP BY user_id HAVING COUNT(*) > {havg}", 1.20, "group"),
    ("group_avg",       "SELECT status, AVG(amount) FROM orders GROUP BY status", 1.00, "group"),

    ("join_small",      "SELECT users.name, orders.amount FROM users INNER JOIN orders ON users.id = orders.user_id WHERE users.id < {few}", 0.60, "join"),
    ("join_big",        "SELECT COUNT(*) FROM orders INNER JOIN items ON orders.id = items.order_id", 2.50, "join"),
    ("join_left",       "SELECT COUNT(*) FROM users LEFT JOIN orders ON users.id = orders.user_id", 1.50, "join"),
    ("join_filtered",   "SELECT COUNT(*) FROM users INNER JOIN orders ON users.id = orders.user_id WHERE users.city = '{city2}' AND orders.status = '{status2}'", 1.50, "join"),
    ("join_three",      "SELECT COUNT(*) FROM users INNER JOIN orders ON users.id = orders.user_id INNER JOIN items ON orders.id = items.order_id WHERE users.id < {some}", 2.50, "join"),

    # Every ORDER BY ... LIMIT carries a unique tiebreak, because without one
    # the answer is not unique and the comparison is unwinnable. `ORDER BY
    # amount DESC LIMIT 20` over 500,000 rows with 99,206 distinct amounts has
    # six rows tied at the twentieth value: SQLite returns three of them, an
    # engine returns three others, and both are correct SQL. That was scored as
    # WRONG for hours and drew five critiques from agents hunting a bug that
    # was not there.
    ("topn_price",      "SELECT id, price FROM items ORDER BY price DESC, id LIMIT {lim}", 1.20, "topn"),
    ("topn_amount",     "SELECT id, amount FROM orders ORDER BY amount DESC, id LIMIT {lim2}", 0.80, "topn"),
    ("topn_group",      "SELECT sku, COUNT(*) FROM items GROUP BY sku ORDER BY COUNT(*) DESC, sku LIMIT {lim3}", 1.80, "topn"),

    ("sub_in",          "SELECT COUNT(*) FROM orders WHERE user_id IN (SELECT id FROM users WHERE city = '{city3}')", 1.20, "sub"),
    ("sub_notin",       "SELECT COUNT(*) FROM orders WHERE user_id IN (SELECT id FROM users WHERE age > {oldish})", 1.20, "sub"),
]


def build_workload(rng, counts):
    """Fill the templates with fresh literals for one scoring run."""
    p = {
        "uid": rng.randrange(1, counts["users"] + 1),
        "oid": rng.randrange(1, counts["orders"] + 1),
        "iid": rng.randrange(1, counts["items"] + 1),
        "sku": f"sku-{rng.randrange(5000)}",
        "city": rng.choice(CITIES),
        "city2": rng.choice(CITIES),
        "city3": rng.choice(CITIES),
        "status": rng.choice(STATUS),
        "status2": rng.choice(STATUS),
        "hi_amount": rng.randrange(950, 999),
        "hi_age": rng.randrange(70, 79),
        "havg": rng.randrange(8, 20),
        "few": rng.randrange(40, 120),
        "some": rng.randrange(300, 900),
        "lim": rng.randrange(5, 25),
        "lim2": rng.randrange(5, 40),
        "lim3": rng.randrange(5, 20),
        "oldish": rng.randrange(60, 76),
    }
    return [(name, sql.format(**p), budget, cls)
            for name, sql, budget, cls in WORKLOAD_TEMPLATES]


CLASSES = ["lookup", "scan", "group", "join", "topn", "sub"]


def coerce(v, ty):
    if v == "\\N":
        return None
    if ty == "INTEGER":
        return int(v)
    if ty == "REAL":
        return float(v)
    return v


def digest(rows, ordered):
    """A checksum, not the rows themselves.

    Some of these answers are large, and shipping them between processes to
    compare would cost more than the query. Rows are canonicalised the same way
    on both sides and hashed; unordered results are sorted first, because SQL
    promises no order without an ORDER BY.
    """
    norm = []
    for r in rows:
        vals = []
        for v in r:
            if isinstance(v, bool):
                v = int(v)
            if isinstance(v, float):
                v = round(v, 6)
                if v == int(v):
                    v = int(v)
            vals.append("\x00N" if v is None else f"{type(v).__name__}:{v}")
        norm.append("\x01".join(vals))
    if not ordered:
        norm.sort()
    h = hashlib.sha256()
    for line in norm:
        h.update(line.encode())
        h.update(b"\x02")
    return h.hexdigest()[:16]


