---
title: "Solo Quickstart"
description: "One-person workflow: create problem, propose solution, critique, approve"
replaces: "uxr/scenarios/01-solo-quickstart.sh"
covers:
  - "Problem: Open → InProgress → Solved"
  - "Solution: Proposed → Submitted → Approved"
  - "Critique: Open → Addressed"
tags: [quickstart, single-user]
---

# Solo Developer Quick Start

## Step 1: Initialize

Set up jjj in a fresh repository:

```jjj
init
> initialized
```

Double-init is safely rejected:

```jjj:fail
init
> already
```

## Step 2: Create a Problem

```jjj
problem new "Search is slow" --priority high
> Search is slow
```

Verify it appears in the list:

```jjj
problem list
> Search is slow
```

## Step 3: Propose a Solution

```jjj
solution new "Add search index" --problem "Search is slow"
> Add search index
```

```jjj
solution list
> Add search index
```

The problem auto-transitions to in_progress when a solution is attached:

```jjj
problem show "Search is slow"
> in_progress
```

## Step 4: Resume Working

Pick up where you left off by resuming a solution:

```jjj
solution resume "search index"
```

## Step 5: Add a Critique

```jjj
critique new "search index" "Missing error handling" --severity medium
> Missing error handling
```

```jjj
critique list
> Missing error
```

## Step 6: Check Status

The solution is blocked by the open critique:

```jjj
status
> BLOCKED
```

## Step 7: Address the Critique

```jjj
critique address "Missing error"
```

Status no longer shows blocked:

```jjj
status
>! BLOCKED
```

## Step 8: Submit and Approve

Submit for review, then approve with all critiques resolved:

```jjj
solution submit "search index"
```

```jjj
solution approve "search index" --no-rationale
```

The problem auto-transitions to solved when its only solution is approved:

```jjj
problem show "Search is slow"
> solved
```

## Step 9: Entity Resolution

jjj resolves entities by full title, partial title, or case-insensitive match:

```jjj
problem new "Authentication is broken" --priority critical
```

Full title match:

```jjj
problem show "Authentication is broken"
```

Partial title match:

```jjj
problem show "auth"
> Authentication
```

Case-insensitive match:

```jjj
problem show "AUTHENTICATION"
```

## Step 10: Error Handling

Non-existent entity:

```jjj:fail
problem show "zzz-nonexistent-zzz"
```

Invalid priority:

```jjj:fail
problem new "test" --priority invalid
> Use
```

Invalid status transition:

```jjj:fail
problem edit "auth" --status solved
```

## Step 11: Help

```jjj
--help
> problem
> solution
> critique
```

```jjj
problem --help
> new
> list
```
