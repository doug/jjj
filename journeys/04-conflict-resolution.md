---
title: "Conflict Resolution"
description: "Concurrent edits, competing solutions, state machine enforcement, and cascade effects"
replaces: "uxr/scenarios/04-conflict-resolution.sh"
covers:
  - "Sequential edits to different fields"
  - "Concurrent critique resolution"
  - "Back-to-back status transitions"
  - "Competing solutions with critique blocking"
  - "Rapid entity creation"
  - "Dissolve and re-create"
  - "Frontmatter integrity after many edits"
  - "Solution state machine enforcement"
  - "Cascade effects on dissolve"
tags: [conflicts, state-machine, integrity]
---

# Conflict Resolution

## Setup: Create Shared Project

```jjj:setup
init
```

```jjj:setup
problem new "Shared problem" --priority high --force
```

```jjj:setup
solution new "Initial approach" --problem "Shared" --force
```

```jjj:setup
critique new "Initial approach" "Needs tests" --severity medium
```

Verify the initial state:

```jjj
problem show "Shared"
```

## Test 1: Sequential Edits (No Conflict)

Alice edits the problem title:

```jjj
problem edit "Shared" --title "Shared problem (updated by Alice)"
```

Bob edits a different field (priority):

```jjj
problem edit "updated by Alice" --priority critical
```

Both edits persist:

```jjj
problem show "updated by Alice"
> critical
> Alice
```

## Test 2: Concurrent Critique Resolution

Add two more critiques on the same solution:

```jjj:setup
critique new "Initial approach" "Missing error handling" --severity high
```

```jjj:setup
critique new "Initial approach" "No input validation" --severity medium
```

Address them in rapid succession:

```jjj:setup
critique address "error handling"
```

```jjj:setup
critique address "input validation"
```

Only the original "Needs tests" critique should remain open:

```jjj
critique list --solution "Initial" --status open
```

## Test 3: Edit Same Entity Back-to-Back

Edit the problem status back to open, then to in_progress:

```jjj
problem edit "Alice" --status open
```

```jjj
problem edit "Alice" --status in_progress
```

Verify the entity is still consistent:

```jjj
problem show "Alice"
> in_progress
```

## Test 4: Competing Solutions for Same Problem

```jjj:setup
problem new "Performance issue" --priority high --force
```

Alice proposes one solution:

```jjj
solution new "Add caching layer" --problem "Performance"
> caching
```

Bob proposes a different solution to the same problem:

```jjj
solution new "Optimize database queries" --problem "Performance"
> database
```

Both coexist:

```jjj
solution list --problem "Performance"
> caching
> database
```

Submit both solutions for review:

```jjj:setup
solution submit "caching"
```

```jjj:setup
solution submit "database"
```

Charlie critiques Alice's solution:

```jjj
critique new "caching" "Cache invalidation is hard" --severity high
> Cache invalidation
```

Approval is blocked by the open critique:

```jjj:fail
solution approve "caching" --no-rationale
```

Bob's solution has no critiques and can be approved:

```jjj
solution approve "database" --no-rationale
```

## Test 5: Concurrent Entity Creation

Rapidly create multiple problems:

```jjj
problem new "Bug A" --priority low
> Bug A
```

```jjj
problem new "Bug B" --priority low
> Bug B
```

```jjj
problem new "Bug C" --priority low
> Bug C
```

All three exist with unique IDs:

```jjj
problem list
> Bug A
> Bug B
> Bug C
```

## Test 6: Delete and Re-create

Create a problem, dissolve it, then create one with a similar name:

```jjj
problem new "Temporary problem"
> Temporary problem
```

```jjj
problem show "Temporary"
```

```jjj
problem dissolve "Temporary" --reason "Was a duplicate"
```

Create a new one with a similar name:

```jjj
problem new "Temporary issue (new)"
> Temporary issue
```

Both exist (dissolved and new):

```jjj
problem list
```

## Test 7: Frontmatter Integrity After Many Edits

```jjj:setup
problem new "Stress test entity" --priority low --force
```

```jjj:setup
problem edit "Stress test" --priority medium
```

```jjj:setup
problem edit "Stress test" --priority high
```

```jjj:setup
problem edit "Stress test" --title "Stress test entity (v3)"
```

```jjj:setup
problem edit "Stress test" --priority critical
```

After many edits, the final state is consistent:

```jjj
problem show "Stress test"
> critical
> v3
```

## Test 8: Solution State Machine Under Concurrent Pressure

```jjj:setup
problem new "State machine test" --priority medium --force
```

```jjj
solution new "SM solution" --problem "State machine"
> SM solution
```

```jjj:setup
critique new "SM solution" "Issue found" --severity low
```

```jjj:setup
critique address "Issue found"
```

Submit then approve:

```jjj:setup
solution submit "SM solution"
```

```jjj
solution approve "SM solution" --no-rationale
```

Double-approve should be rejected:

```jjj:fail
solution approve "SM solution" --no-rationale
```

Invalid transition (approved back to proposed) should be rejected:

```jjj:fail
solution edit "SM solution" --status proposed
```

## Test 9: Cascade Effects

```jjj:setup
problem new "Cascade test" --priority low --force
```

```jjj:setup
solution new "Cascade solution" --problem "Cascade test" --force
```

```jjj:setup
critique new "Cascade solution" "Cascade critique" --severity low
```

Dissolving a problem with children keeps everything accessible:

```jjj
problem dissolve "Cascade test" --reason "Testing cascade"
```

```jjj
problem show "Cascade test"
> dissolved
```
