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

---

# Result (2026-08-30): inconclusive, and why

Four runs were attempted. Three completed; the fourth died to an infrastructure
failure. The trial **cannot answer its own question**, for a reason that has
nothing to do with the briefs.

|  | n | final | plateau | coverage | dupes | minutes |
|---|---|---|---|---|---|---|
| control | 2 | 100.0 | 1.5 | 1.0 | 0.0 | 20 |
| diverse | 1 | 100.0 | 4.0 | 1.0 | 0.0 | 40 |

## The instrument saturated

Control run 1 took the synth target from its 700,004-op baseline to **60,004 ops
— the scorer's declared full-marks floor — in ten minutes**, and every completed
run finished at 100. `decode.parse` sat at 60,000 and everything else at 4: the
fleet found the optimal decomposition, quickly, in both arms.

So "diverse scored no better" is a fact about the scale, not about the briefs.
The comparison could not have come out any other way.

This is the harness's own fitness-function rule turned on itself. *No reachable
ceiling* is written into `tools/swarm/README.md` as a lesson from earlier
targets, and the synth scorer violates it: `FLOOR = 60_000` is not merely
generous, it is **attainable**, and six agents attain it in ten minutes.

Class coverage failed for the same underlying reason. Both arms scored 1 — one
site moved — because the fixture has essentially **one lever**. A metric designed
to detect whether different priors spread the search cannot do so on a fixture
with nothing to spread across.

## What the run did establish

Not about diversity, but worth recording:

- `SWARM_STRATEGIES` works. Verified live: builders received `measure`,
  `structure` and `algorithm`. Every previous trial ran six copies of one brief,
  so this is the first time the variable has ever been exercised.
- The arms behave differently even at the same score. Control converged with one
  approved solution and no open critiques; diverse ran twice as long with three
  approved and six critiques open. Something is different — the instrument just
  cannot say whether it is better.

## Two confounds, both recorded rather than argued away

**Duration is an outcome, not a constant.** The watchdog stops a fleet once
nothing is open and nothing awaits review, so control got 20 minutes and diverse
40. Coverage and duplicates both accumulate with time, so the longer arm is
flattered on them. The pre-registration said "same duration" and the harness
quietly did not deliver it.

**The fourth run was lost to an expired credential**, not to anything about the
briefs. It is excluded, which leaves diverse at n=1 — and n=1 was already too
few.

## What would make this trial answerable

1. **A target whose optimum is out of reach inside a run.** Either a harder
   fixture with several independent levers — so class coverage has something to
   measure — or a longer budget against a target that does not top out.
2. **Equal durations.** Cap both arms at a fixed wall-clock, or the watchdog
   turns a behavioural difference into a time difference.
3. **More runs.** Agent runs are noisy; n=2 was optimistic even with a working
   instrument.

## The most valuable thing the run produced

A data-loss bug in `jjj fetch`, found because the credential expiry made the
escalation path fire for real.

The refresher raised an escalation and pushed it. It reached the remote. The
watchdog still reported `escalations=0`, and the fleet spent the rest of its
deadline failing every turn — the exact outage `jjj escalate` was built to
prevent, defeated one layer down.

Fetch enumerated metadata bookmarks with `heads(...)`, which drops any ref that
is an ancestor of another, on the reasoning that a descendant already contains
its ancestor's content. For these bookmarks that is false: a metadata commit is a
snapshot of one actor's whole tree, and `push` builds it with `jj new <heads...>`
and then copies the pusher's own files over the merged result. A pod that had
fetched the *refs* but not the *content* pushed a commit descending from the
escalation while carrying the shard from before it. Ancestry is reachability, not
subsumption.

That bug had been latent since per-pod bookmarks were introduced. No amount of
reading found it; a real credential expiry did. Which is the same lesson as the
plan's through-line — *not "is it implemented" but "did something use it end to
end"* — arriving from the other direction.


---

# What changed, so the re-run can answer the question

All three fixes named above are in.

## A target with five levers (`synth2`)

`tools/swarm/targets/synth2/` is `synth` with the flaw removed. The cost is
spread across five *independent* inefficiencies, each wanting a different kind
of change:

| lever | the waste | the insight |
|---|---|---|
| 1 | the decoder runs once per stage, not once per record | control flow |
| 2 | `stage_count` scans a tuple where a set would do | asymptotics |
| 3 | `stage_sum` rebuilds a constant table inside its loop | hoisting |
| 4 | `stage_group` accumulates a list it only ever counts | representation |
| 5 | `stage_filter` normalises four fields to compare one | doing less |

Fixing one leaves the others untouched — verified by applying each in isolation:

```
baseline                          18
L1 fuse the four passes           26
L2 set instead of scan            22
L3 hoist the constant table       21
L4 count, do not accumulate       20
L5 normalise one field, not four  20
all five, fused                   83
```

Ten sites carry 20,000+ operations at baseline, the largest 22% of the total, so
**class coverage finally has something to measure**. `groundtruth.sh` asserts
both properties — no site over 40%, at least five sites with real cost — because
the flaw it is guarding against is precisely the one that wasted the first trial.

## A ceiling that is out of reach by construction

`decode.parse` charges 3 operations for each of 20,000 records and no correct
answer can skip a record, so **60,000 operations is a hard lower bound**. A fully
optimised reference with all five levers pulled measures 100,004 and scores 83.
Full marks needs 50,000. It cannot be reached.

The reference lives in `reference/`, which `seed.sh` deliberately does not copy —
it holds a worked answer. `reference/ceiling.sh` asserts the property from the
repository side: an optimised tree must score under 100, the range must span at
least 40 points, and the optimum must sit above the full-marks floor.

`score.sh` also refuses a modified `fixture/ops.py`. The meter is not part of the
program under optimisation: editing it does not make anything cheaper, it makes
the measurement lie, and the fitness function is the one artifact a swarm cannot
critique.

## Equal durations

The trial no longer passes `--stop-when-done`. The watchdog ends a run when its
queue empties, which made length an outcome of the arm — 21 minutes against 60 —
and flattered the longer arm on the two metrics that accumulate with time. Both
arms now run to the same deadline.

A run whose host credential dies is marked `invalid` in `results.tsv` and
excluded from the means, rather than dropped. A missing row reads as "never ran",
which is a different fact.
