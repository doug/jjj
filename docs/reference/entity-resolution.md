---
title: Entity Resolution
description: How jjj resolves entity references via UUID, truncated prefix, and fuzzy title match
---

# Entity Resolution

jjj uses flexible entity resolution that lets you reference problems, solutions, critiques, and milestones in multiple ways. You can use whichever method is most convenient for your workflow.

## Resolution Methods

When you reference an entity (e.g., `jjj problem show "auth bug"`), jjj tries to resolve it in this order:

### 1. Full UUID

The most explicit method. Entity IDs are UUID7 format:

```
01957d3e-a8b2-7def-8c3a-9f4e5d6c7b8a
```

UUID7 identifiers are time-ordered, meaning entities created later have lexicographically larger IDs. This provides natural chronological sorting in listings.

### 2. Truncated Prefix

Like git short hashes, you can use the first N characters of the UUID (minimum 6 hex characters):

```bash
jjj problem show 01957d
jjj solution approve 01958a
```

In listings, jjj automatically calculates the shortest unique prefix for each entity:

```
Problems:
  01957d  App crashes on startup     [open]
  01958a  Login is slow              [open]
  01958b  Search performance         [in_progress]
```

If your prefix matches multiple entities, jjj extends it automatically or prompts you to be more specific.

### 3. Fuzzy Title Match

The most human-friendly method. Type any part of the entity's title:

```bash
jjj problem show "auth bug"
jjj solution resume "connection pooling"
jjj critique address "error handling"
```

Fuzzy matching is case-insensitive and matches partial strings:
- `"auth"` matches "Fix authentication bug"
- `"search slow"` matches "Search is slow"
- `"v1.0"` matches "v1.0 Release"

## Disambiguation

### Interactive Mode (TTY)

When multiple entities match your query in an interactive terminal, jjj shows a fuzzy-searchable picker:

```
Multiple problems match "auth":
> Fix authentication bug          [open]
  Update auth module              [in_progress]
  Auth token expiration           [solved]

Use arrow keys to select, type to filter
```

Use arrow keys to navigate and press Enter to select. You can also type to further filter the results.

### Non-Interactive Mode (Scripts/Pipes)

When running in scripts or piped commands, jjj returns an error with suggestions:

```
Error: Ambiguous match for "auth"

Did you mean one of these?
  01957d  Fix authentication bug
  01958a  Update auth module
  01958b  Auth token expiration

Specify a longer prefix or more specific title.
```

## Context-Aware Resolution

Commands know what entity type they expect, so you don't need type prefixes:

```bash
# These work because the command implies the type
jjj problem show "auth bug"       # Searches problems only
jjj solution approve "pooling"     # Searches solutions only
jjj critique address "error"      # Searches critiques only
```

## Mixed-Type Listings

In commands that show multiple entity types (like `jjj status`), type prefixes distinguish entities:

```
Next Actions:
  p/01957d  App crashes on startup     [open]
  s/01958a  Add connection pooling     [review]
  c/01958b  Missing error handling     [open]
```

The `p/`, `s/`, `c/`, `m/` prefixes indicate:
- `p/` = Problem
- `s/` = Solution
- `c/` = Critique
- `m/` = Milestone

## Examples

### Creating Related Entities

```bash
# Create a problem
jjj problem new "Search is slow" --priority high

# Reference by title when creating a solution
jjj solution new "Add search index" --problem "Search is slow"

# Partial match works too
jjj solution new "Use caching" --problem "slow"

# Reference solution by title when creating critique
jjj critique new "search index" "Missing pagination" --severity medium
```

### Chaining Operations

```bash
# Address a critique by partial title
jjj critique address "pagination"

# Accept the solution
jjj solution approve "search index"

# Solve the problem
jjj problem solve "Search"
```

### Filtering Lists

```bash
# Filter solutions by problem title
jjj solution list --problem "auth"

# Filter critiques by solution
jjj critique list --solution "pooling"
```

## Best Practices

1. **Use titles for readability**: `jjj solution approve "pooling"` is clearer than `jjj solution approve 01958a`

2. **Use prefixes in scripts**: For automation, use the prefix from `--json` output to avoid ambiguity

3. **Be specific enough**: If "auth" matches multiple items, try "auth bug" or "authentication"

4. **Check listings first**: Run `jjj problem list` to see available prefixes and titles

5. **Trust the picker**: In interactive mode, let the fuzzy picker help you find the right entity
