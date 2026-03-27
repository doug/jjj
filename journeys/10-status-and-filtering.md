---
title: "Status and Filtering"
description: "Flag-based filtering and output modes across all entity types"
replaces: "uxr/scenarios/10-status-and-filtering.sh"
covers:
  - "status --limit, --all, --json"
  - "problem list --status, --assignee, --search, --milestone, --sort, --tree, --json"
  - "problem show --json"
  - "solution list --status, --problem, --search, --sort, --json"
  - "solution show --json"
  - "critique list --status, --solution, --reviewer, --json"
  - "critique show --json"
  - "milestone list --json and show --json"
  - "search --type, --text-only, --json"
  - "timeline --json"
  - "db status"
  - "completion bash/zsh/fish"
tags: [status, filtering, json, search, completion]
---

# Status and Filtering

## Setup: a project with varied entities

```jjj:setup
init
```

```jjj:setup
problem new "Performance regression in search" --priority critical --force
```

```jjj:setup
problem new "Missing dark mode" --priority low --force
```

```jjj:setup
problem new "Auth tokens expire too quickly" --priority high --force
```

```jjj:setup
problem new "Onboarding flow is confusing" --priority medium --force
```

```jjj:setup
problem assign "Performance regression" --to "alice@example.com"
```

```jjj:setup
problem assign "Auth tokens" --to "bob@example.com"
```

```jjj:setup
milestone new "v2.0 Release" --date "2026-06-01"
```

```jjj:setup
milestone add-problem "v2.0" "Performance regression"
```

```jjj:setup
milestone add-problem "v2.0" "Auth tokens"
```

```jjj:setup
solution new "Add search result cache" --problem "Performance regression" --force
```

```jjj:setup
solution new "Extend token lifetime to 30 days" --problem "Auth tokens" --force
```

```jjj:setup
solution new "Add token refresh flow" --problem "Auth tokens" --force
```

```jjj:setup
critique new "Add search result cache" "Cache invalidation not handled" --severity high
```

```jjj:setup
critique new "Extend token lifetime" "30 days is too long for security policy" --severity critical --reviewer "security@example.com"
```

Build FTS index before search tests:

```jjj:setup
db rebuild
```

## Step 1: Status flags

```jjj
status
```

```jjj
status --limit 2
> problem
```

Status `--all` shows all items; blocked solutions appear with their critique summary:

```jjj
status --all
> BLOCKED
```

```jjj
status --json
> "items"
> "summary"
> "total_count"
```

Status `--json` enables CI dashboards and custom tooling on top of jjj.

## Step 2: Problem list filters

Problems with solutions are `in_progress`; genuinely open ones have no solution:

```jjj
problem list --status open
> Onboarding flow
>! Performance regression
```

```jjj
problem list --status in_progress
> in_progress
> Performance regression
```

Assignee filter uses substring comparison:

```jjj
problem list --assignee "alice"
> Performance regression
>! Missing dark mode
```

```jjj
problem list --assignee "bob@example.com"
> Auth tokens
```

Search uses FTS with porter stemmer -- "token" matches "tokens":

```jjj
problem list --search "token"
> Auth tokens
>! Performance regression
```

Milestone filter accepts UUID, prefix, or title -- no need to look up the UUID:

```jjj
problem list --milestone "v2.0"
> Performance regression
>! Missing dark mode
```

Sort variants:

```jjj
problem list --sort priority
```

```jjj
problem list --sort title
```

```jjj
problem list --sort created
```

```jjj
problem list --tree
```

```jjj
problem list --json
> "id"
> "priority"
> "status"
```

List filters reduce noise -- show only what is relevant to the current focus.
Assignee and reviewer filters use substring matching -- partial email or username works.

## Step 3: Problem show --json

```jjj
problem show "Performance regression" --json
> "title"
> "priority"
> "status"
> alice@example.com
```

## Step 4: Solution list filters

Solution new auto-attaches the current change but stays in Proposed state:

```jjj
solution list --status proposed
> proposed
> Add search result cache
```

```jjj
solution list --problem "Auth tokens"
> Extend token
> Add token refresh
>! search result cache
```

```jjj
solution list --search "token"
> token
```

```jjj
solution list --sort title
```

```jjj
solution list --sort created
```

```jjj
solution list --json
> "id"
> "status"
```

## Step 5: Solution show --json

```jjj
solution show "Add search result cache" --json
> "title"
> "status"
> "problem_id"
```

## Step 6: Critique list filters

```jjj
critique list --status open
> Cache invalidation
> 30 days is too long
```

```jjj
critique list --solution "Add search result cache"
> Cache invalidation
>! 30 days
```

Reviewer filter uses substring match:

```jjj
critique list --reviewer "security"
> 30 days is too long
```

```jjj
critique list --json
> "id"
> "severity"
```

## Step 7: Critique show --json

```jjj
critique show "Cache invalidation" --json
> "title"
> "severity"
> "status"
```

## Step 8: Milestone show --json and list --json

```jjj
milestone list --json
> "id"
> "title"
```

```jjj
milestone show "v2.0" --json
> "title"
> "status"
> 2026-06-01
```

## Step 9: Search --type and --text-only

FTS uses porter stemming -- "token" finds "tokens", "cache" finds "cached":

```jjj
search "token" --type problem
> Auth tokens
>! Extend token lifetime
```

```jjj
search "cache" --type solution
> Add search result cache
```

`--text-only` skips embeddings and uses FTS only:

```jjj
search "cache" --text-only
> cache
```

JSON output field is `type` (not `entity_type`):

```jjj
search "invalidation" --json
> "title"
> "type"
```

Search `--type` narrows results when you know what kind of entity you are looking for.
Search `--text-only` is useful when embeddings have not been computed yet.

## Step 10: Timeline --json

```jjj
timeline "Performance regression" --json
> "type"
> "entity"
> "when"
```

Timeline `--json` is useful for generating changelogs and audit reports programmatically.

## Step 11: DB status

```jjj
db status
> Database
```

DB status shows cache health -- useful for debugging search and embedding issues.

## Step 12: Shell completion

Completion generates shell-specific completion scripts:

```jjj
completion bash
```

```jjj
completion zsh
```

```jjj
completion fish
```

Shell completions lower the barrier to learning all the flags.
