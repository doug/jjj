"""Evaluator: executes the AST produced by sqlengine.parser against in-memory
tables, following SQLite's semantics for NULLs, type comparison, and joins.
"""

import functools
import re
from collections import namedtuple

from .parser import Parser, AGG_FUNCS

Env = namedtuple("Env", ["row", "rows", "db", "cache"])


class Database:
    def __init__(self):
        self.tables = {}  # name -> {"columns": [(name, affinity)], "rows": [tuple]}

    def bulk_load(self, table, rows):
        """Load many rows at once, bypassing the SQL layer.

        The latency target measures queries, not parsing a million INSERT
        statements, so data arrives already typed. Nothing here builds an index
        — that is the work.
        """
        if table not in self.tables:
            raise KeyError(f"no such table: {table}")
        self.tables[table]["rows"].extend(tuple(r) for r in rows)

    def execute(self, sql):
        ast = Parser.parse_sql(sql)
        if ast["type"] == "create":
            return self._create(ast)
        if ast["type"] == "insert":
            return self._insert(ast)
        if ast["type"] == "select":
            return self._select(ast, {})
        raise NotImplementedError(f"unsupported statement type: {ast['type']}")

    # -- DDL / DML --------------------------------------------------
    def _create(self, ast):
        cols = [(name, _affinity_of(decl)) for name, decl in ast["columns"]]
        self.tables[ast["table"]] = {"columns": cols, "rows": []}

    def _insert(self, ast):
        name = ast["table"]
        if name not in self.tables:
            raise KeyError(f"no such table: {name}")
        cache = {}
        env = Env(row={}, rows=None, db=self, cache=cache)
        for row_exprs in ast["rows"]:
            self.tables[name]["rows"].append(tuple(eval_expr(e, env) for e in row_exprs))

    # -- SELECT -------------------------------------------------------
    def _select(self, ast, cache):
        from_table = ast["from"]
        if from_table not in self.tables:
            raise KeyError(f"no such table: {from_table}")
        tables_order = [from_table] + [j["table"] for j in ast["joins"]]
        for t in tables_order:
            if t not in self.tables:
                raise KeyError(f"no such table: {t}")

        rows = self._base_rows(from_table)
        for j in ast["joins"]:
            rows = self._apply_join(rows, j, cache)

        if ast["where"] is not None:
            kept = []
            for r in rows:
                env = Env(row=r, rows=None, db=self, cache=cache)
                if eval_expr(ast["where"], env) is True:
                    kept.append(r)
            rows = kept

        aggregate = bool(ast["group_by"]) or _contains_agg_list(ast["columns"]) or (
            ast["having"] is not None and _contains_agg(ast["having"]))

        out_rows = []
        order_vals = []

        if aggregate:
            groups = self._group_rows(rows, ast["group_by"], cache)
            if ast["having"] is not None:
                kept = []
                for key, grows in groups:
                    env = Env(row=(grows[0] if grows else {}), rows=grows, db=self, cache=cache)
                    if eval_expr(ast["having"], env) is True:
                        kept.append((key, grows))
                groups = kept
            for key, grows in groups:
                env = Env(row=(grows[0] if grows else {}), rows=grows, db=self, cache=cache)
                out_rows.append(self._project(ast["columns"], env, tables_order))
                if ast["order_by"]:
                    order_vals.append([eval_expr(oe, env) for oe, _ in ast["order_by"]])
        else:
            for r in rows:
                env = Env(row=r, rows=None, db=self, cache=cache)
                out_rows.append(self._project(ast["columns"], env, tables_order))
                if ast["order_by"]:
                    order_vals.append([eval_expr(oe, env) for oe, _ in ast["order_by"]])

        if ast["distinct"]:
            seen = set()
            dedup_rows, dedup_order = [], []
            for i, row in enumerate(out_rows):
                if row not in seen:
                    seen.add(row)
                    dedup_rows.append(row)
                    if order_vals:
                        dedup_order.append(order_vals[i])
            out_rows, order_vals = dedup_rows, dedup_order

        if ast["order_by"]:
            directions = [d for _, d in ast["order_by"]]

            def row_cmp(i, j):
                for k in range(len(directions)):
                    c = _compare_nullable(order_vals[i][k], order_vals[j][k])
                    if directions[k] == "DESC":
                        c = -c
                    if c != 0:
                        return c
                return 0

            idx = sorted(range(len(out_rows)), key=functools.cmp_to_key(row_cmp))
            out_rows = [out_rows[i] for i in idx]

        empty_env = Env(row={}, rows=None, db=self, cache=cache)
        if ast["offset"] is not None:
            off = eval_expr(ast["offset"], empty_env)
            out_rows = out_rows[off:]
        if ast["limit"] is not None:
            lim = eval_expr(ast["limit"], empty_env)
            out_rows = out_rows[:lim]

        return out_rows

    def _base_rows(self, table):
        cols = [c[0] for c in self.tables[table]["columns"]]
        return [{table: dict(zip(cols, r))} for r in self.tables[table]["rows"]]

    def _apply_join(self, rows, join, cache):
        table = join["table"]
        if table not in self.tables:
            raise KeyError(f"no such table: {table}")
        cols = [c[0] for c in self.tables[table]["columns"]]
        cand_rows = self.tables[table]["rows"]
        out = []
        for r in rows:
            matched = False
            for cand in cand_rows:
                merged = dict(r)
                merged[table] = dict(zip(cols, cand))
                env = Env(row=merged, rows=None, db=self, cache=cache)
                if eval_expr(join["on"], env) is True:
                    out.append(merged)
                    matched = True
            if join["kind"] == "LEFT" and not matched:
                merged = dict(r)
                merged[table] = {c: None for c in cols}
                out.append(merged)
        return out

    def _group_rows(self, rows, group_by, cache):
        if not group_by:
            return [(None, rows)]
        groups = {}
        order = []
        for r in rows:
            env = Env(row=r, rows=None, db=self, cache=cache)
            key = tuple(eval_expr(g, env) for g in group_by)
            if key not in groups:
                groups[key] = []
                order.append(key)
            groups[key].append(r)
        return [(k, groups[k]) for k in order]

    def _project(self, columns, env, tables_order):
        out = []
        for expr, _alias in columns:
            if expr[0] == "star":
                for t in tables_order:
                    cols = [c[0] for c in self.tables[t]["columns"]]
                    tab = env.row.get(t)
                    for c in cols:
                        out.append(tab[c] if tab is not None else None)
            elif expr[0] == "tablestar":
                t = expr[1]
                cols = [c[0] for c in self.tables[t]["columns"]]
                tab = env.row.get(t)
                for c in cols:
                    out.append(tab[c] if tab is not None else None)
            else:
                out.append(eval_expr(expr, env))
        return tuple(out)


