---
title: Orientation & Planning
description: CLI reference for prime, query, plan, escalate, contention, tags, and whoami
---

# Orientation & Planning

These are the commands an agent or a returning human runs *before* touching an
entity: what is the state, what is already in flight, what is blocked, and how
do I create a whole graph of work in one call.

## `jjj prime`

```bash
jjj prime [--full] [--json]
```

Orient in one call — what to work on, what is already proposed so you do not
duplicate it, what has already been measured so you do not re-measure it, and
whether anything is blocked on a person.

| Option | Description |
|--------|-------------|
| `--full` | Every item, rather than the first few per section |
| `--json` | Output as JSON |

**Why this exists.** An agent turn starts with no memory and re-derives the
world. One swarm run made 2,322 orientation calls against 138 that produced
anything. `prime` is that orientation collapsed into a single command.

**Its default output is constant in corpus size** — 7 lines at 50 entities, at
2,000, and at 25,000. Each section lists at most five items with an
`… and N more` line and the true count in its heading; `--full` lifts the cap.
That includes the escalation section, which is the one most likely to be long.

**Workflow instructions are deliberately not here.** `prime` prints state, not
teaching; the instructions live in the skill file. A command that re-teaches the
method on every call wastes the context it was meant to save.

If an escalation has been answered with `escalate --respond`, `prime` prints the
reply. For the next agent that is usually the most useful line in the briefing.

## `jjj query`

```bash
jjj query [EXPR] [--json] [--count-by <FIELD>]
```

Filter every entity kind with one query grammar. **This is a filter, not a
search** — use [`jjj search`](cli-search) for full-text and semantic similarity.

Whitespace-separated terms. Repeating a key **ORs** its values; different keys
**AND**. A bare word matches the title; the bare word `mine` means the current
actor.

