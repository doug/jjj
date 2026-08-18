---
title: "Ranking: authored order, sized gaps, and aggregation"
description: "A per-user ordering with sized gaps drives the aggregate ranking; two users combine with equal influence"
covers:
  - "rank show with no ordering authored"
  - "A single user's order determines the aggregate"
  - "An XL gap sinks the item below it"
  - "rank show --by-user renders gaps"
  - "Two users' orderings aggregate with equal budget"
  - "rank show --json"
tags: [ranking, gaps, aggregation]
---

# Ranking

Orderings are authored in the TUI (`jjj ui`, then Shift+K/J to nudge and `p` to
cycle the gap below an item). On disk each one is a
`rankings/{milestone}/{user}.json` file, which is what this journey writes
directly so the maths is testable without a terminal.

Two signals combine: **order** (direction) and **gap** (intensity). Walking a
list top-to-bottom, each gap adds depth — 1/2/4/8/16 for none/S/M/L/XL — and an
item at cumulative depth `d` earns `1/(1+d)`. Weights are then scaled so every
user's points sum to the same budget, which is what makes a five-item list and a
fifty-item list count equally.

```jjj:setup
init
```

```jjj:setup
milestone new "Q1"
```

```jjj:setup
problem new "Alpha" --milestone Q1 --force
```

```jjj:setup
problem new "Beta" --milestone Q1 --force
```

```jjj:setup
problem new "Gamma" --milestone Q1 --force
```

## Nothing Is Ranked Until Someone Ranks It

An unranked milestone says so rather than inventing an order:

```jjj
rank show Q1
> No rankings yet
```

```jjj
rank show Q1 --json
> []
```

## Authoring an Ordering

Capture the ids the ranking file refers to:

```jjj:setup
milestone show Q1 --json
>= MILESTONE "id": "([0-9a-f-]{36})"
```

```jjj:setup
problem show "Alpha" --json
>= ALPHA "id": "([0-9a-f-]{36})"
```

```jjj:setup
problem show "Beta" --json
>= BETA "id": "([0-9a-f-]{36})"
```

```jjj:setup
problem show "Gamma" --json
>= GAMMA "id": "([0-9a-f-]{36})"
```

Rank Gamma first, then Alpha, then Beta — and put an `XL` gap *below* Alpha,
the "Beta must not win" signal:

```shell:setup
mkdir -p .jj/jjj-meta/rankings/$MILESTONE
cat > ".jj/jjj-meta/rankings/$MILESTONE/Test User.json" << JSONEOF
{
  "order": ["$GAMMA", "$ALPHA", "$BETA"],
  "gaps": {"$ALPHA": "XL"},
  "updated_at": "2026-08-18T00:00:00Z"
}
JSONEOF
```

The aggregate follows the authored order:

```jjj
rank show Q1
> Rankings for milestone: Q1
>~ 1\s+Gamma
>~ 2\s+Alpha
>~ 3\s+Beta
```

## The Gap Is Intensity, Not Just Order

Beta sits one slot below Alpha but scores an order of magnitude lower, because
the `XL` gap pushed it far down the depth scale. Order alone could not express
that:

```jjj
rank show Q1 --json
>~ "title": "Gamma"[^}]*"score": 6[0-9]
>~ "title": "Beta"[^}]*"score": [0-9]\.
```

The per-user view shows where the gap was authored:

```jjj
rank show Q1 --by-user
> --- Test User ---
>~ Alpha\s+XL
```

## Two Users Aggregate With Equal Influence

A second user ranks the same three problems in the opposite order, with no gaps:

```shell:setup
cat > ".jj/jjj-meta/rankings/$MILESTONE/bob.json" << JSONEOF
{
  "order": ["$BETA", "$ALPHA", "$GAMMA"],
  "gaps": {},
  "updated_at": "2026-08-18T00:00:00Z"
}
JSONEOF
```

Both users now appear, and every problem is backed by two voters:

```jjj
rank show Q1 --by-user
> --- Test User ---
> --- bob ---
```

```jjj
rank show Q1
>~ Alpha\s+[0-9.]+\s+2
>~ Gamma\s+[0-9.]+\s+2
>~ Beta\s+[0-9.]+\s+2
```

Gamma still wins. Being the top of one list outweighs being the bottom of
another, because the harmonic weighting concentrates points at the top — that
asymmetry is deliberate, so a strongly-held first choice is not cancelled by
someone else's indifference:

```jjj
rank show Q1 --json
>~ "rank": 1,[^}]*"title": "Gamma"
```

Beta is the interesting one. It is bob's *first* choice, yet it finishes last —
because the `XL` gap above it is a stronger statement than a plain top slot.
Alpha, unremarkable on both lists, edges past it:

```jjj
rank show Q1 --json
>~ "rank": 3,[^}]*"title": "Beta"
```