# -- type / comparison helpers ----------------------------------------

def _affinity_of(decl):
    d = decl.upper()
    if "INT" in d:
        return "INTEGER"
    if "CHAR" in d or "CLOB" in d or "TEXT" in d:
        return "TEXT"
    if "REAL" in d or "FLOA" in d or "DOUB" in d:
        return "REAL"
    if "BLOB" in d or d == "":
        return "BLOB"
    return "NUMERIC"


def _type_class(v):
    if isinstance(v, (int, float)):
        return 0  # numeric
    if isinstance(v, str):
        return 1  # text
    return 2  # other (blob)


def cmp3(a, b):
    """Type-aware 3-way compare for two non-NULL SQL values."""
    ca, cb = _type_class(a), _type_class(b)
    if ca != cb:
        return -1 if ca < cb else 1
    if a < b:
        return -1
    if a > b:
        return 1
    return 0


def _compare_nullable(a, b):
    if a is None and b is None:
        return 0
    if a is None:
        return -1
    if b is None:
        return 1
    return cmp3(a, b)


def _and3(a, b):
    if a is False or b is False:
        return False
    if a is None or b is None:
        return None
    return True


def _or3(a, b):
    if a is True or b is True:
        return True
    if a is None or b is None:
        return None
    return False


def _not3(a):
    return None if a is None else (not a)