| Key | Values |
|-----|--------|
| `type` | `problem`, `solution`, `critique`, `finding`, `milestone` |
| `status` | any status of the kind being filtered |
| `tag` | a tag in use (see [`jjj tags`](#jjj-tags)) |
| `author`, `assignee` | an actor, or `mine` |
| `problem` | ID, prefix, or fuzzy title |
| `sort` | `edit` (causal clock), `created`, `updated`, `title` — prefix `-` to reverse |
| `limit` | a number |

```bash
jjj query "status:open tag:perf sort:edit limit:10"
jjj query "type:solution assignee:mine status:submitted"
jjj query "status:open status:in_progress"     # ORs the two statuses
jjj query waiting                              # parents held back by sub-problems
```

**`waiting` is a predicate, not a key.** It lists problems held back by
unfinished sub-problems *and names what each one is blocked by* — naming only
the blocked parents answered half the question, since the next step is always
"blocked by what". A problem with unfinished children is not ready, so
`jjj next` will not offer it; `waiting` is how you see the ones being held back.
It is problem-only and ignores any `type:` term.

**`sort:edit` sorts by the causal clock**, not by wall-clock time. That is the
ordering that survives a clone with a wrong system time; `sort:updated` does not.

### `--count-by`

```bash
jjj query "type:problem" --count-by status
```

Counts matches grouped by a field — `status`, `type`, `who`, `tag`, or
`priority` — instead of listing them. It answers "how much of what" in one call
rather than one listing per value.

## `jjj plan apply`

```bash
jjj plan apply <FILE> [--dry-run] [--json]
```

Create a whole graph of work from one JSON document. `<FILE>` may be `-` to read
from stdin.

| Option | Description |
|--------|-------------|
| `--dry-run` | Validate and report without writing anything |
| `--json` | Print the key-to-id map as JSON |

Nodes refer to each other by symbolic `key`, so a problem, its sub-problems,
their evidence and the conjectures that cite them can be described together. The
plan is **validated in full before anything is written**, and the result maps
each key to the id it became.

```json
{
  "milestones": [
    { "key": "m1", "title": "Q4 search work", "date": "2026-12-31" }
  ],
  "problems": [
    { "key": "slow", "title": "Search is slow", "body": "p99 over 2s",
      "priority": "high", "tags": ["perf"], "milestone": "m1" },
    { "key": "index", "title": "No trigram index", "parent": "slow" }
  ],
  "findings": [
    { "key": "f1", "title": "p99 is 2.3s at 10k docs", "problem": "slow",
      "method": "ab -n 1000 -c 10", "body": "2312ms",
      "measured_at": "2026-10-16T14:00:00Z" }
  ],
  "solutions": [
    { "key": "s1", "title": "Add a trigram index", "problem": "index",
      "cites": ["f1"] }
  ],
  "critiques": [
    { "key": "c1", "title": "benchmark used a warm cache", "solution": "s1",
      "severity": "medium" }
  ],
  "updates": [
    { "kind": "problem", "on": "slow", "assignee": "alice" }
  ],
  "actions": [
    { "action": "submit", "on": "s1" }
  ]
}
```

Every reference — `parent`, `problem`, `solution`, `milestone`, `on`, `cites`,
`refs` — takes **either a plan key or an existing entity** (id, prefix, or fuzzy
title), so a plan can extend a graph that is already there.

### Node fields

| Node | Required | Optional |
|------|----------|----------|
| `milestones` | `key`, `title` | `body`, `date` |
| `problems` | `key`, `title` | `body`, `priority`, `tags`, `parent`, `milestone` |
| `findings` | `key`, `title`, `problem` | `body`, `method`, `refs`, `measured_at` |
| `solutions` | `key`, `title`, `problem` | `body`, `tags`, `cites` |
| `critiques` | `key`, `title`, `solution` | `body`, `severity` |
| `updates` | `kind`, `on` | `title`, `body`, `priority`, `tags`, `assignee`, `if_lamport` |
| `actions` | `action`, `on` | `reason` (dissolve), `rationale` (withdraw) |

Unknown fields are **refused, not ignored**. A misspelled key would otherwise
leave the author believing it was applied.

### Why `updates` and `actions` are separate

`updates` changes annotations; it cannot set `status`. Every lifecycle
transition has a guard and emits an event — solving needs an approved solution,
approving is refused while a critique is open — and a bulk path that wrote
`status` directly would bypass all of it. Transitions therefore go through
`actions`, which calls the same functions the single-entity commands call:
`solve`, `dissolve`, `reopen`, `submit`, `withdraw`, `address`, `validate`,
`dismiss`.

Likewise, a field that does not apply to a kind (`priority` on a milestone) is
refused rather than dropped, and a duplicate `key` is refused outright — it
would silently decide which of two nodes a `parent` reference pointed at, with
no way for the author to tell which.

An `update` may carry `if_lamport` to refuse that one change if the entity moved
since it was read.

## `jjj escalate`

```bash
jjj escalate [REASON] [OPTIONS]
```

Say a person is needed — the one signal a swarm cannot resolve itself. With no
arguments, lists what is currently blocked.

| Option | Description |
|--------|-------------|
| `--about <ID>` | Entities this concerns, any kind (repeatable) |
| `--list` | List open escalations instead of raising one |
| `--respond <ID>` | Answer an escalation without closing it; `REASON` is the message |
| `--clear <ID>` | Mark an escalation resolved, by id or prefix |
| `--json` | Output as JSON |

```bash
jjj escalate "the staging credential is dead" --about 01957d
jjj escalate --respond 01957d "rotated; new key in the vault"
jjj escalate --clear 01957d
```

**What belongs here.** Something only a human can do: a dead credential, a
destructive step, a blocker the playbook does not cover. **Not** routine
progress, retries, or anything the fleet will fix by itself — a channel carrying
those is a channel nobody reads.

> **`--respond` and `--clear` take an id or prefix, not the reason text.**
> Passing the reason gets `no open escalation matches '...'` — fuzzy title
> matching does not apply to escalations. Get the id from `jjj escalate --list`
> or `--json`.

**`--respond` and `--clear` are separate on purpose.** "Here is the answer" and
"this no longer needs a person" are different claims that often arrive at
different times. Answering without closing is what stops the next agent
rediscovering the same wall; `jjj prime` prints the reply.

## `jjj contention`

```bash
jjj contention [--json]
```

Show where the fleet is doubled up and where nobody is.

**It reports; it does not route.** Several actors on one problem is rivalry —
the method working — and only becomes waste when the rest of the queue is
untouched at the same time. The fix is a re-ranking, which any agent may still
disagree with, not an assignment.

See also [`jjj overlaps`](cli-workflow#jjj-overlaps), which reports the
narrower, mechanical version: files touched by more than one solution.

## `jjj tags`

```bash
jjj tags [--json]
```

List every tag in use with its count. Useful for finding the real vocabulary
before writing a `tag:` term in `jjj query` — tags are free-form, so a fleet
drifts into `perf`, `performance`, and `speed` unless someone looks.

## `jjj whoami`

```bash
jjj whoami [--json]
```

Show the resolved coordination identity: **actor**, **pod**, and **push
bookmark**.

Identity comes from `JJJ_USER` if set and non-blank, otherwise the jj user name;
the pod comes from `JJJ_POD`. Both are resolved rather than read back, so this
is the command that answers "which clone does the graph think I am" — worth
running first when two agents appear to be writing as the same actor. See
[Environment Variables](configuration#environment-variables).

## A worked example

Everything on this page, in the order a turn actually uses it:

```bash,test
jjj init
jjj problem new "Search is slow"
jjj problem new "No trigram index" --parent "Search is slow"
jjj query waiting
# expect: No trigram index
jjj query "type:problem" --count-by status
# expect: open
jjj problem new "Tagged thing" --tags perf,backend
jjj tags
# expect: perf
jjj whoami
# expect: push bookmark
jjj escalate "the staging credential is dead" --about "Search is slow"
# expect: Escalated
jjj escalate --list
# expect: 1 open escalation
jjj contention
# expect: Untouched
jjj prime
# expect: Next for you
```

Note what `prime` does once an escalation is open: it leads with it, above the
work queue, and says not to start anything that depends on it being fixed. That
ordering is the reason `prime` is one call rather than a habit of running four.
