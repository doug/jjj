# SQLite Storage Layer Design

## Overview

SQLite becomes the runtime source of truth for all jjj commands. Markdown files remain the sync format for the shadow branch, enabling human readability, jj conflict resolution, and external editing.

**Motivation:**
- Full-text search across all entities
- Fast queries over events and relationships
- Well-defined schema as the canonical structure
- Validation layer before push catches errors from hand-edited markdown

**Data flow:**
- `jjj fetch` → dump local changes to markdown → jj git fetch → rebuild SQLite from markdown
- `jjj push` → dump SQLite to markdown → validate → commit → jj git push
- All other commands → read/write SQLite directly

**Location:** `.jj/jjj.db` (local only, never synced)

## Key Decisions

### Markdown Remains Sync Format

SQLite is a local cache/index. Markdown files are the distributed format because:
- Human-readable and editable
- jj handles merge conflicts naturally
- Works with standard diff/review tools
- No binary blobs in the shadow branch

### Sync Points: fetch and push

No new commands. Existing `jjj fetch` and `jjj push` become the sync points:
- **fetch**: Rebuilds SQLite from markdown after fetching
- **push**: Dumps SQLite to markdown, validates, then pushes

### Schema Versioning

Track schema version in SQLite. On any command:
- If DB missing → rebuild from markdown
- If schema version mismatch → rebuild from markdown
- On fetch → always rebuild from markdown

No migrations needed since markdown is authoritative.

### Conflict Resolution

On fetch, if local SQLite has uncommitted changes:
1. Dump SQLite → markdown first
2. Commit to shadow branch
3. Fetch (jj handles merge conflicts in markdown)
4. User resolves any conflicts
5. Rebuild SQLite from merged markdown

Markdown always wins after conflict resolution.

## Schema

```sql
-- Version tracking (for rebuild-on-mismatch)
CREATE TABLE meta (
    key TEXT PRIMARY KEY,
    value TEXT
);
-- INSERT ('schema_version', '1')

-- Core entities
CREATE TABLE problems (
    id TEXT PRIMARY KEY,           -- 'p1', 'p2', ...
    title TEXT NOT NULL,
    status TEXT NOT NULL,          -- 'open', 'in_progress', 'solved', 'dissolved'
    priority TEXT,
    parent_id TEXT REFERENCES problems(id),
    milestone_id TEXT REFERENCES milestones(id),
    assignee TEXT,
    created_at TEXT,
    updated_at TEXT,
    body TEXT                      -- markdown content
);

CREATE TABLE solutions (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    status TEXT NOT NULL,          -- 'proposed', 'testing', 'accepted', 'refuted'
    problem_id TEXT REFERENCES problems(id),
    change_ids TEXT,               -- JSON array of jj change IDs
    supersedes TEXT REFERENCES solutions(id),
    assignee TEXT,
    created_at TEXT,
    updated_at TEXT,
    body TEXT
);

CREATE TABLE critiques (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    status TEXT NOT NULL,          -- 'open', 'addressed', 'valid', 'dismissed'
    solution_id TEXT REFERENCES solutions(id),
    severity TEXT,
    reviewer TEXT,
    file_path TEXT,
    line_number INTEGER,
    created_at TEXT,
    updated_at TEXT,
    body TEXT
);

CREATE TABLE milestones (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    status TEXT,
    target_date TEXT,
    assignee TEXT,
    created_at TEXT,
    updated_at TEXT,
    body TEXT
);

-- Events (from events.jsonl)
CREATE TABLE events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp TEXT NOT NULL,
    type TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    actor TEXT NOT NULL,
    rationale TEXT,
    refs TEXT,                     -- JSON array
    extra TEXT                     -- JSON for type-specific fields
);

-- Full-text search
CREATE VIRTUAL TABLE fts USING fts5(
    entity_type,                   -- 'problem', 'solution', 'critique', 'milestone', 'event'
    entity_id,
    title,
    body,
    author,
    tags,
    content='',                    -- contentless, we manage sync
    tokenize='porter unicode61'
);

-- Indexes
CREATE INDEX idx_events_timestamp ON events(timestamp);
CREATE INDEX idx_events_type ON events(type);
CREATE INDEX idx_events_entity ON events(entity_id);
CREATE INDEX idx_solutions_problem ON solutions(problem_id);
CREATE INDEX idx_critiques_solution ON critiques(solution_id);
CREATE INDEX idx_problems_milestone ON problems(milestone_id);
```

