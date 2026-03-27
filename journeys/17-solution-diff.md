---
title: "Solution Diff"
description: "View diffs for solution change IDs, with error handling for missing solutions and empty changes"
replaces: "uxr/scenarios/17-solution-diff.sh"
covers:
  - "Error on unknown solution"
  - "Informative message when no change IDs attached"
  - "Diff header with change ID for attached changes"
tags: [solution, diff, change-id]
---

# Solution Diff

## Setup

```jjj:setup
init
```

```jjj:setup
problem new "Rendering lag on large datasets" --priority high --force
```

## Step 1: solution diff on unknown solution returns error

```jjj:fail
solution diff "totally-nonexistent-solution-xyz"
```

`solution diff` correctly errors on non-existent solution.

## Step 2: solution with no change IDs shows informative message

```jjj:setup
solution new "Virtualise row rendering" --problem "Rendering lag" --force
```

Capture the auto-attached change ID for later verification:

```shell:setup
jj log --no-graph -r @ -T 'change_id' 2>/dev/null | head -1
>= CHANGE_ID ([a-z]+)
```

Detach the auto-attached change so we can test the empty case:

```jjj:setup
solution detach "Virtualise row rendering" --force
```

```jjj
solution diff "Virtualise row rendering"
> No change IDs
```

`solution diff` handles empty change_ids gracefully.

## Step 3: solution with a change ID shows diff header

```jjj
solution attach "Virtualise row rendering"
> Attached
```

```jjj
solution diff "Virtualise row rendering"
> === Change:
> $CHANGE_ID
```

`solution diff` shows a header for each attached change.
