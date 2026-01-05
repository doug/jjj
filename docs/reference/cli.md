# CLI Reference

Complete reference for all jjj commands.

## Global Options

All commands support these global options:

| Option | Description |
|--------|-------------|
| `-h, --help` | Show help information |
| `-V, --version` | Show version information |

## Commands Overview

| Command | Description |
|---------|-------------|
| [`init`](#init) | Initialize jjj in a repository |
| [`board`](#board) | Display Kanban board |
| [`dashboard`](#dashboard) | Show overview dashboard |
| [`task`](#task) | Manage tasks |
| [`feature`](#feature) | Manage features |
| [`milestone`](#milestone) | Manage milestones |
| [`bug`](#bug) | Manage bugs |
| [`review`](#review) | Manage code reviews |
| [`resolve`](#resolve) | Resolve conflicts |

---

## `jjj init`

Initialize jjj metadata in the current Jujutsu repository.

**Usage**:
```bash
jjj init
```

**What it does**:
- Creates `jjj/meta` bookmark (shadow graph)
- Sets up metadata directory structure
- Creates default configuration

**Example**:
```bash
$ cd ~/my-project
$ jjj init
Initialized jjj in ~/my-project
Created metadata bookmark: jjj/meta
```

---

## `jjj board`

Display the Kanban board view.

**Usage**:
```bash
jjj board [OPTIONS]
```

**Options**:
| Option | Description |
|--------|-------------|
| `--json` | Output in JSON format |

**Examples**:
```bash
# Interactive TUI board
jjj board

# JSON output for scripting
jjj board --json

# Parse with jq
jjj board --json | jq '.TODO | length'
```

**JSON Output Format**:
```json
{
  "TODO": [...tasks],
  "In Progress": [...tasks],
  "Review": [...tasks],
  "Done": [...tasks]
}
```

---

## `jjj dashboard`

Show overview dashboard with reviews and tasks.

**Usage**:
```bash
jjj dashboard [OPTIONS]
```

**Options**:
| Option | Description |
|--------|-------------|
| `--json` | Output in JSON format |

**Example**:
```bash
jjj dashboard
```

**Output**:
```
Dashboard

Pending Reviews:
  kpqxywon... - Add authentication (@alice, @bob)

Tasks:
  TODO: 5 tasks
  In Progress: 2 tasks
  Done: 10 tasks
```

---

## `jjj task`

Manage tasks (individual units of work).

**Subcommands**:
- [`new`](#jjj-task-new) - Create a new task
- [`list`](#jjj-task-list) - List tasks
- [`show`](#jjj-task-show) - Show task details
- [`attach`](#jjj-task-attach) - Attach change to task
- [`detach`](#jjj-task-detach) - Detach change from task
- [`move`](#jjj-task-move) - Move task to different column
- [`edit`](#jjj-task-edit) - Edit task details
- [`delete`](#jjj-task-delete) - Delete a task

### `jjj task new`

Create a new task.

**Usage**:
```bash
jjj task new <TITLE> --feature <FEATURE_ID> [OPTIONS]
```

**Arguments**:
| Argument | Description |
|----------|-------------|
| `<TITLE>` | Task title (required) |

**Options**:
| Option | Description |
|--------|-------------|
| `--feature <ID>` | Feature this task belongs to (required) |
| `--tag <TAG>` | Add tags (can be repeated) |
| `--column <COL>` | Initial column (default: "TODO") |

**Examples**:
```bash
# Basic task
jjj task new "Implement password hashing" --feature F-1

# With tags
jjj task new "Add login API" \
  --feature F-1 \
  --tag backend \
  --tag api

# In specific column
jjj task new "Write docs" \
  --feature F-2 \
  --column "In Progress"
```

### `jjj task list`

List all tasks with optional filters.

**Usage**:
```bash
jjj task list [OPTIONS]
```

**Options**:
| Option | Description |
|--------|-------------|
| `--column <COL>` | Filter by column |
| `--tag <TAG>` | Filter by tag |
| `--json` | Output in JSON format |

**Examples**:
```bash
# All tasks
jjj task list

# In-progress tasks only
jjj task list --column "In Progress"

# Backend tasks
jjj task list --tag backend

# JSON output
jjj task list --json | jq '.[] | {id, title, column}'
```

### `jjj task show`

Show detailed information about a task.

**Usage**:
```bash
jjj task show <TASK_ID>
```

**Example**:
```bash
$ jjj task show T-1
Task: T-1 - Implement password hashing
Column: In Progress
Feature: F-1
Tags: backend, security
Changes: kpqxywon, lmnopqrs
Created: 2025-11-23 10:00:00
```

### `jjj task attach`

Attach the current change to a task.

**Usage**:
```bash
jjj task attach <TASK_ID>
```

**Example**:
```bash
# Make some code changes
echo "fn hash() {}" >> src/auth.rs

# Attach current change to task
jjj task attach T-1
```

### `jjj task move`

Move a task to a different column.

**Usage**:
```bash
jjj task move <TASK_ID> <COLUMN>
```

**Example**:
```bash
jjj task move T-1 "In Progress"
jjj task move T-1 "Review"
jjj task move T-1 "Done"
```

---

## `jjj feature`

Manage features (user-facing capabilities).

**Subcommands**:
- `new` - Create a feature
- `list` - List features
- `show` - Show feature details
- `board` - Show feature board view
- `progress` - Show feature progress
- `move` - Move feature to different status

**Quick Reference**:

```bash
# Create feature
jjj feature new "User Authentication" \
  --milestone M-1 \
  --priority high

# List features
jjj feature list --milestone M-1

# Show feature details
jjj feature show F-1

# Show feature board (all features)
jjj feature board

# Show tasks for specific feature
jjj feature board F-1

# Show progress
jjj feature progress F-1

# Update status
jjj feature move F-1 inprogress
jjj feature move F-1 done
```

**Status Values**: `backlog`, `inprogress`, `review`, `done`, `blocked`

**Priority Values**: `low`, `medium`, `high`, `critical`

---

## `jjj milestone`

Manage milestones (release targets).

**Subcommands**:
- `new` - Create a milestone
- `list` - List milestones
- `show` - Show milestone details
- `add-feature` - Add feature to milestone
- `add-bug` - Add bug to milestone
- `roadmap` - Show roadmap view

**Quick Reference**:

```bash
# Create milestone
jjj milestone new "v1.0 Release" \
  --date 2025-12-31 \
  --description "Initial release"

# List milestones
jjj milestone list --json

# Show details
jjj milestone show M-1

# Add feature to milestone
jjj milestone add-feature M-1 F-1

# Add bug to milestone
jjj milestone add-bug M-1 B-1

# Show roadmap
jjj milestone roadmap
```

---

## `jjj bug`

Manage bugs (defects and issues).

**Subcommands**:
- `new` - Report a bug
- `list` - List bugs
- `show` - Show bug details
- `link` - Link bug to feature/milestone
- `status` - Update bug status
- `triage` - Show bug triage view

**Quick Reference**:

```bash
# Report bug
jjj bug new "Login fails with special chars" \
  --severity high \
  --repro "1. Enter email with +\n2. Click login"

# List bugs
jjj bug list --severity critical --open

# Show details
jjj bug show B-1

# Link to feature and milestone
jjj bug link B-1 --feature F-1 --milestone M-1

# Update status
jjj bug status B-1 confirmed
jjj bug status B-1 fixed

# Triage view
jjj bug triage --json
```

**Severity Values**: `low`, `medium`, `high`, `critical`

**Status Values**: `new`, `confirmed`, `inprogress`, `fixed`, `closed`, `wontfix`, `duplicate`

---

## `jjj review`

Manage code reviews.

**Subcommands**:
- `request` - Request a review
- `list` - List reviews
- `start` - Start reviewing
- `comment` - Add a comment
- `status` - Show review status
- `approve` - Approve a change
- `request-changes` - Request changes

**Quick Reference**:

```bash
# Request review
jjj review request @alice @bob

# Request stack review
jjj review request @alice --stack

# List reviews
jjj review list --pending

# Start reviewing
jjj review start kpqxywon

# Add inline comment
jjj review comment kpqxywon \
  --file src/auth.rs \
  --line 42 \
  --body "Use bcrypt instead"

# Add general comment
jjj review comment kpqxywon \
  --body "Looks good overall!"

# Approve
jjj review approve kpqxywon

# Request changes
jjj review request-changes kpqxywon \
  --message "Please address security concerns"

# Check status
jjj review status kpqxywon
```

---

## `jjj resolve`

Resolve conflicts in tasks or reviews.

**Usage**:
```bash
jjj resolve <ID> [OPTIONS]
```

**Options**:
| Option | Description |
|--------|-------------|
| `--pick <VERSION>` | Pick specific version |

**Example**:
```bash
# Interactive conflict resolution
jjj resolve T-1

# Pick specific version
jjj resolve T-1 --pick "Done"
```

---

## JSON Output Format

All commands that support `--json` return structured data:

### Task JSON

```json
{
  "id": "T-1",
  "title": "Implement password hashing",
  "feature_id": "F-1",
  "column": "In Progress",
  "tags": ["backend", "security"],
  "change_ids": ["kpqxywon"],
  "assignee": "alice@example.com",
  "created_at": "2025-11-23T10:00:00Z",
  "updated_at": "2025-11-23T15:30:00Z"
}
```

### Feature JSON

```json
{
  "id": "F-1",
  "title": "User Authentication",
  "milestone_id": "M-1",
  "status": "InProgress",
  "priority": "High",
  "task_ids": ["T-1", "T-2", "T-3"],
  "bug_ids": ["B-1"],
  "tags": ["auth", "security"],
  "story_points": 8,
  "created_at": "2025-11-20T10:00:00Z",
  "updated_at": "2025-11-23T15:30:00Z"
}
```

### Milestone JSON

```json
{
  "id": "M-1",
  "title": "v1.0 Release",
  "target_date": "2025-12-31T00:00:00Z",
  "status": "Active",
  "feature_ids": ["F-1", "F-2"],
  "bug_ids": ["B-1"],
  "version": "1.0.0",
  "created_at": "2025-11-15T10:00:00Z",
  "updated_at": "2025-11-23T15:30:00Z"
}
```

### Bug JSON

```json
{
  "id": "B-1",
  "title": "Login fails with special chars",
  "severity": "High",
  "status": "Fixed",
  "feature_id": "F-1",
  "milestone_id": "M-1",
  "repro_steps": "1. Enter email...",
  "affected_version": "0.9.0",
  "fixed_version": "1.0.0",
  "created_at": "2025-11-22T10:00:00Z",
  "updated_at": "2025-11-23T15:30:00Z"
}
```

## Scripting Examples

### Generate Report

```bash
#!/bin/bash
# Generate release report

MILESTONE="M-1"

echo "# Release Report for $MILESTONE"
echo

echo "## Features"
jjj feature list --milestone $MILESTONE --json | \
  jq -r '.[] | "- \(.id): \(.title) (\(.status))"'

echo
echo "## Tasks"
jjj task list --json | \
  jq -r --arg m "$MILESTONE" \
    'group_by(.column) | .[] |
     "### \(.[0].column)\n" +
     (map("- \(.id): \(.title)") | join("\n"))'

echo
echo "## Bugs"
jjj bug list --json | \
  jq -r '.[] | "- \(.id): \(.title) [\(.severity)] [\(.status)]"'
```

### Find Blocked Work

```bash
# Find all blocked features
jjj feature list --json | \
  jq '.[] | select(.status == "Blocked") | {id, title, task_count: (.task_ids | length)}'

# Find tasks in review
jjj task list --json | \
  jq '.[] | select(.column == "Review")'
```

### Export for External Tools

```bash
# Export to CSV
jjj task list --json | \
  jq -r '["ID","Title","Feature","Column","Tags"],
         (.[] | [.id, .title, .feature_id, .column, (.tags | join(";"))]) |
         @csv'
```

## Next Steps

- [**Configuration Reference**](configuration.md) - Customize jjj behavior
- [**Task Management Guide**](../guides/task-management.md) - Learn task workflows
- [**Code Review Guide**](../guides/code-review.md) - Master code reviews
