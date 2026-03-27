---
title: "Milestone Advanced"
description: "Full milestone lifecycle: create, edit, assign, roadmap progression, completion, JSON output"
replaces: "uxr/scenarios/11-milestone-advanced.sh"
covers:
  - "Milestone new with and without date"
  - "Milestone edit (title, date, status)"
  - "problem new --milestone assigns on creation"
  - "Milestone add-problem and remove-problem"
  - "Milestone assign"
  - "Roadmap progression as problems get solved"
  - "Milestone completion when all problems solved"
  - "Milestone show/list/roadmap --json"
tags: [milestone, lifecycle, roadmap, json]
---

# Milestone Advanced

## Setup

```jjj:setup
init
```

## Step 1: Milestone New (with and without date)

```jjj
milestone new "v1.0 Launch"
> v1.0
```

```jjj
milestone new "v1.1 Patch" --date "2026-09-01"
> v1.1
```

```jjj
milestone list
> v1.0
> v1.1
```

Milestones can be created with or without a target date for flexible planning.

## Step 2: Milestone Edit (title, date, status)

```jjj:setup
milestone edit "v1.0 Launch" --title "v1.0 GA Release"
```

```jjj
milestone show "v1.0 GA"
> v1.0 GA
```

```jjj:setup
milestone edit "v1.0 GA" --date "2026-07-15"
```

```jjj
milestone show "v1.0 GA"
> 2026-07-15
```

```jjj:setup
milestone edit "v1.1 Patch" --status active
```

```jjj
milestone show "v1.1"
> active
```

Milestone edit lets you refine plans as dates and scope become clearer.

## Step 3: Problem New --milestone (assign on creation)

```jjj:setup
problem new "Login crashes on empty password" --priority critical --milestone "v1.0 GA" --force
```

```jjj:setup
problem new "Settings page layout broken" --priority high --milestone "v1.0 GA" --force
```

```jjj:setup
problem new "Add export to CSV" --priority medium --milestone "v1.1 Patch" --force
```

```jjj
milestone show "v1.0 GA"
> Login crashes
> Settings page
```

Using `problem new --milestone` saves a step compared to `milestone add-problem`.

## Step 4: Milestone Add-problem (explicit)

```jjj:setup
problem new "Dark mode flickers" --priority low --force
```

```jjj:setup
milestone add-problem "v1.1 Patch" "Dark mode flickers"
```

```jjj
milestone show "v1.1"
> Dark mode
```

## Step 5: Milestone Remove-problem

```jjj:setup
milestone remove-problem "v1.1 Patch" "Dark mode flickers"
```

```jjj
milestone show "v1.1"
>! Dark mode
```

Remove-problem lets you adjust scope without deleting the problem itself.

## Step 6: Milestone Assign

```jjj:setup
milestone assign "v1.0 GA" --to "alice@example.com"
```

```jjj
milestone show "v1.0 GA"
> alice
```

## Step 7: Milestone Roadmap

```jjj
milestone roadmap
> v1.0
> v1.1
> problems solved
```

Roadmap gives a cross-milestone view of what is planned and what is solved.

## Step 8: Milestone Progression (solve problems to complete milestone)

Solve the v1.0 problems through the full solution lifecycle:

```jjj:setup
solution new "Add nil check before auth" --problem "Login crashes" --force
```

```jjj
critique new "Add nil check" "Test coverage missing" --severity low
> Test coverage missing
```

```jjj:setup
critique address "Test coverage"
```

```jjj:setup
solution submit "Add nil check"
```

```jjj:setup
solution approve "Add nil check" --no-rationale
```

```jjj:setup
solution new "Fix flexbox order in settings" --problem "Settings page" --force
```

```jjj:setup
solution submit "Fix flexbox"
```

```jjj:setup
solution approve "Fix flexbox" --no-rationale
```

Both v1.0 problems are now solved. Check roadmap reflects this:

```jjj
milestone roadmap
> solved
```

Mark the milestone complete:

```jjj:setup
milestone edit "v1.0 GA" --status completed
```

```jjj
milestone show "v1.0 GA"
> completed
```

Milestone progression is visible in the roadmap as problems get solved.

## Step 9: Milestone Show --json

```jjj
milestone show "v1.0 GA" --json
> "title"
> "status"
> completed
```

```jjj
milestone list --json
> "id"
> v1.0
> v1.1
```

JSON output makes milestone data available to project dashboards and reports.

## Step 10: Milestone Roadmap --json

```jjj
milestone roadmap --json
> "id"
> "problem_ids"
```
