---
title: Search Commands
description: CLI reference for searching across all jjj entities
---

# Search Commands

Search across problems, solutions, critiques, findings, milestones, and events.
Supports both text search and entity reference-based similarity search.

> **Search requires the SQLite cache** at `.jj/jjj.db`. Full-text search and
> embeddings are indexes, not stored data, so unlike the rest of jjj these
> commands cannot fall back to walking the markdown. `jjj search` syncs the
> cache itself; the `--search` flags on `problem list`, `solution list`,
> `critique list` and `finding list` update it incrementally but need the file
> to exist already. If a `--search` filter reports no index, run
> `jjj db rebuild` once.
>
> This is also why `JJJ_NO_DB=1` does not affect these commands: it is a
> read-path measurement switch for the paths that *have* a fallback.
>
> For filtering rather than searching — by status, tag, author, assignee —
> use [`jjj query`](cli-orientation#jjj-query), which needs no cache.

## `jjj search`

```bash
jjj search <query> [options]
```

The search query can be free text or an entity reference (e.g., `p/01957d`) for similarity search.

**Options:**

| Option | Type | Description |
|--------|------|-------------|
| `<query>` | string | Search query or entity reference for similarity search |
| `--type`, `-t` | string | Filter by entity type: `problem`, `solution`, `critique`, `milestone`, `event` |
| `--text-only` | flag | Force text-only search (disable semantic/embedding features) |
| `--json` | flag | Output results as JSON |

**Examples:**

```bash
# Search for anything mentioning "authentication"
jjj search "authentication"

# Search only problems
jjj search "login bug" --type problem

# Find entities similar to a specific problem
jjj search p/01957d

# Text-only search with JSON output
jjj search "refactor" --text-only --json
```
