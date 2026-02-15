---
title: Database Commands
description: CLI reference for jjj database status and maintenance
---

# Database Commands

Database commands provide visibility into and maintenance of jjj's internal data store. These are primarily useful for troubleshooting.

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

Rebuild the database from the event log. Use this if the database becomes corrupted or out of sync with the underlying events.

> **Note:** This re-processes all events from the shadow graph. It is safe to run but may take a moment on repositories with extensive history.

**Example:**

```bash
jjj db rebuild
```
