---
title: "Milestone Status"
description: "Milestone status command with completion percentages, JSON output, and empty milestone handling"
replaces: "uxr/scenarios/14-milestone-status.sh"
covers:
  - "Milestone status shows 0% at start"
  - "Completion updates as problems are solved"
  - "Dissolved problems count toward completion"
  - "milestone status --json structured output"
  - "Empty milestone handles 0/0 gracefully"
tags: [milestone, status, json, completion]
---

# Milestone Status

## Setup

```jjj:setup
init
```

## Step 1: Create Milestone with 3 Problems

```jjj
milestone new "Beta Release" --date "2026-12-01"
> Beta Release
```

```jjj:setup
problem new "Fix login crash" --priority critical --force
```

```jjj:setup
problem new "Add rate limiting" --priority high --force
```

```jjj:setup
problem new "Improve error messages" --priority medium --force
```

```jjj:setup
milestone add-problem "Beta Release" "Fix login crash"
```

```jjj:setup
milestone add-problem "Beta Release" "Add rate limiting"
```

```jjj:setup
milestone add-problem "Beta Release" "Improve error messages"
```

Milestone has 3 problems, all open.

## Step 2: Milestone Status Shows 0% at Start

```jjj
milestone status "Beta Release"
> 3 total
> 0% complete
> Beta Release
> 2026-12-01
```

Milestone status shows correct 0% when no problems are solved.

## Step 3: Solve 1 Problem (33%)

```jjj:setup
solution new "Add nil guard to auth handler" --problem "Fix login crash" --force
```

```jjj:setup
solution submit "Add nil guard"
```

```jjj:setup
solution approve "Add nil guard" --no-rationale
```

```jjj:setup
problem solve "Fix login crash"
```

```jjj
milestone status "Beta Release"
> 1 solved
> 33% complete
```

Completion percentage updates correctly as problems are solved.

## Step 4: Dissolve a Problem (counts toward completion)

```jjj:setup
problem dissolve "Improve error messages" --reason "Out of scope for beta"
```

```jjj
milestone status "Beta Release"
> 66% complete
```

Dissolved problems count toward milestone completion.

## Step 5: Milestone Status --json

```jjj
milestone status "Beta Release" --json
> "title"
> "total"
> "solved"
> "dissolved"
> "pct_complete"
> "target_date"
> "days_remaining"
> "in_progress"
> "open"
> "total": 3
> "solved": 1
> "dissolved": 1
> "pct_complete": 66
> 2026-12-01
```

JSON output provides machine-readable milestone completion data.

## Step 6: Empty Milestone Handles 0/0 Gracefully

```jjj:setup
milestone new "Future Plans"
```

```jjj
milestone status "Future Plans"
> 0 total
> 0% complete
```

```jjj
milestone status "Future Plans" --json
> "total": 0
> "pct_complete": 0
```

Milestone status handles milestones with no problems without error.