## Sync Flow

### On `jjj fetch`

```
1. Check if SQLite has uncommitted changes (dirty flag in meta table)
2. If dirty:
   a. Dump SQLite → markdown files
   b. Dump events table → events.jsonl
   c. Commit to shadow branch: "Sync local changes before fetch"
3. Run `jj git fetch`
4. If conflicts in markdown files:
   a. Warn user: "Conflicts in metadata, resolve and run `jjj fetch` again"
   b. Exit (user resolves in jj, then re-runs fetch)
5. Delete jjj.db
6. Create fresh DB with schema
7. Parse all markdown files → INSERT into tables
8. Parse events.jsonl → INSERT into events
9. Rebuild FTS index
10. Clear dirty flag
```

### On `jjj push`

```
1. Dump SQLite → markdown files
2. Dump events table → events.jsonl
3. Validate:
   a. All foreign keys resolve (problem_id, solution_id, etc.)
   b. All required fields present
   c. Status values are valid enums
   d. Dates parse correctly
4. If validation fails:
   a. Show errors
   b. Abort push
5. Commit to shadow branch with structured message
6. Run `jj git push`
7. Clear dirty flag
```

### On Any Other Command

```
1. If DB missing or schema version mismatch:
   a. Rebuild from markdown (same as fetch steps 5-10)
2. Read/write SQLite
3. Set dirty flag on writes
```

## Validation Rules

On push, validate before committing:

| Check | Error message |
|-------|---------------|
| `solution.problem_id` exists | "s3 references non-existent problem p99" |
| `critique.solution_id` exists | "c5 references non-existent solution s12" |
| `problem.parent_id` exists | "p4 references non-existent parent p99" |
| `problem.milestone_id` exists | "p2 references non-existent milestone m5" |
| Status values valid | "c5 has invalid status 'pending'" |
| Required fields present | "s7 missing required field 'title'" |
| Dates parseable | "p3 has invalid date '2024-13-45'" |
| No circular parent references | "p3 → p5 → p3 creates a cycle" |

Example output:

```
$ jjj push
Validating metadata...
  ✗ s3 references non-existent problem p99
  ✗ c5 has invalid status "pending"
Push aborted. Fix errors and retry.
```

## CLI Changes

### New Command: Global Search

```bash
# Full-text search across all entities
jjj search "authentication timeout"
jjj search "cache" --type problem      # limit to problems
jjj search "alice" --type event        # search event rationales/actors
```

### Enhanced List Commands

```bash
jjj problem list --search "auth"
jjj solution list --search "retry"
jjj critique list --search "race condition"
jjj events --search "benchmark"
```

## Implementation

### Crate Additions

```toml
[dependencies]
rusqlite = { version = "0.31", features = ["bundled"] }
```

The `bundled` feature compiles SQLite into the binary—no system dependency.

### New Modules

- `src/db.rs` — SQLite connection, schema creation, version check
- `src/db/load.rs` — markdown → SQLite parsing
- `src/db/dump.rs` — SQLite → markdown serialization
- `src/db/fts.rs` — FTS index management and search queries

### Modified Modules

- `src/storage.rs` — Replace file-based CRUD with SQLite calls; keep markdown parsing for load/dump
- `src/commands/fetch.rs` — Add dump-before-fetch, rebuild-after-fetch logic
- `src/commands/push.rs` — Add dump, validate, commit logic
- All entity commands — Switch from `MetadataStore` file ops to DB queries

### Migration Path

- Existing repos work unchanged—first command rebuilds DB from markdown
- No breaking changes to markdown format
- Users don't need to take any action

## Future Considerations

Not part of initial implementation:

- **Structured sections** — Parse markdown `## Section` headers into separate columns for section-specific queries
- **Incremental sync** — Diff changes rather than full rebuild (optimization if rebuild becomes slow)
- **Content hash** — Skip rebuild when markdown unchanged (optimization)

## Summary

SQLite provides fast queries and full-text search while markdown remains the human-readable, mergeable sync format. The design:

- Keeps the existing shadow branch architecture
- Adds no new sync commands (fetch/push handle everything)
- Validates data integrity before push
- Handles conflicts through jj's native merge resolution
- Requires no migration—existing repos just work
