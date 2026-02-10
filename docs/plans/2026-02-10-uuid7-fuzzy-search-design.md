# UUID7 + Fuzzy Search ID System

## Overview

Replace sequential IDs (`p1`, `s1`) with UUID7 identifiers and fuzzy title search. This enables distributed entity creation without collisions while improving human usability through title-based lookup.

## Storage Format

Entities stored on the `jjj` branch (renamed from `jjj/meta`):

```
problems/01957d3e-a8b2-7def-8c3a-9f4e5d6c7b8a.md
solutions/01957d3e-b1c4-7abc-9d2e-3f4a5b6c7d8e.md
critiques/01957d3e-c2d5-7fed-ae4b-5c6d7e8f9a0b.md
milestones/01957d3e-d3e6-7cba-bf5c-6d7e8f9a0b1c.md
config.toml
events.jsonl
```

UUID7 provides natural time-ordering. `ls problems/` shows oldest first.

## Truncated Prefix Display

Listings show a short hex prefix, auto-extended to be unambiguous within that entity type:

```
$ jjj problem list
a3f8c2  open    Fix auth timeout bug
a3f8d1  open    Auth token refresh fails
b7e2f9  solved  Database connection pooling
```

Minimum prefix length: 6 characters. If `a3f8c` would match multiple problems, it extends to `a3f8c2` and `a3f8d1`.

### Mixed-Type Listings

Commands showing multiple types use type prefixes for clarity:

```
$ jjj status
Next actions:
  p/a3f8c2  Address critique on auth fix
  s/b7e2f9  Testing: DB pooling solution
  c/d4e5f6  Respond to review comment
```

Type prefix (`p/`, `s/`, `c/`, `m/`) only appears when context doesn't imply the type.

## Entity Resolution

When a user provides an identifier, resolve through this priority chain:

1. **Exact UUID match** - If input is valid UUID7 format, direct lookup
2. **Prefix match** - If input looks like hex (e.g., `a3f8c2`), find UUIDs starting with that prefix
3. **Fuzzy title search** - Otherwise, query SQLite FTS on titles

Resolution is context-aware: `jjj problem show "auth"` only searches problems.

### Ambiguity Handling

When multiple entities match:

**Interactive TTY:**
```
$ jjj problem show "auth"
? Select problem:
> a3f8c2  Fix auth timeout bug
  a3f8d1  Auth token refresh fails
  a3f9b2  Auth middleware refactor

[↑↓ navigate] [type to filter] [enter to select] [esc to cancel]
```

**Non-TTY (piped/scripted):**
```
$ jjj problem show "auth" | head
Error: Multiple problems match "auth"
  a3f8c2  Fix auth timeout bug
  a3f8d1  Auth token refresh fails
Use a more specific query or the short ID.
```

### Match Thresholds

- 1 match: proceed automatically
- 2-10 matches: show picker (TTY) or error with suggestions
- >10 matches: show picker with "and N more...", rely on type-to-filter
- 0 matches: error "No problems match 'xyz'"

### Scripts and Automation

Scripts should use full UUIDs for determinism. The `--json` output always includes full UUIDs.

## Fuzzy Search Implementation

### SQLite FTS5 Index

Extend existing SQLite database (`.jj/jjj.db`):

```sql
CREATE VIRTUAL TABLE entities_fts USING fts5(
    uuid,
    entity_type,    -- 'problem', 'solution', 'critique', 'milestone'
    title,
    description,
    content_body
);
```

### Search Query

```
jjj problem show "auth timeout"
```

Translates to:

```sql
SELECT uuid, title, rank
FROM entities_fts
WHERE entity_type = 'problem'
  AND entities_fts MATCH 'auth timeout'
ORDER BY rank
LIMIT 10;
```

### Index Sync

SQLite index syncs from markdown files:
- On CLI startup (if stale)
- After any write operation
- Explicit `jjj sync` command

Markdown files on the `jjj` branch remain the source of truth. SQLite is a local read-optimization cache.

## File Format Changes

Frontmatter `id` field changes from sequential to UUID7:

```yaml
---
id: 01957d3e-a8b2-7def-8c3a-9f4e5d6c7b8a
title: Fix auth timeout bug
parent_id: 01957d3e-a1b1-7abc-8d2e-1f2a3b4c5d6e
status: open
priority: medium
solution_ids:
  - 01957d3e-b1c4-7def-9e3f-4a5b6c7d8e9f
created_at: 2026-02-10T14:30:00Z
updated_at: 2026-02-10T15:45:00Z
---

## Description
Users experience timeout errors when...
```

All cross-reference fields change to UUID7:

| Field | Old | New |
|-------|-----|-----|
| `id` | `p1` | UUID7 |
| `parent_id` | `p2` | UUID7 or null |
| `solution_ids` | `["s1", "s2"]` | UUID7 array |
| `problem_id` (solutions) | `p1` | UUID7 |
| `solution_id` (critiques) | `s1` | UUID7 |
| `milestone_id` | `m1` | UUID7 |
| `problem_ids` (milestones) | `["p1"]` | UUID7 array |

## Migration

### Automatic Migration with Confirmation

On first run after upgrade, detect old-style files and prompt:

```
$ jjj problem list
Found 12 entities using legacy IDs (p1, s1, ...).
Migrate to UUID7 identifiers? [Y/n]
```

Migration steps:
1. Generate UUID7 for each entity (using original `created_at` timestamp to preserve ordering)
2. Rename files: `problems/p1.md` → `problems/01957d3e-....md`
3. Update all cross-references (parent_id, solution_ids, etc.)
4. Commit to `jjj` branch with message "Migrate to UUID7 identifiers"

Non-TTY environments skip with warning, require explicit `jjj migrate`.

### No Backwards Compatibility

Clean break - no legacy_id field or old ID support after migration.

## Implementation Scope

### Core Changes

1. **ID Generation** (`src/storage.rs`)
   - Replace `next_problem_id()` etc. with UUID7 generation
   - Use `uuid` crate with v7 feature

2. **Entity Resolution** (`src/resolve.rs` - new)
   - Resolution chain: exact UUID → prefix match → fuzzy search
   - Ambiguity detection and match counting

3. **Interactive Picker** (`src/picker.rs` - new)
   - TTY detection
   - Integrate `dialoguer` or `skim` for selection UI

4. **SQLite FTS** (`src/db/search.rs`)
   - Extend existing FTS to be primary lookup
   - Index all entity types with title + description

5. **CLI Argument Parsing** (`src/cli.rs`)
   - All ID arguments become `String` resolved at runtime
   - Remove type prefixes from argument names

6. **Display Formatting** (`src/display.rs` - new or extend)
   - Truncated prefix calculation
   - Type-prefixed display for mixed listings

7. **Branch Rename**
   - Change `jjj/meta` to `jjj`
   - Update `META_BOOKMARK` constant in `src/storage.rs`

### Files Unchanged

- VS Code extension (uses CLI JSON output)
- TUI (consumes resolved entities)
- Models (ID field type stays `String`)

### New Dependencies

- `uuid` with `v7` feature
- `dialoguer` or `skim` for interactive picker
