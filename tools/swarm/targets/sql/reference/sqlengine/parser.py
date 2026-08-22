"""Recursive-descent parser producing an AST for the supported SQL subset.

Expression precedence, low to high:

    OR
    AND
    NOT (unary)
    comparison  (= != <> < <= > >= IS [NOT] NULL  [NOT] IN  [NOT] LIKE  [NOT] BETWEEN)
    ||          (string concatenation)
    + -
    * /
    unary - +
    primary     (literal, column ref, function call, (expr), (subquery), CASE)
"""

from .tokenizer import tokenize

AGG_FUNCS = {"COUNT", "SUM", "AVG", "MIN", "MAX"}


class ParseError(ValueError):
    pass


class Parser:
    def __init__(self, tokens):
        self.toks = tokens
        self.i = 0

    @classmethod
    def parse_sql(cls, sql):
        p = cls(tokenize(sql))
        stmt = p.parse_statement()
        p.expect("EOF")
        return stmt

    # -- token plumbing -----------------------------------------------
    def peek(self, k=0):
        return self.toks[self.i + k]

    def at(self, *kinds):
        return self.peek().kind in kinds

    def advance(self):
        t = self.toks[self.i]
        self.i += 1
        return t

    def expect(self, kind):
        t = self.peek()
        if t.kind != kind:
            raise ParseError(f"expected {kind}, got {t.kind} ({t.value!r}) at {t.pos}")
        return self.advance()

    # -- statements -----------------------------------------------------
    def parse_statement(self):
        if self.at("CREATE"):
            return self.parse_create()
        if self.at("INSERT"):
            return self.parse_insert()
        if self.at("SELECT"):
            return self.parse_select()
        t = self.peek()
        raise ParseError(f"unsupported statement starting at {t.kind} ({t.value!r})")

    def parse_create(self):
        self.expect("CREATE")
        self.expect("TABLE")
        name = self.expect("ID").value
        self.expect("(")
        cols = []
        while True:
            cname = self.expect("ID").value
            ctype = ""
            while not self.at(",", ")"):
                ctype += " " + self.advance().value
            cols.append((cname, ctype.strip()))
            if self.at(","):
                self.advance()
                continue
            break
        self.expect(")")
        return {"type": "create", "table": name, "columns": cols}

    def parse_insert(self):
        self.expect("INSERT")
        self.expect("INTO")
        name = self.expect("ID").value
        self.expect("VALUES")
        rows = [self.parse_value_tuple()]
        while self.at(","):
            self.advance()
            rows.append(self.parse_value_tuple())
        return {"type": "insert", "table": name, "rows": rows}

    def parse_value_tuple(self):
        self.expect("(")
        vals = [self.parse_expr()]
        while self.at(","):
            self.advance()
            vals.append(self.parse_expr())
        self.expect(")")
        return vals

    def parse_select(self):
        self.expect("SELECT")
        distinct = False
        if self.at("DISTINCT"):
            self.advance()
            distinct = True
        columns = [self.parse_select_item()]
        while self.at(","):
            self.advance()
            columns.append(self.parse_select_item())

        self.expect("FROM")
        from_table = self.expect("ID").value

        joins = []
        while self.at("INNER", "LEFT", "JOIN"):
            if self.at("INNER"):
                self.advance()
                kind = "INNER"
            elif self.at("LEFT"):
                self.advance()
                kind = "LEFT"
            else:
                kind = "INNER"
            self.expect("JOIN")
            table = self.expect("ID").value
            self.expect("ON")
            on = self.parse_expr()
            joins.append({"kind": kind, "table": table, "on": on})

        where = None
        if self.at("WHERE"):
            self.advance()
            where = self.parse_expr()

        group_by = []
        if self.at("GROUP"):
            self.advance()
            self.expect("BY")
            group_by.append(self.parse_expr())
            while self.at(","):
                self.advance()
                group_by.append(self.parse_expr())

        having = None
        if self.at("HAVING"):
            self.advance()
            having = self.parse_expr()

        order_by = []
        if self.at("ORDER"):
            self.advance()
            self.expect("BY")
            order_by.append(self.parse_order_item())
            while self.at(","):
                self.advance()
                order_by.append(self.parse_order_item())

        limit = None
        offset = None
        if self.at("LIMIT"):
            self.advance()
            limit = self.parse_expr()
            if self.at("OFFSET"):
                self.advance()
                offset = self.parse_expr()

        return {
            "type": "select",
            "distinct": distinct,
            "columns": columns,
            "from": from_table,
            "joins": joins,
            "where": where,
            "group_by": group_by,
            "having": having,
            "order_by": order_by,
            "limit": limit,
            "offset": offset,
        }

    def parse_select_item(self):
        if self.at("*"):
            self.advance()
            return (("star",), None)
        if self.at("ID") and self.peek(1).kind == "." and self.peek(2).kind == "*":
            table = self.advance().value
            self.advance()
            self.advance()
            return (("tablestar", table), None)
        expr = self.parse_expr()
        alias = None
        if self.at("AS"):
            self.advance()
            alias = self.expect("ID").value
        elif self.at("ID"):
            alias = self.advance().value
        return (expr, alias)

    def parse_order_item(self):
        expr = self.parse_expr()
        direction = "ASC"
        if self.at("ASC"):
            self.advance()
        elif self.at("DESC"):
            self.advance()
            direction = "DESC"
        return (expr, direction)

    # -- expressions ------------------------------------------------
    def parse_expr(self):
        return self.parse_or()

    def parse_or(self):
        left = self.parse_and()
        while self.at("OR"):
            self.advance()
            right = self.parse_and()
            left = ("or", left, right)
        return left

    def parse_and(self):
        left = self.parse_not()
        while self.at("AND"):
            self.advance()
            right = self.parse_not()
            left = ("and", left, right)
        return left

    def parse_not(self):
        if self.at("NOT"):
            self.advance()
            return ("not", self.parse_not())
        return self.parse_comparison()

    def parse_comparison(self):
        left = self.parse_concat()

        negate = False
        if self.at("NOT") and self.peek(1).kind in ("IN", "LIKE", "BETWEEN"):
            self.advance()
            negate = True

        if self.at("IS"):
            self.advance()
            neg = False
            if self.at("NOT"):
                self.advance()
                neg = True
            if self.at("NULL"):
                self.advance()
                return ("isnull", left, neg)
            right = self.parse_concat()
            node = ("eq", left, right)
            return ("not", node) if neg else node

        if self.at("IN"):
            self.advance()
            self.expect("(")
            if self.at("SELECT"):
                sub = self.parse_select()
                items = ("subquery", sub)
            else:
                items = []
                if not self.at(")"):
                    items.append(self.parse_expr())
                    while self.at(","):
                        self.advance()
                        items.append(self.parse_expr())
                items = ("list", items)
            self.expect(")")
            return ("in", left, items, negate)

        if self.at("LIKE"):
            self.advance()
            pattern = self.parse_concat()
            return ("like", left, pattern, negate)

        if self.at("BETWEEN"):
            self.advance()
            lo = self.parse_concat()
            self.expect("AND")
            hi = self.parse_concat()
            return ("between", left, lo, hi, negate)

        if negate:
            raise ParseError("NOT must be followed by IN, LIKE, or BETWEEN here")

        ops = {"=", "!=", "<>", "<", "<=", ">", ">="}
        if self.peek().kind in ops:
            op = self.advance().kind
            right = self.parse_concat()
            return ("cmp", op, left, right)

        return left

    def parse_concat(self):
        left = self.parse_additive()
        while self.at("||"):
            self.advance()
            right = self.parse_additive()
            left = ("concat", left, right)
        return left

    def parse_additive(self):
        left = self.parse_multiplicative()
        while self.at("+", "-"):
            op = self.advance().kind
            right = self.parse_multiplicative()
            left = ("arith", op, left, right)
        return left

    def parse_multiplicative(self):
        left = self.parse_unary()
        while self.at("*", "/", "%"):
            op = self.advance().kind
            right = self.parse_unary()
            left = ("arith", op, left, right)
        return left

    def parse_unary(self):
        if self.at("-", "+"):
            op = self.advance().kind
            operand = self.parse_unary()
            return ("neg", operand) if op == "-" else operand
        return self.parse_primary()

    def parse_primary(self):
        t = self.peek()
        if t.kind == "NUM":
            self.advance()
            text = t.value
            if "." in text or "e" in text or "E" in text:
                return ("lit", float(text))
            return ("lit", int(text))
        if t.kind == "STR":
            self.advance()
            return ("lit", t.value)
        if t.kind == "NULL":
            self.advance()
            return ("lit", None)
        if t.kind == "TRUE":
            self.advance()
            return ("lit", 1)
        if t.kind == "FALSE":
            self.advance()
            return ("lit", 0)
        if t.kind == "CASE":
            return self.parse_case()
        if t.kind == "(":
            self.advance()
            if self.at("SELECT"):
                sub = self.parse_select()
                self.expect(")")
                return ("subquery", sub)
            expr = self.parse_expr()
            self.expect(")")
            return ("paren", expr)
        if t.kind == "ID":
            name = self.advance().value
            if self.at("("):
                self.advance()
                distinct = False
                if self.at("DISTINCT"):
                    self.advance()
                    distinct = True
                if self.at("*"):
                    self.advance()
                    args = [("star",)]
                else:
                    args = []
                    if not self.at(")"):
                        args.append(self.parse_expr())
                        while self.at(","):
                            self.advance()
                            args.append(self.parse_expr())
                self.expect(")")
                return ("call", name.upper(), args, distinct)
            if self.at("."):
                self.advance()
                if self.at("*"):
                    self.advance()
                    return ("tablestar", name)
                col = self.expect("ID").value
                return ("col", name, col)
            return ("col", None, name)
        raise ParseError(f"unexpected token {t.kind} ({t.value!r}) at {t.pos}")

    def parse_case(self):
        self.expect("CASE")
        base = None
        if not self.at("WHEN"):
            base = self.parse_expr()
        whens = []
        while self.at("WHEN"):
            self.advance()
            cond = self.parse_expr()
            self.expect("THEN")
            result = self.parse_expr()
            whens.append((cond, result))
        else_expr = None
        if self.at("ELSE"):
            self.advance()
            else_expr = self.parse_expr()
        self.expect("END")
        return ("case", base, whens, else_expr)
