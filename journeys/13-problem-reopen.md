---
title: "Problem Reopen"
description: "Reopen solved or dissolved problems and verify state transitions"
replaces: "uxr/scenarios/13-problem-reopen.sh"
covers:
  - "Problem: Solved → Open (reopen)"
  - "Problem: Dissolved → Open (reopen)"
  - "Reopen of open problem rejected"
  - "Timeline shows reopen events"
tags: [problem, reopen, state-machine]
---

# Problem Reopen

## Setup

```jjj:setup
init
```

Create and solve a problem through the full lifecycle:

```jjj:setup
problem new "Login times out after idle" --priority high --force
```

```jjj:setup
solution new "Add session keepalive" --problem "Login times out" --force
```

```jjj:setup
solution submit "Add session keepalive"
```

```jjj:setup
solution approve "Add session keepalive" --no-rationale
```

```jjj:setup
problem solve "Login times out"
```

## Solved Problems Hide from Default List

Solved problems are filtered out of the default list:

```jjj
problem list
>! Login times out
```

But can be found with `--status solved`:

```jjj
problem list --status solved
> Login times out
```

## Reopen a Solved Problem

```jjj
problem reopen "Login times out"
> reopened
```

The problem reappears in the default list:

```jjj
problem list
> Login times out
```

Status is back to open:

```jjj
problem show "Login times out"
> open
>! solved
```

## Dissolve and Reopen

```jjj:setup
problem new "Confusing error messages" --priority medium --force
```

```jjj
problem dissolve "Confusing error" --reason "Error messages were already clarified in v1.2"
> dissolved
```

```jjj
problem list --status dissolved
> Confusing error
```

```jjj
problem reopen "Confusing error"
> reopened
```

```jjj
problem show "Confusing error"
> open
```

## Reopen an Already-Open Problem

Reopening a problem that is already open is rejected:

```jjj:setup
problem new "Another open issue" --priority low --force
```

```jjj:fail
problem reopen "Another open"
> open
```

## Timeline Shows Reopen Events

```jjj
timeline "Login times out"
> problem reopened
> problem solved
```

```jjj
timeline "Confusing error"
> problem reopened
> problem dissolved
```
