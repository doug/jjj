---
title: Database Commands
description: CLI reference for jjj database status and maintenance
---

# Database Commands

The database is a **disposable SQLite cache** at `.jj/jjj.db`. Markdown under
`.jj/jjj-meta/` is the source of truth; the cache only serves reads faster.
Deleting it loses nothing — `jjj push` validates the markdown without it, and
most commands fall back to walking the files.

Two things do require the file, because they have no filesystem fallback:
`jjj search` and the `--search` flags on `problem`/`solution`/`critique`/`finding
list`. Full-text search and embeddings are indexes, not stored data, so they
cannot be derived on the fly.

## `jjj db status`

```bash
jjj db status
```

Display the current status of the jjj database, including entity counts and storage health.

**Example:**

```bash
jjj db status
```

## `jjj db rebuild`

```bash
jjj db rebuild
```

Rebuild the database from the markdown files, re-indexing full-text search and
embeddings. Use it when the cache is stale or missing — including the first time
in a repository where `--search` reports no index.

> **Note:** This re-reads and re-parses every entity, so it scales with corpus
> size rather than with how stale the cache is. It is always safe to run: it
> only ever derives from markdown, so it cannot lose data.

It rebuilds from **markdown, not from the event log**. The event log records
what happened; the entity files record what is true now.

**Example:**

```bash
jjj db rebuild
```
