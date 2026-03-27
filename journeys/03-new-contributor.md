---
title: "New Contributor Discovery"
description: "A new contributor explores an existing project, discovers commands, and makes a first contribution"
replaces: "uxr/scenarios/03-new-contributor.sh"
covers:
  - "Status overview as entry point"
  - "Exploring problems, solutions, and critiques"
  - "Milestone and roadmap discovery"
  - "Help discoverability"
  - "Error messages for common mistakes"
  - "Sorting and filtering"
tags: [discovery, onboarding, errors]
---

# New Contributor Discovery

## Setup: Create an Existing Project

```jjj:setup
init
```

```jjj:setup
milestone new "v2.0 Beta" --date 2025-09-01
```

```jjj:setup
problem new "Authentication fails on expired tokens" --priority critical --force
```

```jjj:setup
problem new "Search pagination broken" --priority high --force
```

```jjj:setup
problem new "Dashboard loads slowly" --priority medium --force
```

```jjj:setup
problem new "Add dark mode support" --priority low --force
```

```jjj:setup
problem new "API rate limiting missing" --priority high --force
```

```jjj:setup
milestone add-problem "v2.0" "auth"
```

```jjj:setup
milestone add-problem "v2.0" "pagination"
```

```jjj:setup
milestone add-problem "v2.0" "rate limiting"
```

```jjj:setup
solution new "Use JWT refresh tokens" --problem "auth" --force
```

```jjj:setup
solution new "Add cursor-based pagination" --problem "pagination" --force
```

```jjj:setup
critique new "JWT refresh" "XSS vulnerability in token storage" --severity critical
```

```jjj:setup
critique new "JWT refresh" "No token rotation implemented" --severity high
```

```jjj:setup
problem assign "auth" --to alice
```

```jjj:setup
problem assign "Dashboard" --to bob
```

## Charlie's First Command: What's Going On?

The first thing a new contributor sees is the project status:

```jjj
status
> BLOCKED
```

## Exploring Problems

```jjj
problem list
```

Filter problems by status:

```jjj
problem list --status open
```

View the problem tree:

```jjj
problem tree
```

Drill into a specific problem by keyword:

```jjj
problem show "auth"
> Authentication
> JWT
```

## Understanding Solutions

```jjj
solution list
> JWT
> pagination
```

```jjj
solution show "JWT"
> XSS
```

## Checking Critiques

```jjj
critique list
> XSS
```

```jjj
critique show "XSS"
```

## Milestone and Roadmap

```jjj
milestone list
> v2.0
```

```jjj
milestone roadmap
```

```jjj
milestone show "v2.0"
```

## Help Discoverability

Top-level help shows all major commands:

```jjj
--help
> problem
> solution
> critique
> status
```

Subcommand help shows available actions:

```jjj
problem --help
> new
> list
> show
```

```jjj
solution --help
```

```jjj
critique --help
```

## Common Mistakes and Error Quality

Typo in subcommand:

```jjj:fail
problm list
```

Missing required argument:

```jjj:fail
problem new
```

Wrong entity type keyword:

```jjj:fail
issue list
```

Non-existent entity:

```jjj:fail
problem show "zzz-does-not-exist"
```

## Charlie Contributes: Propose a Solution

```jjj
solution new "Add Redis-based rate limiter" --problem "rate limiting"
> Redis
```

Verify it appears in the list:

```jjj
solution list --problem "rate"
> Redis
```

Charlie adds a self-critique:

```jjj
critique new "Redis" "Redis adds operational complexity" --severity low
> operational complexity
```

## Events and Timeline

```jjj
events
```

## Sorting and Filtering

```jjj
problem list --sort title
```

```jjj
problem list --sort priority
```

```jjj
problem list --sort created
```

```jjj
solution list --sort status
```
