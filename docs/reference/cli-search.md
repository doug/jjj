---
title: Search Commands
description: CLI reference for searching across all jjj entities
---

# Search Commands

Search across problems, solutions, critiques, milestones, and events. Supports both text search and entity reference-based similarity search.

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
