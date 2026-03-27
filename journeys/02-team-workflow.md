---
title: "Team Workflow"
description: "Team lead manages a sprint: milestones, assignments, competing solutions, critiques, and approval"
replaces: "uxr/scenarios/02-team-workflow.sh"
covers:
  - "Milestones: create, add problems, roadmap"
  - "Problem assignment to team members"
  - "Competing solutions for the same problem"
  - "Critique blocking prevents premature approval"
  - "Solution withdrawal"
  - "Auto-solve on approval"
  - "Problem hierarchy and dissolve"
  - "Events audit trail"
tags: [team, milestones, workflow]
---

# Team Workflow

## Setup: Initialize Project

```jjj:setup
init
```

## Bob: Create Milestone and Problems

```jjj
milestone new "v1.0 Sprint" --date 2025-06-01
> v1.0 Sprint
```

```jjj:setup
problem new "User login broken" --priority critical --force
```

```jjj:setup
problem new "Search is slow" --priority high --force
```

```jjj:setup
problem new "Dashboard crashes" --priority medium --force
```

Add problems to the milestone:

```jjj:setup
milestone add-problem "v1.0" "login"
```

```jjj:setup
milestone add-problem "v1.0" "Search"
```

```jjj:setup
milestone add-problem "v1.0" "Dashboard"
```

Verify the milestone lists its problems:

```jjj
milestone show "v1.0"
> login
```

## Bob: Assign Problems

```jjj:setup
problem assign "login" --to alice
```

```jjj:setup
problem assign "Search" --to bob
```

```jjj:setup
problem assign "Dashboard" --to charlie
```

## Alice: Propose Solutions

```jjj
solution new "Fix OAuth token refresh" --problem "login" --reviewer @bob
> Fix OAuth token refresh
```

```jjj:setup
solution new "Add elasticsearch" --problem "Search" --force
```

## Bob: Propose Competing Solution

```jjj
solution new "Use simple SQL LIKE search" --problem "Search"
> SQL LIKE
```

Both solutions appear for the same problem:

```jjj
solution list --problem "Search"
> elasticsearch
> SQL LIKE
```

## Charlie: Raise Critiques

```jjj
critique new "OAuth token" "Token refresh doesn't handle clock skew" --severity high
> clock skew
```

```jjj
critique new "OAuth token" "Missing rate limiting on refresh endpoint" --severity medium
> rate limiting
```

```jjj
critique new "elasticsearch" "Elasticsearch requires Java runtime" --severity high
> Java runtime
```

## Check: Approval Should Be Blocked

Submit so the approval attempt hits the critique check:

```jjj:setup
solution submit "OAuth token"
```

Approval is blocked by open critiques:

```jjj:fail
solution approve "OAuth token" --no-rationale
> critique
```

## Alice: Address Critiques

```jjj:setup
critique address "clock skew"
```

```jjj:setup
critique address "rate limiting"
```

Check remaining critiques on the OAuth solution:

```jjj
critique list --solution "OAuth"
```

## Bob: Withdraw Elasticsearch, Approve OAuth

Withdraw the heavier solution:

```jjj:setup
solution withdraw "elasticsearch" --rationale "Too heavyweight for our needs" --no-rationale
```

Verify it shows as withdrawn:

```jjj
solution show "elasticsearch"
> withdrawn
```

Force-approve the OAuth solution (reviewer critique may still be open):

```jjj
solution approve "OAuth token" --force --no-rationale
```

## Verify Auto-Solve and Check Milestone

The login problem auto-solved when its solution was approved:

```jjj
problem show "login"
> solved
```

```jjj
milestone roadmap
> v1.0
```

## Events Audit Trail

```jjj
events
```

Filter events by type:

```jjj
events --event-type problem_created
```

## Status Overview

```jjj
status
```

## Problem Hierarchy

Create a sub-problem under the login problem:

```jjj
problem new "Fix OAuth for mobile" --parent "login"
> Fix OAuth for mobile
```

```jjj
problem tree "login"
> mobile
```

## Dissolve a Problem

```jjj
problem dissolve "Dashboard" --reason "Turned out to be a browser caching issue"
```

```jjj
problem show "Dashboard"
> dissolved
```
