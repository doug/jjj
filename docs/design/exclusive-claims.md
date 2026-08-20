---
title: Exclusive Claims
description: What it would take to make jjj claims into real locks, and why the answer is probably not to
---

# Exclusive claims

Design decision 4 makes `jjj next --claim` **advisory**: it records intent, it
does not exclude. Trials bear out what that costs — 42 of 72 contested entities
in one nine-agent hour were claimed by more than one agent, and the duplicated
work is real.

This note asks what a genuine lock would take, and concludes that jjj should not
have one. It is written down because "we chose advisory" is only a defensible
answer if the alternative was actually examined.

## What makes this hard

jjj has no server. Metadata lives in a git bookmark and reaches other clones by
push and fetch. Mutual exclusion normally comes from a single place that says yes
to one requester and no to the rest — and there is no such place here.

Three properties are in tension, and **any lock design gives up at least one**:

| Property | Why jjj has it |
|---|---|
| **Offline-first** | An agent on a plane, or behind a flaky link, keeps working |
| **No server** | The whole point: metadata is just files in a bookmark |
| **Exclusion** | What a lock is for |

You cannot have all three. A lock is an agreement, and agreement requires
communication. An offline clone cannot know whether anyone else took the item.

## Option 1 — Git refs as compare-and-swap

The strongest option, and the only one that yields *true* exclusion without a
server.

A `git push` that updates a ref is a **compare-and-swap**: it succeeds only if
the remote ref still points where the pusher thought it did, and is otherwise
rejected as non-fast-forward. That is exactly the primitive a lock needs, and jjj
already depends on it.

```
acquire(item):
    fetch the locks ref
    if item is held by a live lease: return DENIED
    append { item, holder, expires_at } and push the locks ref
    if the push was rejected: someone else won the race — refetch and retry
    return HELD
```

Locks would live on their own single ref (`jjj-locks`), deliberately *not* the
per-pod bookmarks: contention on that ref is the serialization point, and is the
mechanism rather than a flaw.

**What it costs.** Every acquisition becomes a network round trip — measured at
roughly 300ms median, and 1–2s at p95 under nine agents. Fine for a work item
that takes minutes; ruinous for anything fine-grained. And under N contenders the
ref is exactly the thundering herd that M0 measured at ~quadratic cost, which is
why per-pod bookmarks exist in the first place. It reintroduces Break #5 on
purpose, in one place.

**What it breaks.** Offline operation, completely. An agent that cannot reach the
remote cannot acquire, so it either blocks or proceeds unlocked — and an
unlocked-because-offline path is no lock at all. This is the fatal objection for
jjj, not the latency.

## Option 2 — Deterministic partition

Skip agreement entirely: derive ownership from the work itself.

```
owner(item) = agents[ hash(item) mod len(agents) ]
```

Zero coordination, zero latency, no possibility of a duplicate claim. `jjj next`
already does a weaker version of this — it orders tied work by a hash of the
actor and entity id, which is what stopped nine agents stampeding one problem.

**What it costs.** It needs an agreed agent roster, which is the membership
problem wearing a hat: agents die, restart, and join mid-run, and every clone
must agree on the set or they compute different owners. It also load-balances
badly — an agent assigned three hard items while another gets three easy ones
cannot rebalance, because rebalancing is the coordination we were avoiding.

Viable for a **static, supervised fleet**. Not viable for the open-membership,
humans-and-agents-together model jjj targets.

## Option 3 — Leases with fencing tokens

Option 1 plus a monotonically increasing token per acquisition, carried into
every write. A holder whose lease expired mid-task is detected when its writes
arrive stamped with a stale token, and they can be rejected.

This is the Chubby/ZooKeeper answer, and it is correct: it closes the window
where a paused process wakes up believing it still holds a lock. The ref
generation from Option 1 supplies the counter for free.

**What it costs.** Everything Option 1 costs, plus a token on every entity write
and a rejection path for stale ones. It solves a problem jjj does not have yet:
the damage a stale holder does here is a *duplicate solution*, which is visible,
attributable and cheap to dissolve — not a corrupted balance.

## Option 4 — Make duplication cheap instead of impossible

The current design, stated as a choice rather than an absence.

Claims are advisory; duplicates happen; the system is built so they cost little:

- **`jjj next` spreads a fleet** by ordering tied work per actor, so agents pick
  different items without coordinating.
- **A contested claim converges in one merge** to the earliest claimant, so
  clones stop disagreeing about who holds what.
- **Claims expire**, so a dead agent's work returns to the pool.
- **`solution new` refuses a near-duplicate title**, so the second agent to
  arrive is told someone already has it.
- **`problem duplicate --of`** dissolves a genuine duplicate with a back-reference,
  keeping the record.

Together these do not prevent duplication — they make it rare, visible, and
recoverable.

**What it costs.** Real duplicated effort, at some rate. In the measured hour,
duplicate claims cost perhaps a handful of redundant implementations out of 29
operations completed.

## Recommendation

**Keep advisory claims.** Not because exclusion is unachievable — Option 1 works
— but because the price is offline operation, and offline-first is a load-bearing
property of jjj rather than a nice-to-have.

There is also a reason specific to what jjj is *for*. This is a tool for
conjecture and refutation, where several people attacking one problem from
different angles is the intended behaviour. A lock optimises for "nobody does the
same work twice", and that is the wrong objective for a system whose thesis is
that rival attempts are how you find out which one survives. The failure mode a
lock prevents — two agents implementing the same thing — is the *cheap* failure.
The one it would cause — an agent blocked from attacking a problem someone else
already holds — is the expensive one.

If exclusion is ever genuinely needed, the smallest defensible step is
**Option 1, scoped narrowly**: a lock ref used only for operations that are
actually unsafe to duplicate — a release, a migration, a destructive bulk edit —
while ordinary work stays advisory. That keeps the offline path intact for
everything except the handful of operations that genuinely cannot tolerate two
writers, and those are exactly the operations a person is present for anyway.

## What would change the answer

- **Duplication stops being cheap.** If a duplicated work item cost real money
  (provisioning infrastructure, sending mail, spending compute), the calculus
  inverts and Option 1's round trip is obviously worth paying.
- **The fleet stops being offline-capable.** Containerised agents on one host
  already always have connectivity; if that became the only supported topology,
  the fatal objection to Option 1 disappears.
- **Contention rate rises with scale.** At nine agents duplicates are a nuisance.
  If the rate grows super-linearly toward the design's 10–20 target and beyond,
  measure it before assuming the current mitigations still hold.
