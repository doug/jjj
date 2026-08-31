"""A deterministic cost counter.

Cost is counted, never timed. Six agents share a machine, and every timing-based
fitness function in this harness has had to be rewritten after contention made
an unchanged tree score anywhere from 0 to 28. An operation count cannot drift.
"""

_COUNT = {}


def tick(name, n=1):
    """Charge `n` operations to `name`."""
    _COUNT[name] = _COUNT.get(name, 0) + n


def reset():
    _COUNT.clear()


def total():
    return sum(_COUNT.values())


def by_site():
    return dict(sorted(_COUNT.items(), key=lambda kv: -kv[1]))
