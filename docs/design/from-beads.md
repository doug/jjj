---
title: What jjj took from beads
description: Two ideas adopted, five refused with reasons, and what their storage choice cost them
---

# What jjj took from beads

[beads](https://github.com/gastownhall/beads) (`bd`) is a graph issue tracker
built for AI coding agents, stored in [Dolt](https://github.com/dolthub/dolt) — a
version-controlled SQL database. It calls itself "a memory upgrade for your
coding agent", and it solves a problem jjj has too: agents lose context on long
tasks, and markdown plans drift.

It is a good comparison because it reaches the opposite conclusion on almost
every axis. beads is a *tool you install* with a database for a backend; jjj is
*metadata in your repository* with markdown for a backend. Where they agree is
more interesting than where they differ, and where beads has had to work around
its own storage choice is the most interesting part of all.

## Taken

### 1. One call that orients an agent — `jjj prime`

beads has `bd prime`, and its stated purpose names the failure exactly:

> Designed for Claude Code, Gemini CLI, and Codex SessionStart hooks to prevent
> agents from forgetting bd workflow after context compaction.

It adapts its own size — "MCP mode: Brief workflow reminders (~50 tokens) / CLI
mode: Full command reference (~1-2k tokens)" — and has `--memories-only` for
hook contexts that can only afford the essentials.

jjj had the same problem and a worse answer. An agent turn starts with no memory
and re-derives the world; the swarm harness worked around it by hand-rolling a
briefing out of **four `jjj` invocations and sixty lines of embedded Python**,
which helped only agents running under that harness and could not be tested. The
measured cost of not having it, recorded in that harness's own comment: **2,322
orientation calls against 138 that produced anything, a ratio of 17 to 1.**

`jjj prime` is that briefing as a command. One pass over the metadata gives what
to work on, what is already proposed so an agent does not build the same idea
twice, what has already been measured so it does not re-measure it, which claims
are live, and whether anything is blocked on a person.

One deliberate difference: **jjj's version prints state, not teaching.** beads'
`prime` mixes live repository state with a command reference, and keeps a
`.beads/PRIME.md` override to customise the latter. jjj's workflow instructions
live in `skills/jjj/SKILL.md`, loaded by the harness. A command that printed both
would go stale in two places at once.

### 2. Work that is not ready yet

beads' best idea is that a dependency graph decides what is *claimable*:

> An issue is ready when ALL of its blocking dependencies are closed.

It has 19 typed edge types in one table, four of which block (`blocks`,
`parent-child`, `conditional-blocks`, `waits-for`), the rest being annotations
(`related`, `discovered-from`, `caused-by`, `validates`, `tracks`, …), plus an
open vocabulary for custom types. Cycles are rejected at write time, and
`bd blocked` shows what is held back and by what.

jjj had nothing of the kind, and the gap was not theoretical. In a three-problem
repository:

```
1. [TODO] Ship the release        ← offered first, and claimable
2. [TODO] Update the changelog      (sub-problem, open)
3. [TODO] Fix the failing test      (sub-problem, open)
```

`jjj next` ranked the parent **first** — its own priority was critical — and an
agent could claim it with every sub-problem still open. Nothing anywhere said it
was not ready. The agent then either flounders or silently does the children's
work without recording it against them.

jjj now treats a problem waiting on unfinished sub-problems as not actionable:
`jjj next` skips it, `jjj problem show` says what it is waiting on, and
`jjj query waiting` lists exactly the parents being held back. When the last
child closes, the parent returns to the queue on its own.

**Derived from `parent_id`, which jjj already stores — no new schema.** That is
the whole of the idea that was going wrong, and it cost one predicate rather than
an edge table. beads' `is_blocked` is a materialized column that **eight
migrations** have had to recompute; deriving it avoids owning that.

## Refused, with reasons

### Explicit `blocks` edges between unrelated problems

The general case beads supports — any issue blocking any other — is a real
capability jjj does not have. Deferred rather than refused outright, because
there is no evidence jjj needs it: the hierarchy covered the case that was
actually failing, `jjj rank` already expresses soft ordering, and an edge table
brings cycle detection, a new relation to merge, and the materialized-state
upkeep beads is paying for. Revisit when something concrete wants it.

### Gates

beads has "gates" — issues that block until an external condition (a PR merge,
CI, a timer) is met. The documentation is unusually honest about why they exist:

> When you use Dolt (server or embedded), issue state is decoupled from code
> state. […] With file-based storage (JSONL), issue updates land atomically with
> code in the same commit. With Dolt, they don't. Gates solve this by making the
> dependency wait for the external condition — not just the beads issue status.

Gates are a workaround for a problem the database created. **jjj has the property
beads describes as the better one**: metadata is committed and pushed with the
code it describes. Nothing to port.

### Merge slots

A genuinely elegant mechanism: one singleton entity per project, where
`status=open` means available, `status=in_progress` means held,
`metadata.holder` names the holder and `metadata.waiters` is a priority-ordered
queue. It exists to prevent

> "monkey knife fights" where multiple polecats race to resolve conflicts and
> create cascading conflicts.

Refused for two reasons. First, the *guarantee* does not port: beads gets
exclusivity from its database, while jjj's claims are advisory by design — two
agents would both acquire and the merge would pick one, which is eventual
exclusion, not mutual exclusion. Second, jjj's swarm has **one integrator** and
nobody else merges, so the race it prevents is already designed away.

Worth building if jjj ever runs several integrators. Until then it is a lock for
a door that has one key. (`docs/design/exclusive-claims.md` records the same
reasoning for claims generally.)

### A schema guard

beads "refuses to open a database migrated by a newer binary, unless
overridden". That is right for beads, because its **database is the source of
truth** — an older binary writing to it could corrupt data nothing else holds.

Tested rather than assumed: set jjj's cache `schema_version` to 999, far newer
than the binary, and run a read. jjj rebuilt the cache from markdown, reset the
version, returned correct results and left every entity file intact. The guard is
unnecessary because jjj's cache is a **disposable index** — the equivalent of
beads' corruption risk cannot happen when the authority is the markdown.

(This is the second time a version gate has been considered and declined for
jjj. The reason is the same both times: the thing a version would protect is
already protected by markdown being canonical and unknown fields being
preserved.)

### `bd remember` / project memories

A keyword-searchable store of durable insights, written by agents. jjj's
`Finding` already covers measured evidence about a problem, and project-wide
conventions live in the target brief and the skill file. The remaining slice —
an insight attached to nothing — would be a sixth entity type for a need no
observed run has demonstrated.

## Noted, not taken

**Semantic compaction.** `bd compact` turned out to be two different things: the
README's "summarizes old closed tasks" is `bd admin compact`, while `bd compact`
alone squashes *Dolt commit history* for disk space. The semantic one is frank
about its cost — "This is **permanent graceful decay** - original content is
discarded" — and its Tier 2 is "planned, not yet implemented".

The design worth remembering is that its **recommended** mode is agent-driven
rather than LLM-automatic: `--analyze` exports candidates, the agent writes the
summary, `--apply` accepts it. No API key, and the judgement stays with something
that can be reviewed.

jjj has no compaction and a corpus that grows forever, with a benchmark at 25,000
entities, so this is a real gap. But jjj would have a strictly better version of
it: because markdown lives in git, a compaction could replace an entity's body
with a summary **without discarding anything** — the original stays in the
bookmark's history. Worth doing when corpus size becomes a measured problem
rather than a predicted one.

**Hash-based ids.** beads uses short content hashes (`bd-a1b2`) specifically to
"avoid merge collisions across branches and agents". jjj's UUID7 serves the same
purpose and additionally sorts chronologically. Equivalent; nothing to change.
