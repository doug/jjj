---
title: Shadow Graph Consistency
description: How jjj reconciles concurrent edits across clones without a server, and why it orders by causality rather than by clock time.
---

# Shadow Graph Consistency

Unlike GitHub, where metadata is central and code is distributed, `jjj`
distributes both. There is no server to serialize writes, so two clones can edit
the same entity at the same time and neither learns of the other until a fetch.
That creates four requirements:

1. **Ref-internal consistency** — every `Solution` must point to a valid
   Jujutsu `ChangeId`.
2. **Concurrency** — two actors adding critiques or proposing solutions must not
   clobber each other.
3. **Convergence** — given the same inputs, every clone must reach byte-identical
   state. Clones that merely *agree with each other while being wrong* are the
   hardest class of bug here, because no convergence test can see it.
4. **Branch awareness** — when you switch changes in `jj`, `jjj status` should
   know which entities are relevant.

## What is the source of truth

**The entity markdown files are.** `problems/{uuid}.md` and its siblings hold
current state; they are what `jjj push` publishes and what `jjj push` validates.

Two things are *derived* and can be deleted without loss:

| Artifact | Derived from | Rebuild with |
|----------|--------------|--------------|
| `.jj/jjj.db` (SQLite cache) | the markdown files | `jjj db rebuild` |
| `.events_offsets.json` | the event shards | automatically |

The event log is **not** the source of truth for state, and the cache is not
rebuilt from it. The log records *what happened and why* — it is the audit
trail, and the thing you read to answer "why was this decided". The entity files
record *what is true now*. Conflating the two leads to the belief that
`db rebuild` replays events; it does not.

## Ordering: causality, not wall clock

Every entity carries a `lamport` logical clock. Two rules define it:

- **Increment on write.** A new version gets a clock strictly greater than the
  version it came from, and greater than anything this clone has seen.
- **Witness on read.** Observing a higher clock advances this clone's counter to
  at least that value, so anything written *after* reading is ordered *after*
  what was read.

Together these make "later" mean **causally later** rather than "stamped with a
bigger number by a machine whose clock is wrong".

### Why not timestamps

Ordering by `updated_at` makes the **fastest clock the winner**. A machine a
year fast won every merge on every entity it touched, permanently: ten honest
later edits by another clone all lost to one year-ahead record, and a single bad
NTP sync is enough to cause it. A timestamp is informational only — nothing
verifies it, and a machine whose clock is wrong is indistinguishable from one
that genuinely wrote later.

### The counter is machine-local

`.lamport` lives beside `.sync_state.json` under `.jj/jjj-meta/` and is never
synced. It has to be: it is *this clone's* view of how much of the world it has
seen, and merging two clones' counters would be meaningless.

Witnessing is capped at a **1,000,000 jump**. Without a bound, one entity
claiming `u64::MAX` would pin the local counter there and every subsequent write
would saturate, destroying the order permanently. A real repository advances the
clock by one per write, so the cap rejects only values that could not have been
reached honestly.

A clock of `0` is omitted from JSON rather than printed, so **an absent
`lamport` means exactly one write**. Read it with a default:

```bash
jjj problem show 01957d --json | jq '.lamport // 0'
```

## Refusing a write that would clobber an unseen edit

`--if-lamport <N>` is available on every edit and on the decisions —
`solve`, `dissolve`, `assign`, `submit`, `approve` — which are as destructive as
an edit and were previously unconditional.

```bash
L=$(jjj problem show 01957d --json | jq '.lamport // 0')
jjj problem edit 01957d --body "..." --if-lamport "$L"
```

A refused write **exits 13**, not 1. The distinction is the point: 13 means
nothing was written and the same command may succeed after a re-read, while 1
means the command was wrong and retrying changes nothing. Without separate codes
a caller in a loop has to tell a stale read from a malformed argument by
matching on prose.

This guards only against an edit **this clone has already seen**. It cannot
guard against one on a clone that has not synced yet — that collision surfaces
at merge, below.

## Merging concurrent edits

`jjj fetch` performs a deterministic three-way merge. The base is a jj
*revision* — `GCA(last_synced_rev, head)` — and base content is reconstructed
from a single full-context `jj diff --git`.

| Shape | Resolution |
|-------|------------|
| Sequences (`solution_ids`, `tags`) | Set union, preserving base items and additions from either side |
| Mappings | Recursively merged key by key |
| Scalars diverged on both sides | Higher `lamport` wins |
| Scalars with equal clocks | `updated_at` compared **as an instant**, then a lexicographic tiebreak |
| Event log / shards | Line union with dedup |
| `created_at` / `updated_at` | Normalized to min / max across both sides |
| Body changed on one side | Clean last-writer-wins |
| Body changed on both sides | Wrapped once in `<<<<<<<` / `>>>>>>>` markers |

Output YAML keys are sorted alphabetically, so every clone produces the same
canonical byte sequence from the same three inputs.

### Three properties that are easy to get wrong

**Timestamps are compared as instants, not strings.** RFC3339 is not
fixed-width here: chrono drops trailing subsecond zeros, so a write on an exact
millisecond is `.191Z` while one 220µs later is `.191220Z`. Lexicographically
`Z` beats `2`, so the shorter string sorted higher and the *earlier* write won —
then stamped its own timestamp on the result. Clones agreed with each other
while doing it.

**A conflicted body is never wrapped twice.** Re-merging a conflicted body
against either of its own parents used to add another marker set every time.
That is the mechanism behind a swarm run whose conflict markers went from 4,371
at single depth to 14,215 nested. A conflicted side already contains both
versions, so nesting adds nothing and costs bytes on every replica — which then
re-conflict on the next sync.

**Unrecognised frontmatter keys are preserved**, not deleted, in an `extra` map.
Otherwise an older client silently destroys a newer one's fields.

### A field cannot outlive the status that gave it meaning

Some merges would produce entities that cannot exist. One clone dissolving a
problem while another solved it merged to `status: solved` with
`dissolved_reason` still set — simultaneously Solved and Dissolved, with the
reason printed to the user. The same shape covered a finding's `superseded_by`.
These merges are refused. Nothing is lost: the event log records the detail, and
it merges by line union.

### Working with conflicts

```bash
jjj conflicts                      # entities left with unresolved markers after a fetch
jjj resolve <id> --ours            # keep this clone's local edit
jjj resolve <id> --theirs          # keep the fetched remote edit
jjj resolve <id> --ours --rationale "the remote benchmark used a warm cache"
```

Resolving strips the markers and records a `conflict_resolved` event, with the
`--rationale` note attached if you give one — so the *reason* a side was chosen
survives even though the discarded text does not.

`jjj push` refuses to publish an entity that still carries markers, so a
conflict stays local instead of propagating to every clone.

## Change ID mapping

`jjj` stores the Jujutsu **Change ID**, not the Git commit ID. Commit IDs change
on rebase or amend; Change IDs are stable through both. A solution therefore
stays attached to the logical work even as the physical commits are rewritten —
which is the property that makes a shadow graph viable at all.

## Verified transitions

Every state change is checked at the moment of the request, against the shadow
graph rather than the cache.

- `jjj solution approve` scans for critiques that may have been fetched recently
  and not yet addressed.
- `jjj problem solve` requires an approved solution; a parent with unfinished
  sub-problems is not ready and `jjj next` will not offer it.
- The bulk path cannot bypass any of this: `jjj plan apply`'s `updates` cannot
  set `status` at all, and its `actions` call the same functions the
  single-entity commands call.

"Approved" is a conclusion drawn from the surviving evidence, not a cached row.
