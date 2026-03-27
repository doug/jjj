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

## Step 5: jjj next --mine (shows work for current user)

`next --mine` shows all open TODO problems (no active solutions) regardless of assignee.
Problem 3 (Avatar upload, unassigned) appears as a TODO:

```jjj
next --mine
> Avatar upload
```

`next --mine` does not filter problems by assignee -- use `problem list --assignee` for that.

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
