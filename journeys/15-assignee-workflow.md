---
title: "Assignee Workflow"
description: "Assign problems, solutions, and milestones to people or self; filter by assignee"
replaces: "uxr/scenarios/15-assignee-workflow.sh"
covers:
  - "Assign problem/solution/milestone to named person"
  - "Self-assign via jj identity when --to omitted"
  - "Filter with --assignee on problem and solution list"
  - "Reassign overwrites previous assignee"
  - "Assignee shown in both JSON and plain-text output"
  - "next --mine restricts the queue to work this actor owns"
  - "JJJ_USER scopes the queue per agent"
  - "Work owned by another actor is not offered"
  - "Claims are leases; explicit assignments never expire"
tags: [assignee, workflow, self-assign, filter]
---

# Assignee Workflow

## Setup

```jjj:setup
init
```

Discover the current jj user identity for self-assign assertions:

```shell:setup
jj config get user.name 2>/dev/null || echo "Test User"
>= SELF_NAME (.+)
```

Create three problems with different priorities:

```jjj:setup
problem new "Database connection pool exhausted" --priority critical --force
```

```jjj:setup
problem new "Search results are slow" --priority high --force
```

```jjj:setup
problem new "Avatar upload fails silently" --priority medium --force
```

Create two solutions linked to problems:

```jjj:setup
solution new "Use connection pooling library" --problem "Database connection" --force
```

```jjj:setup
solution new "Add search result caching" --problem "Search results" --force
```

Create a milestone:

```jjj
milestone new "v2.0 Release" --date "2026-06-01"
> v2.0
```

## Step 2: Assign problems and solutions

Assign problem 1 to alice:

```jjj
problem assign "Database connection" --to alice
> alice
```

Assign solution 1 to bob:

```jjj
solution assign "connection pooling" --to bob
> bob
```

Assign problem 2 to self (no `--to` uses jj identity):

```jjj
problem assign "Search results are slow"
> $SELF_NAME
```

Assign milestone to alice:

```jjj
milestone assign "v2.0" --to alice
> alice
```

`problem assign` with no `--to` uses the jj `user.name`, not the git config `user.name`.
`milestone assign` shares the same pattern as problem/solution assign.

## Step 3: Verify assignees in show output

```jjj
problem show "Database connection" --json
> "alice"
```

```jjj
solution show "connection pooling" --json
> "bob"
```

```jjj
problem show "Search results" --json
> $SELF_NAME
```

```jjj
milestone show "v2.0" --json
> "alice"
```

All entity types (problem, solution, milestone) persist assignee through save/load.

## Step 4: Filter with --assignee

Filter problems assigned to alice:

```jjj
problem list --assignee alice
> Database connection
>! Search results
>! Avatar upload
```

Filter problems assigned to self:

```jjj
problem list --assignee "$SELF_NAME"
> Search results
>! Database connection
```

Filter solutions assigned to bob:

```jjj
solution list --assignee bob
> connection pooling
>! search result caching
```

`--assignee` filter uses case-insensitive substring matching.

## Step 5: jjj next --mine (work this actor owns)

`next --mine` restricts the queue to items you own — the assignee of a problem
or solution, or the reviewer of a critique. Assign the remaining unowned problem
so there is something to own:

```jjj:setup
problem assign "Avatar upload" --to alice
```

Alice sees it; nobody else does:

```shell
JJJ_USER=alice $JJJ next --top 0 --mine
> Avatar upload
```

```shell
JJJ_USER=bob $JJJ next --top 0 --mine
> Nothing to do
```

Work that belongs to someone is not offered to anyone else, with or without the
flag. Every problem here is now owned, so a third actor is told there is nothing
to pick up rather than being pointed at someone else's job:

```shell
JJJ_USER=carol $JJJ next --top 0
> Nothing to do
```

This is what makes a fleet of agents workable. If the queue were shared, every
agent starting at once would be handed the same top item and all of them would
take it — which is exactly what happened the first time a swarm ran against jjj:
four of four agents claimed one problem.

A claim is a **lease**, not a lock. `jjj next --claim` records when it was taken,
and once the lease lapses the item returns to the pool, so an agent that dies
mid-task cannot strand its work forever. An explicit `problem assign` has no
lease and never expires — handing work to a person is a decision, not a claim.
The lease is an hour by default; set `claim_ttl_minutes` under `[settings]` in
`config.toml` to change it.

## Step 6: Reassign to a different person

Reassign problem 1 from alice to charlie:

```jjj
problem assign "Database connection" --to charlie
> charlie
```

```jjj
problem show "Database connection" --json
> "charlie"
>! "alice"
```

Reassign solution from bob to self (no `--to`):

```jjj
solution assign "connection pooling"
> $SELF_NAME
```

```jjj
solution show "connection pooling" --json
> $SELF_NAME
>! "bob"
```

Reassignment overwrites the previous assignee with no confirmation required.

## Step 7: Assignee appears in non-JSON show output

```jjj
problem show "Database connection"
> Assignee
> charlie
```

```jjj
solution show "connection pooling"
> Assignee
> $SELF_NAME
```

Assignee field renders in both `--json` and plain-text output modes.