def _sql_div(a, b):
    if b == 0:
        return None
    if isinstance(a, int) and isinstance(b, int):
        q = abs(a) // abs(b)
        if (a < 0) != (b < 0):
            q = -q
        return q
    return a / b


def _sql_mod(a, b):
    if b == 0:
        return None
    if isinstance(a, int) and isinstance(b, int):
        r = abs(a) % abs(b)
        return -r if a < 0 else r
    return float(a) % float(b)


def _arith(op, a, b):
    if op == "+":
        return a + b
    if op == "-":
        return a - b
    if op == "*":
        return a * b
    if op == "/":
        return _sql_div(a, b)
    if op == "%":
        return _sql_mod(a, b)
    raise NotImplementedError(f"unknown arithmetic operator {op}")


def _cmp_op(op, a, b):
    r = cmp3(a, b)
    if op == "=":
        return r == 0
    if op in ("!=", "<>"):
        return r != 0
    if op == "<":
        return r < 0
    if op == "<=":
        return r <= 0
    if op == ">":
        return r > 0
    if op == ">=":
        return r >= 0
    raise NotImplementedError(f"unknown comparison operator {op}")


def _to_text(v):
    if isinstance(v, str):
        return v
    if isinstance(v, float) and v == int(v):
        return str(int(v))
    return str(v)


def _like_to_regex(pattern):
    out = ["^"]
    for ch in pattern:
        if ch == "%":
            out.append(".*")
        elif ch == "_":
            out.append(".")
        else:
            out.append(re.escape(ch))
    out.append("$")
    return "".join(out)


def _like_match(value, pattern):
    return re.match(_like_to_regex(pattern), value, re.IGNORECASE | re.DOTALL) is not None


# -- column resolution --------------------------------------------------

def _resolve_col(env, table, name):
    if table is not None:
        if table not in env.row:
            raise KeyError(f"no such table: {table}")
        tab = env.row[table]
        if name not in tab:
            raise KeyError(f"no such column: {table}.{name}")
        return tab[name]
    matches = [t for t in env.row if name in env.row[t]]
    if not matches:
        raise KeyError(f"no such column: {name}")
    return env.row[matches[0]][name]


def _row_env(row, env):
    return Env(row=row, rows=None, db=env.db, cache=env.cache)


# -- aggregates -------------------------------------------------------

def _eval_aggregate(fname, args, distinct, env):
    rows = env.rows
    if fname == "COUNT":
        arg = args[0]
        if arg[0] == "star":
            return len(rows)
        vals = [eval_expr(arg, _row_env(r, env)) for r in rows]
        vals = [v for v in vals if v is not None]
        if distinct:
            vals = list(dict.fromkeys(vals))
        return len(vals)

    arg = args[0]
    vals = [eval_expr(arg, _row_env(r, env)) for r in rows]
    vals = [v for v in vals if v is not None]
    if distinct:
        vals = list(dict.fromkeys(vals))
    if fname == "SUM":
        return sum(vals) if vals else None
    if fname == "AVG":
        return (sum(vals) / len(vals)) if vals else None
    if fname == "MIN":
        return min(vals) if vals else None
    if fname == "MAX":
        return max(vals) if vals else None
    raise NotImplementedError(f"unknown aggregate function {fname}")


def _eval_subquery_list(select_ast, env):
    key = id(select_ast)
    if key in env.cache:
        return env.cache[key]
    result_rows = env.db._select(select_ast, env.cache)
    vals = [r[0] for r in result_rows]
    env.cache[key] = vals
    return vals


# -- generic expression eval --------------------------------------------

