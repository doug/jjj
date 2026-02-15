---
title: Event Lifecycle
description: How a jjj command is transformed into a shadow graph event.
---

# Event Lifecycle

`jjj` uses an **Event Sourcing** model. This architecture ensures that all project metadata is distributed, immutable, and resides within your repository without polluting your commit history.

## From CLI to Shadow Graph

The lifecycle of an action (like creating a problem) follows this path:

### 1. Command Execution
When you run `jjj problem new "Fix bug"`, the CLI parses your intent and creates an `Event`. This event contains:
- **Timestamp**: When the event occurred.
- **Author**: Who created it (from your `.jjj/config.toml`).
- **Action**: The specific operation (`CreateProblem`).
- **Payload**: The data for the action (title, priority, etc.).

### 2. Event Log Persistence
The event is serialized into a **Protobuf** format and written to the shadow graph. 
- Location: `.jj/.jjj/events/` (inside your local Jujutsu metadata).
- Filename: A unique hash of the event.

### 3. The Shadow Graph
The shadow graph is a parallel commit graph that stores these events. 
- It maps `jj` change IDs to `jjj` entities.
- It is synchronized across machines during `jjj push` and `jjj fetch`.

### 4. Database Indexing
For high-performance querying and TUI rendering, `jjj` maintains a local **SQLite** index of the shadow graph.
- When you run a command, `jjj` checks if the shadow graph has new events.
- New events are "replayed" into the SQLite database to update the view of the world.
- If the database is missing or corrupted, it can be entirely rebuilt from the event log via `jjj db rebuild`.

## Event Sourcing Benefits
1. **Auditability**: We have a perfect log of every decision made in the project.
2. **Offline-First**: You can create problems and solutions while on a plane; they sync when you're back online.
3. **Branch Awareness**: `jjj` knows which features are being worked on in which `jj` changes because the event links them.
