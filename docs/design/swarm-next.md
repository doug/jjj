---
title: What to change next in the swarm, and why
description: A sequenced plan drawn from six trials and from what Firstmate does differently
---

# What to change next in the swarm

This plan comes from two sources: six swarm trials against four targets, and a
reading of [Firstmate](https://github.com/kunchenguid/firstmate), which solves a
similar problem with a different architecture. Each milestone below names the
evidence that motivates it, what would count as done, and what it costs.

The ordering is by *evidence strength divided by cost*, not by ambition. Two of
these are small and clearly right; two are experiments that could come back
negative; one is deliberately refused.

## Status

| | Milestone | State |
|---|---|---|
| M1 | Findings as a first-class kind | **shipped** — `jjj finding`, `--cites` on solutions and critiques |
| M2 | The swarm can ask for a human | **shipped** — `jjj escalate`, leads `status`, stops the fleet after a grace period |
| M3 | Observe only through jjj | **shipped** — `analyze.py` runs on any jjj repository; harness figures are labelled as such |
| M4 | Supervision that costs nothing while idle | **shipped** — ref-fingerprint skip, turn-end backstop |
| M5 | Test the diversity thesis | first run **inconclusive** — the target saturated. Instrument rebuilt (`synth2`, five levers, unreachable ceiling, equal durations); **re-run pending** |
| M6 | Routing only where contention is real | **shipped** — `jjj contention` |

Three things surfaced only by using the new paths end to end, and all are the
same failure this document opens with — a capability existing without a path to
it. `jjj contention` printed `rank move` commands that all shared a six-character
UUID7 prefix, so at most one of them could ever have run; `rank move` refused for
anyone without a prior ordering, which is precisely the person the advice is
aimed at; and `fetch` could bury an escalation behind a descendant push, which
defeated M2 one layer down and was found only when a credential expired during a
real run. None was visible from reading the code.

## The through-line

Six trials produced one repeated failure, in five distinct forms: **a capability
existed and the path to it did not.**

- `jjj rank` had `show` and no way to author an ordering outside the TUI
- targets seeded no milestone, so ranking had nowhere to live
- sub-problems did not inherit a milestone, so rankings never aggregated
- `claimed_at` lived on disk and was dropped by the cached read, so a claim's
  age was invisible to the caller deciding whether to respect it
- the merge gate ran `cargo build` on a Python workbench, so nothing ever merged

None were visible from reading the code. Each surfaced only when something tried
to use the whole path end to end. That is what preflight and the ground-truth
checks now exist for, and it is the standard every milestone below should be
held to: *not "is it implemented" but "did something use it end to end".*

## M1 — Scout work becomes a first-class kind

**The problem.** jjj models conjectures and refutations. It does not model
*evidence*, and investigations therefore arrive disguised as solutions:

    "Symbol-size breakdown of gallery.wasm: harfbuzz 20%, crypto/tls tax ~1MB"
    "Measure the 15MB: package/section breakdown + nm-on-wasm doesn't work"
    "Root cause found and documented; not fixed"
    "Confirm decode.parse's 120,004-op floor; document to prevent re-investigation"

Four of 21 solutions in one run, three of 25 in another. A Solution is a
conjecture attached to a change; an investigation is neither, so it is either
approved as though it were code or withdrawn as "not fixed" — and that last title
shows the fleet hand-rolling a workaround for the missing concept.

Firstmate splits this explicitly: **ship** tasks produce a change, **scout** tasks
*"produce knowledge in a report, used for investigation when unresolved
uncertainty could materially change what gets built"*, and a scout must leave a
self-contained report before its worktree is discarded.

**Why it fits.** Popper distinguishes a conjecture from the observations that
motivate it. jjj models the conjecture and not the observation. A finding is not
a fourth thing bolted on; it is the missing third.

**Deliverables.**

- `Finding` entity: `findings/{uuid}.md`, attached to a problem, with an author,
  a body, and no state machine beyond existing/superseded — a measurement is not
  approved, it is cited or contradicted.
- `jjj finding new <problem> "title" --body -`, `finding list`, `finding show`,
  all with `--json`.
- Sync, merge and cache: one row in `ENTITY_KINDS`, a `Persist` impl, a table.
- Critiques may cite a finding; `problem show` lists findings alongside solutions.
- Agent guidance: publish what you measured as a finding, not as a solution you
  then withdraw.

**Done when.** A trial produces findings, later solutions cite them, and no
solution in that run is withdrawn with a rationale of the form "documented, not
fixed". `analyze.py`'s Evidence section reports both halves — how many findings
say *how* they were measured, and how many are cited by later work — so a
filing cabinet nobody reads is distinguishable from evidence in use.

**Cost.** Bounded and almost entirely additive — the entity plumbing is generic
over a `(dir, singular)` table, so nothing existing needs restructuring. Roughly
the size of `commands/critique.rs` plus a model and a migration.

**Risk.** Low. The main one is scope creep into a state machine findings do not
need.

## M2 — The swarm can ask for a human

**The problem.** When the host OAuth session expired, the fleet failed 400
consecutive turns over 6.8 hours. Every container stayed up, the sampler kept
writing rows, and the score sat frozen. Nothing in the system could say *"I am
blocked on something only a person can fix."* It was found by reading logs.

The `AUTH_DEAD` marker added afterwards is a patch for one instance of a general
gap.

Firstmate states the contract in both directions. Escalate immediately for: work
ready for review, finished findings, a real blocker after the playbook is
exhausted, anything destructive or security-sensitive, a needed credential. Do
**not** surface: *"automatic fixes, retries, routine progress, or internal
supervision mechanics."*

**Deliverables.**

- `jjj escalate "<reason>" [--entity <id>]` emitting an `EscalationRaised` event
  and, until cleared, surfacing at the top of `jjj status` and `swarm.sh status`.
- `jjj escalate --clear <id>` for when the human has acted.
- An escalation contract in the agent brief, phrased as both a whitelist and a
  blacklist — the blacklist matters more, because an escalation channel that
  carries routine progress is one nobody reads.
- The harness treats an open escalation as a reason to stop the fleet rather than
  burn the remaining deadline.

**Done when.** A deliberately broken credential produces an escalation within one
turn, and `swarm.sh status` leads with it.

**Cost.** Small. An event type, a command, a status section.

**Risk.** Low, with one real failure mode: agents escalating routine things. The
blacklist and a cap on open escalations per agent are the mitigations.

## M3 — Observe the swarm only through jjj

**The problem.** I watch trials through `podman logs`, agent-local `score.sh`
runs, and a `jjj-invocations.jsonl` shim. All three are side channels, and every
misreading this session came through one:

- reported "0 failures" during a 90%-failure outage, from a field that happened
  to read zero
- reported three agents at 0 when they were at 73, from turn-*opening* scores
- reported "137% of solutions withdrawn", from counting calls against creations
- reported "40 lost a race, 1 refuted" when it was 5 and 35, from a classifier
  too coarse to tell duplication from selection on merit

Firstmate's rule is a good one: *"a secondmate's routed reply returns through
status or a document pointer, not by firstmate peeking into its chat."*

**Deliverables.**

- `analyze.py` derives every figure from jjj entities and the event log rather
  than the invocation shim. Where a figure cannot be derived that way, that is a
  gap in jjj's model and should be recorded as one rather than worked around.
- Keep the shim for debugging the harness; stop reporting from it.
- Every reported ratio states its numerator and denominator basis.

**Done when.** `analyze.py` runs against a plain jjj repository with no swarm
instrumentation and produces the same coordination figures.

**Cost.** Moderate. Some figures have no jjj-side representation today, which is
itself the finding.

**Risk.** Low, and it pays for itself by making the analysis work on human
projects rather than only on trials.

## M4 — Supervision that costs nothing while idle

**The problem.** One trial reached its ceiling at minute 35 and made 4,100
further jjj calls for no gain. Another sat at 100 for 80 minutes because the
watchdog needed more stillness than the run had left. Both are now partly fixed —
idle turns skip the model call, patience is capped — but supervision is still a
poll loop that runs whether or not anything happened.

Firstmate uses a bash watcher that *"sleeps on the fleet and wakes the first mate
only when something needs you"*, with harness Stop hooks for tokenless re-arm,
plus a *turn-end backstop* that blocks a stop when work is under way and
supervision is not live.

**Deliverables.**

- Watchdog and sampler wake on jjj metadata changes rather than a fixed interval.
- A turn-end backstop: an agent that would idle while work is unreviewed picks up
  the review instead of sleeping.

**Done when.** Idle jjj calls per hour fall by an order of magnitude with no loss
in time-to-converge.

**Cost.** Small to moderate.

**Risk.** Event-driven supervision that misses an event hangs the run. The poll
must remain as a slow backstop, not be replaced.

## M5 — Test the diversity thesis

**The problem.** The argument for a swarm is not parallel effort — one agent with
more turns achieves that, more cheaply. It is parallel *perspective*: several
agents attacking from genuinely different directions get stuck less often,
because only one of them has to find a way through.

That has never been tested here. `SWARM_STRATEGIES` exists, assigns each builder
a different prior, and has not been switched on in a single trial. Every run to
date measured six copies of one search.

**Deliverables.**

- `tools/swarm/diversity-trial.sh` — `run` executes both arms, `report` applies
  the pre-registered criteria. Runs are **interleaved** (control-1, diverse-1,
  control-2, diverse-2) rather than blocked: if the machine slows over the
  afternoon, blocking would hand the whole penalty to one arm and the difference
  would read as an effect of the briefs.
- Pre-registered metrics from `swarm-diversity-trial.md`: final score on shared
  `main` (scored independently, never from an agent's own report), longest
  plateau, class coverage against a pre-run baseline, and duplicate withdrawals
  using the classifier that distinguishes duplication from selection on merit.
- The refutation condition is *executed*, not interpreted: if diverse scores no
  better and has no shorter plateau, `report` prints REFUTED.
- Two runs per arm on the synthetic target, which converges in about 40 minutes
  and has a scorer that survived a ground-truth check.

**Done when.** Four runs are complete and the write-up states plainly whether the
briefs are load-bearing or decoration.

**Cost.** Roughly four hours of wall clock, little attention.

**Risk.** The honest one: it may come back negative, and n=2 per arm may not
separate the effect from noise. Both outcomes are worth having written down.

## M6 — Routing only where contention is real

**The problem, and the temptation.** Firstmate prevents duplicated work *by
construction*: one first mate routes each request, so two crewmates cannot pick
the same thing. That is a better answer than claims, proposals and staggering to
the problem this session spent most of its time on.

**Why it is refused as an architecture.** jjj's thesis is no server, offline
first, metadata that merges. A central router is precisely what it rejects. And
it would suppress the thing the swarm is for: under central assignment two agents
*cannot* independently attack one problem with rival ideas.

Duplication is therefore the *price* of decentralisation plus rivalry, not a
defect to eliminate. The last trial got duplicated effort to zero while keeping
rivalry, so the price is payable.

**The narrow version worth building.** The integrator is already a partial hub.
Give it routing authority only when the queue is contended — several agents on
one problem while others go untouched — expressed as a re-ranking rather than an
assignment, so it nudges the fleet without becoming the only path to work.

**Done when.** A trial with an artificially contended queue spreads within two
turns, without the integrator becoming a bottleneck when the queue is quiet.

**Cost.** Small, given the integrator exists.

**Risk.** Moderate: an integrator that routes can starve rivalry if it is too
eager. The re-ranking form is deliberately weaker than assignment for that reason.

## Not doing

- **Central routing as the architecture.** Above.
- **Critiques against problems.** A badly framed problem is the artefact most in
  need of review, and `critique new` takes a solution only. But it is a 129-site
  change across the model, cache, FTS and tests, and the cheaper approximations —
  `problem dissolve --rationale`, `problem duplicate --of` — now exist in the
  guidance and are being used. Revisit when there is evidence the approximations
  are insufficient, not before.
- **Ranking solutions.** Solutions are not scored; they survive criticism or they
  do not. Adding a ranking would import positive justification into a system
  built on refutation.

## Sequence

M1 and M2 first: both are small, both are backed by evidence from more than one
trial, and both add vocabulary the later milestones want to measure. M3 next,
because it makes every subsequent claim about the swarm derivable from jjj rather
than from a shim. M4 is a cost reduction and can slot anywhere. M5 wants M3, so
its numbers come from the same place as everything else. M6 last, and only if
contention reappears once M1–M3 have landed.