def eval_expr(expr, env):
    tag = expr[0]

    if tag == "lit":
        return expr[1]
    if tag == "col":
        return _resolve_col(env, expr[1], expr[2])
    if tag == "paren":
        return eval_expr(expr[1], env)
    if tag == "neg":
        v = eval_expr(expr[1], env)
        return None if v is None else -v
    if tag == "arith":
        a = eval_expr(expr[2], env)
        b = eval_expr(expr[3], env)
        if a is None or b is None:
            return None
        return _arith(expr[1], a, b)
    if tag == "concat":
        a = eval_expr(expr[1], env)
        b = eval_expr(expr[2], env)
        if a is None or b is None:
            return None
        return _to_text(a) + _to_text(b)
    if tag == "cmp":
        a = eval_expr(expr[2], env)
        b = eval_expr(expr[3], env)
        if a is None or b is None:
            return None
        return _cmp_op(expr[1], a, b)
    if tag == "eq":  # non-NULL IS comparison
        a = eval_expr(expr[1], env)
        b = eval_expr(expr[2], env)
        if a is None and b is None:
            return True
        if a is None or b is None:
            return False
        return cmp3(a, b) == 0
    if tag == "and":
        return _and3(eval_expr(expr[1], env), eval_expr(expr[2], env))
    if tag == "or":
        return _or3(eval_expr(expr[1], env), eval_expr(expr[2], env))
    if tag == "not":
        return _not3(eval_expr(expr[1], env))
    if tag == "isnull":
        v = eval_expr(expr[1], env)
        result = v is None
        return (not result) if expr[2] else result
    if tag == "like":
        val = eval_expr(expr[1], env)
        pat = eval_expr(expr[2], env)
        if val is None or pat is None:
            return None
        result = _like_match(_to_text(val), _to_text(pat))
        return (not result) if expr[3] else result
    if tag == "between":
        v = eval_expr(expr[1], env)
        lo = eval_expr(expr[2], env)
        hi = eval_expr(expr[3], env)
        negate = expr[4]
        geq = None if (v is None or lo is None) else cmp3(v, lo) >= 0
        leq = None if (v is None or hi is None) else cmp3(v, hi) <= 0
        result = _and3(geq, leq)
        return _not3(result) if negate else result
    if tag == "in":
        left, items, negate = expr[1], expr[2], expr[3]
        v = eval_expr(left, env)
        if items[0] == "list":
            vals = [eval_expr(it, env) for it in items[1]]
        else:
            vals = _eval_subquery_list(items[1], env)
        if v is None:
            result = None
        else:
            has_null = any(x is None for x in vals)
            matched = any(x is not None and cmp3(v, x) == 0 for x in vals)
            if matched:
                result = True
            elif has_null:
                result = None
            else:
                result = False
        return _not3(result) if negate else result
    if tag == "case":
        base, whens, else_expr = expr[1], expr[2], expr[3]
        base_val = eval_expr(base, env) if base is not None else None
        for cond, res in whens:
            if base is not None:
                cv = eval_expr(cond, env)
                matched = cv is not None and base_val is not None and cmp3(base_val, cv) == 0
            else:
                matched = eval_expr(cond, env) is True
            if matched:
                return eval_expr(res, env)
        return eval_expr(else_expr, env) if else_expr is not None else None
    if tag == "call":
        fname, args, distinct = expr[1], expr[2], expr[3]
        if fname in AGG_FUNCS:
            if env.rows is None:
                raise ValueError(f"aggregate {fname} used outside an aggregate query")
            return _eval_aggregate(fname, args, distinct, env)
        raise NotImplementedError(f"unknown function: {fname}")
    if tag == "subquery":
        vals = _eval_subquery_list(expr[1], env)
        return vals[0] if vals else None
    if tag in ("star", "tablestar"):
        raise ValueError(f"{tag} is not valid in this expression position")

    raise NotImplementedError(f"unhandled expression node: {tag}")


def _contains_agg(node):
    if isinstance(node, tuple):
        if node and isinstance(node[0], str):
            tag = node[0]
            if tag == "call" and node[1] in AGG_FUNCS:
                return True
            if tag == "subquery":
                return False
            return any(_contains_agg(e) for e in node[1:])
        return any(_contains_agg(e) for e in node)
    if isinstance(node, list):
        return any(_contains_agg(e) for e in node)
    return False


def _contains_agg_list(columns):
    return any(_contains_agg(expr) for expr, _alias in columns)
