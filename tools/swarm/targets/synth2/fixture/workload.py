"""A fixed workload. Deterministic, so the same tree always costs the same."""

import random


def records(n=20000, seed=20260831):
    rng = random.Random(seed)
    # Deliberately irregular, so nothing incidental about the fixture can be
    # exploited as if it were a guarantee.
    #
    # An earlier fixture used single-character kinds and ids nothing ever read.
    # An agent optimised by skipping the first delimiter scan — KIND is one
    # character, so its position is known — and scored better for it. Another
    # agent reverted that as "a fixture-specific assumption not backed by spec",
    # which was the right call and cost the fleet fifteen points. A benchmark
    # that pays for overfitting is asking to be gamed, and punishing the agent
    # who refuses is worse than not measuring at all.
    #
    # Kinds are mixed-case on purpose: one stage has to compare them
    # case-insensitively, so normalising *that one field* is real work while
    # normalising all four is not.
    kinds = ("a", "A", "bb", "Bb", "ccc", "d", "D")
    owners = ("ana", "bo", "cy", "di", "ed", "fi", "gu")
    return [
        f"rec-{rng.randrange(10**6):06d}|{rng.choice(kinds)}"
        f"|{rng.randrange(1, 100)}|{rng.choice(owners)}"
        for _ in range(n)
    ]
