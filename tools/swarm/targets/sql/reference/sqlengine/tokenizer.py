"""Tokenizer for the SQL subset this engine implements."""

import re

KEYWORDS = {
    "SELECT", "FROM", "WHERE", "AND", "OR", "NOT", "IS", "NULL", "ORDER", "BY",
    "ASC", "DESC", "LIMIT", "OFFSET", "GROUP", "HAVING", "DISTINCT", "INNER",
    "LEFT", "JOIN", "ON", "IN", "BETWEEN", "CASE", "WHEN", "THEN", "ELSE",
    "END", "LIKE", "AS", "CREATE", "TABLE", "INSERT", "INTO", "VALUES",
    "TRUE", "FALSE",
}

TOKEN_RE = re.compile(r"""
    (?P<WS>\s+)
  | (?P<STR>'(?:[^']|'')*')
  | (?P<NUM>\d+\.\d+(?:[eE][+-]?\d+)?|\.\d+(?:[eE][+-]?\d+)?|\d+(?:[eE][+-]?\d+)?)
  | (?P<ID>[A-Za-z_][A-Za-z0-9_]*)
  | (?P<CONCAT>\|\|)
  | (?P<LE><=)
  | (?P<GE>>=)
  | (?P<NE>!=|<>)
  | (?P<OP>[=<>()\.,+\-*/%])
""", re.VERBOSE)


class Token:
    __slots__ = ("kind", "value", "pos")

    def __init__(self, kind, value, pos):
        self.kind = kind
        self.value = value
        self.pos = pos

    def __repr__(self):
        return f"Token({self.kind!r}, {self.value!r})"


def tokenize(sql):
    tokens = []
    i, n = 0, len(sql)
    while i < n:
        m = TOKEN_RE.match(sql, i)
        if not m:
            raise ValueError(f"cannot tokenize at position {i}: {sql[i:i + 20]!r}")
        i = m.end()
        kind = m.lastgroup
        text = m.group()
        if kind == "WS":
            continue
        if kind == "STR":
            tokens.append(Token("STR", text[1:-1].replace("''", "'"), m.start()))
        elif kind == "NUM":
            tokens.append(Token("NUM", text, m.start()))
        elif kind == "ID":
            up = text.upper()
            if up in KEYWORDS:
                tokens.append(Token(up, text, m.start()))
            else:
                tokens.append(Token("ID", text, m.start()))
        elif kind == "CONCAT":
            tokens.append(Token("||", "||", m.start()))
        elif kind == "LE":
            tokens.append(Token("<=", "<=", m.start()))
        elif kind == "GE":
            tokens.append(Token(">=", ">=", m.start()))
        elif kind == "NE":
            tokens.append(Token("!=", "!=", m.start()))
        elif kind == "OP":
            tokens.append(Token(text, text, m.start()))
        else:  # pragma: no cover
            raise ValueError(f"unhandled token kind {kind}")
    tokens.append(Token("EOF", "", n))
    return tokens
