---
title: Milestone Commands
description: CLI reference for creating, editing, and tracking milestones and roadmaps
---

# Milestone Commands

Milestones group problems into time-boxed releases or goals. They track progress across problems and their solutions.

## `jjj milestone new`

Create a new milestone.

```
jjj milestone new <title> [OPTIONS]
```

| Flag | Type | Description |
|------|------|-------------|
| `--date` | string | Target date (YYYY-MM-DD) |

```bash,test
jjj init
jjj milestone new "v1.0 Release" --date 2025-06-01
jjj milestone new "Q3 Goals"
jjj milestone list
```

## `jjj milestone edit`

Edit milestone details.

```
jjj milestone edit <milestone_id> [OPTIONS]
```

| Flag | Type | Description |
|------|------|-------------|
| `--title` | string | New title |
| `--date` | string | New target date (YYYY-MM-DD) |
| `--status` | string | New status (planning, active, completed, cancelled) |

```bash,test
jjj milestone edit "v1.0 Release" --status active
jjj milestone edit "v1.0 Release" --title "v1.0 GA Release" --date 2025-07-01
```

## `jjj milestone list`

List all milestones.

```
jjj milestone list [OPTIONS]
```

| Flag | Type | Description |
|------|------|-------------|
| `--json` | bool | Output in JSON format |

```bash,test
jjj milestone list
jjj milestone list --json
```

## `jjj milestone show`

Show milestone details.

```
jjj milestone show <milestone_id> [OPTIONS]
```

| Flag | Type | Description |
|------|------|-------------|
| `--json` | bool | Output in JSON format |

```bash,test
jjj milestone show "v1.0"
jjj milestone show "v1.0" --json
```

## `jjj milestone add-problem`

Add a problem to a milestone.

```
jjj milestone add-problem <milestone_id> <problem_id>
```

```bash,test
jjj problem new "Improve performance"
jjj milestone add-problem "v1.0" "Improve performance"
```

## `jjj milestone remove-problem`

Remove a problem from a milestone.

```
jjj milestone remove-problem <milestone_id> <problem_id>
```

```bash
jjj milestone remove-problem "v1.0" "Improve performance"
```

## `jjj milestone roadmap`

Show roadmap view with problems and solution progress across milestones.

```
jjj milestone roadmap [OPTIONS]
```

| Flag | Type | Description |
|------|------|-------------|
| `--json` | bool | Output in JSON format |

```bash,test
jjj milestone roadmap
```

## `jjj milestone assign`

Assign a milestone to a person.

```
jjj milestone assign <milestone_id> [OPTIONS]
```

| Flag | Type | Description |
|------|------|-------------|
| `--to` | string | Assignee name (defaults to self) |

```bash
jjj milestone assign "v1.0" --to alice
```
