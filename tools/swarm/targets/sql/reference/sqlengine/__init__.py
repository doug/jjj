"""A SQL engine in pure Python, checked against SQLite for correctness.

The contract the harness depends on, and the only thing that must not change:

    from sqlengine import Database
    db = Database()
    db.execute("CREATE TABLE t (a INTEGER, b TEXT)")   # -> None
    db.execute("INSERT INTO t VALUES (1, 'x')")        # -> None
    db.execute("SELECT * FROM t")                      # -> [(1, 'x')]

`execute` returns a list of tuples for a SELECT and None otherwise, and raises
on anything it cannot answer.
"""

from .tokenizer import tokenize
from .parser import Parser
from .engine import Database

__all__ = ["Database", "tokenize", "Parser"]
