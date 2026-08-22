"""A minimal SQL engine. It answers `SELECT * FROM <table>` and nothing else.

This is a starting shape, not a foundation to be preserved — the parser here is
a regex, which will not survive contact with tier 2. Replace it.

The contract the harness depends on, and the only thing that must not change:

    from sqlengine import Database
    db = Database()
    db.execute("CREATE TABLE t (a INTEGER, b TEXT)")   # -> None
    db.execute("INSERT INTO t VALUES (1, 'x')")        # -> None
    db.execute("SELECT * FROM t")                      # -> [(1, 'x')]

`execute` returns a list of tuples for a SELECT and None otherwise, and raises
on anything it cannot answer. Raising is honest; returning [] for a query you
did not understand scores the same as returning [] for a query that genuinely
has no rows, and hides the failure from you.
"""

import re


class Database:
    def __init__(self):
        self.tables = {}   # name -> {"columns": [str], "rows": [tuple]}

    def execute(self, sql):
        s = sql.strip().rstrip(";")
        low = s.lower()
        if low.startswith("create table"):
            return self._create(s)
        if low.startswith("insert into"):
            return self._insert(s)
        if low.startswith("select"):
            return self._select(s)
        raise NotImplementedError(f"unsupported statement: {s[:60]}")

    def _create(self, s):
        m = re.match(r"create\s+table\s+(\w+)\s*\((.*)\)\s*$", s, re.I | re.S)
        if not m:
            raise ValueError(f"cannot parse: {s[:60]}")
        name, body = m.group(1), m.group(2)
        cols = [c.strip().split()[0] for c in body.split(",")]
        self.tables[name] = {"columns": cols, "rows": []}

    def _insert(self, s):
        m = re.match(r"insert\s+into\s+(\w+)\s+values\s*\((.*)\)\s*$", s, re.I | re.S)
        if not m:
            raise ValueError(f"cannot parse: {s[:60]}")
        name, body = m.group(1), m.group(2)
        if name not in self.tables:
            raise KeyError(f"no such table: {name}")
        self.tables[name]["rows"].append(tuple(_value(v) for v in _split(body)))

    def _select(self, s):
        m = re.match(r"select\s+(.+?)\s+from\s+(\w+)\s*$", s, re.I)
        if not m:
            raise NotImplementedError(f"unsupported select: {s[:60]}")
        sel, name = m.group(1).strip(), m.group(2)
        if name not in self.tables:
            raise KeyError(f"no such table: {name}")
        cols, rows = self.tables[name]["columns"], self.tables[name]["rows"]
        if sel == "*":
            return list(rows)
        want = [c.strip() for c in sel.split(",")]
        if not all(c in cols for c in want):
            raise NotImplementedError(f"only bare column names are implemented: {sel[:40]}")
        idx = [cols.index(c) for c in want]
        return [tuple(r[i] for i in idx) for r in rows]


def _split(body):
    """Split on commas that are not inside a quoted string."""
    out, cur, quoted = [], "", False
    for ch in body:
        if ch == "'":
            quoted = not quoted
            cur += ch
        elif ch == "," and not quoted:
            out.append(cur)
            cur = ""
        else:
            cur += ch
    out.append(cur)
    return [x.strip() for x in out]


def _value(tok):
    t = tok.strip()
    if t.upper() == "NULL":
        return None
    if t.startswith("'") and t.endswith("'"):
        return t[1:-1].replace("''", "'")
    try:
        return int(t)
    except ValueError:
        return float(t)
