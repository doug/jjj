---
title: "Events Audit"
description: "Event log features: rebuild, validate, date/time filtering, search, JSON output"
replaces: "uxr/scenarios/09-events-audit.sh"
covers:
  - "events baseline listing"
  - "events --limit"
  - "events --event-type filtering"
  - "events --problem and --solution filters"
  - "events --search (rationale full-text)"
  - "events --from / --to date filtering"
  - "events --since (RFC3339 timestamp)"
  - "events --json structured output"
  - "events rebuild"
  - "events validate"
  - "No events.jsonl -- events live in commit history"
  - "Approve emits two events in one commit"
tags: [events, audit, filtering, rebuild, validate]
---

# Events Audit

## Setup: build a rich event history

```jjj:setup
init
```

```jjj:setup
problem new "Memory leak in worker pool" --priority critical --force
```

```jjj:setup
problem new "Slow startup time" --priority high --force
```

```jjj:setup
solution new "Fix worker lifecycle" --problem "Memory leak" --force
```

```jjj:setup
solution new "Lazy-load modules on startup" --problem "Slow startup" --force
```

```jjj:setup
critique new "Fix worker lifecycle" "Workers not properly cleaned up on panic" --severity critical
```

```jjj:setup
critique address "not properly cleaned"
```

```jjj:setup
solution submit "Fix worker"
```

```jjj:setup
solution approve "Fix worker" --rationale "RAII-based cleanup eliminates the leak class entirely"
```

```jjj:setup
solution withdraw "Lazy-load" --rationale "Lazy loading increases first-request latency, not acceptable"
```

```jjj:setup
problem dissolve "Slow startup" --reason "Profiling showed startup is 200ms — not actually a problem"
```

## Step 1: Events baseline

```jjj
events
> problem_created
> solution_created
> critique_raised
```

Events gives a chronological audit trail of all decisions.

## Step 2: Events --limit

```jjj
events --limit 3
> problem
```

## Step 3: Events --event-type

```jjj
events --event-type problem_created
> problem_created
>! solution_created
```

```jjj
events --event-type solution_approved
> solution_approved
>! problem_created
```

```jjj
events --event-type critique_raised
> critique_raised
```

## Step 4: Events --problem and --solution filters

```jjj
events --problem "Memory leak"
> problem_created
```

Problem-scoped event view shows the complete history of one problem.

```jjj
events --solution "Fix worker"
> solution_created
> solution_approved
```

## Step 5: Events --search (rationale full-text)

```jjj
events --search "RAII"
> solution_approved
```

```jjj
events --search "latency"
> solution_withdrawn
```

```jjj
events --search "200ms"
> problem_dissolved
```

Rationale search lets you find past decisions by their reasoning, not just by entity ID.

## Step 6: Events --from / --to date filtering

```shell:setup
date +%Y-%m-%d
>= TODAY (.+)
```

```shell:setup
date +%Y
>= YEAR (.+)
```

```shell:setup
date +%Y-%m
>= MONTH (.+)
```

```jjj
events --from "$TODAY"
> problem_created
```

```jjj
events --from "$YEAR-01-01" --to "$YEAR-12-31"
> problem_created
```

```jjj
events --from "$MONTH"
> problem_created
```

Future date should return empty (no events from the future):

```jjj
events --from "2099-01-01"
```

Date filtering makes it easy to review what happened in a sprint or release window.

## Step 7: Events --since (RFC3339)

```shell:setup
date +%Y
>= SINCE_YEAR (.+)
```

```jjj
events --since "$SINCE_YEAR-01-01T00:00:00Z"
> problem_created
```

`--since` is useful for CI/automation: show everything since the last build.

## Step 8: Events --json

```jjj
events --json
> "type"
> "entity"
> "when"
> "by"
```

```jjj
events --event-type solution_approved --json
> "solution_approved"
```

JSON output enables structured processing of the event log in scripts and pipelines.

## Step 9: Events rebuild

```jjj
events rebuild
> rebuilt
```

Events rebuild replays commit history -- lossless, author/timestamp/rationale preserved exactly.

After rebuild, the log should still be consistent:

```jjj
events
> problem_created
```

## Step 10: Events validate

```jjj
events validate
> valid
```

Events validate confirms the event log is internally consistent -- useful in CI.

## Step 11: No events.jsonl -- events live in commit history

Events are embedded as `jjj: <json>` lines in commit descriptions, so the history IS the event log. This means bookmark merges never produce conflict markers in an events file.

```shell
test ! -f .jj/jjj-meta/events.jsonl && echo "no events.jsonl"
> no events.jsonl
```

Events are still fully readable despite having no file:

```jjj
events
> problem_created
```

No events.jsonl means no merge conflicts. Two contributors can push independently; after fetch, all events appear automatically.

## Step 12: Approve emits two events in one commit

Approving a solution that fully resolves its parent problem emits both `solution_approved` and `problem_solved` in the same commit:

```jjj:setup
problem new "Two-Event Problem" --priority high --force
```

```jjj:setup
solution new "Two-Event Solution" --problem "Two-Event Problem" --force
```

```jjj:setup
solution submit "Two-Event Solution"
```

```jjj:setup
solution approve "Two-Event Solution" --force
```

```jjj
events --event-type solution_approved --json
> solution_approved
```

```jjj
events --event-type problem_solved --json
> problem_solved
```

Approving a solution records both `solution_approved` and `problem_solved` in one atomic commit -- the timeline shows the full causal chain with no gaps.
