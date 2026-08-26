"""A fixed workload. Deterministic, so the same tree always costs the same."""

import random


def records(n=20000, seed=20260826):
    rng = random.Random(seed)
    kinds = ("a", "b", "c")
    owners = ("ana", "bo", "cy", "di", "ed")
    return [
        f"r{i}|{rng.choice(kinds)}|{rng.randrange(1, 100)}|{rng.choice(owners)}"
        for i in range(n)
    ]
