---
title: "Solution Lifecycle"
description: "Full solution state machine: submit, attach, detach, withdraw, approve, assign, supersede"
replaces: "uxr/scenarios/07-solution-lifecycle.sh"
covers:
  - "solution new stays Proposed; solution submit advances to Submitted"
  - "solution list with --status and --problem filters"
  - "solution attach and detach (--force from Submitted)"
  - "Validated critique blocks approval"
  - "solution withdraw with --rationale"
  - "solution supersedes for iteration tracking"
  - "solution approve with --rationale"
  - "solution assign to named person"
  - "JSON output for solutions"
tags: [solution, lifecycle, state-machine]
---

# Solution Lifecycle

## Setup

```jjj:setup
init
```

```jjj
problem new "Login takes too long" --priority high
> Login takes too long
```

## Step 1: solution new stays Proposed; solution submit advances to Submitted

```jjj
solution new "Cache session tokens" --problem "Login takes too long"
> Cache session tokens
```

Solution new auto-attaches the current jj change but stays in Proposed state:

```jjj
solution list
> proposed
> Cache session
```

Solution submit explicitly advances to Submitted:

```jjj
solution submit "Cache session"
> submitted
```

Call solution submit explicitly when ready to submit for review.

## Step 2: solution list --status filter

```jjj
solution list --status submitted
> Cache session
```

```jjj:setup
solution list --status proposed
```

```jjj
solution list --problem "Login"
> Cache session
```

## Step 3: solution attach and detach

Create a new jj change for a second solution:

```shell:setup
jj new -m "feat: try a different approach"
```

```jjj
solution new "Use JWT with short expiry" --problem "Login takes too long"
> Use JWT
```

Advance to Submitted, then verify attach is idempotent:

```jjj:setup
solution submit "JWT"
```

```jjj
solution attach "JWT"
> Attached
```

```jjj
solution show "JWT"
> JWT
```

Detaching from a Submitted solution requires --force to prevent accidental loss of work:

```jjj
solution detach "JWT" --force
> Detached
```

## Step 4: Validate critique then withdraw with rationale

```jjj
critique new "JWT" "JWT expiry too short for mobile clients" --severity high
```

```jjj
critique validate "JWT expiry"
> validated
```

Validated critiques mean the solution has a confirmed flaw.

Validated critiques hard-block approval, same as Open critiques:

```jjj:fail
solution approve "JWT" --no-rationale
```

Dismiss the validated critique to unblock approval:

```jjj
critique dismiss "JWT expiry"
```

```jjj
solution approve "JWT" --no-rationale
> approved
```

Now demonstrate withdraw on a fresh solution:

```shell:setup
jj new -m "feat: jwt retry approach"
```

```jjj
solution new "JWT with sliding expiry" --problem "Login takes too long"
```

```jjj
critique new "JWT with sliding" "Sliding expiry still leaks session state" --severity high
```

```jjj
solution withdraw "JWT with sliding" --rationale "JWT statelessness is fundamentally incompatible with immediate revocation requirements"
> withdrawn
```

```jjj
solution show "JWT with sliding"
> withdrawn
```

Withdrawing with a rationale creates a clear audit trail of why the approach failed.

## Step 5: Superseding Solution (Iteration)

```jjj
solution show "JWT with sliding" --json
> "withdrawn"
```

```jjj
solution new "Sliding window sessions with refresh tokens" --problem "Login takes too long" --supersedes "JWT with sliding"
> Sliding window
```

The supersedes link connects the new solution to the one it replaces, maintaining decision history.

## Step 6: Approve with Rationale

```jjj
solution approve "Cache session" --rationale "Session token cache gives 10x speedup with acceptable security tradeoffs"
> approved
```

```jjj
solution show "Cache session" --json
> "approved"
```

Rationale on approve records the "why" alongside the decision.

## Step 7: solution assign

```jjj
problem new "DB connection pool exhaustion" --priority critical
```

```jjj
solution new "Increase pool size" --problem "DB connection"
> Increase pool size
```

```jjj
solution assign "Increase pool" --to "alice@example.com"
> alice
```

```jjj
solution show "Increase pool"
> alice
```

## Step 8: JSON Output

```jjj
solution list --json
> "id"
> "title"
> "status"
```
