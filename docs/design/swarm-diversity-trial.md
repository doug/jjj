---
title: Does perspective diversity help a swarm?
description: A pre-registered A/B trial of homogeneous versus heterogeneous agent briefs
---

# Does perspective diversity help a swarm?

The argument for running several agents is not parallel effort — one agent given
more turns would do that, and more cheaply. It is parallel **perspective**:
several agents attacking a problem from genuinely different directions should
get stuck less often, because only one of them has to find a way through.

Every trial so far has failed to test this. Builders ran an identical prompt, so
six agents were six copies of one search, and any advantage measured was
parallelism rather than diversity.

## The two arms

Identical in every respect except the briefs. Same target, same seed, same
duration, same fleet shape, run sequentially on an otherwise idle machine
because both measure latency.

| | builders |
|---|---|
| **control** | one shared brief (`SWARM_STRATEGIES` unset) |
| **diverse** | `measure`, `structure`, `algorithm` — one each (`SWARM_STRATEGIES=1`) |

Critics are unchanged in both. The question is about how work is *found*, not
how it is judged.

## What is measured

Recorded before running, so the result cannot be chosen afterwards.

1. **Final score on shared `main`**, scored independently after the run rather
   than taken from an agent's own report.
2. **Longest plateau** — the longest stretch with no improvement in the shared
   score. This is the hypothesis stated directly: diversity should shorten the
   time spent stuck, even if it does not raise the ceiling.
3. **Class coverage** — how many of the six scoring classes moved. Six copies of
   one search should concentrate; different priors should spread.
4. **Duplicate solutions** — solutions withdrawn as redundant with work already
   landed. Diversity should reduce collisions on the same idea.

## What would refute it

- Diverse scores the same or worse on (1) **and** has no shorter plateau on (2).
  Then the briefs are decoration and the advantage really is just parallelism.
- Diverse spreads across classes but ends lower. Then diversity trades depth for
  breadth, which is a real cost and worth saying plainly rather than presenting
  breadth as a win.

## What this trial cannot settle

One pair of runs on one target. Agent runs are noisy — the same configuration
twice would not produce the same number — so a small difference means nothing
here. Only a large effect is worth reporting, and even then it is one target's
worth of evidence, not a general claim about swarms.
