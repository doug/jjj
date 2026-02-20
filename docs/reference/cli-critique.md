---
title: Critique Commands
description: CLI reference for creating, listing, editing, and resolving critiques
---

# Critique Commands

Critiques are criticisms of solutions. They track issues found during review, with severity levels and a lifecycle: open, addressed, valid, or dismissed. Critiques can reference specific files and lines.

## `jjj critique new`

Add a critique to a solution.

```
jjj critique new <solution_id> <title> [OPTIONS]
```

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--severity` | string | `medium` | Severity: low, medium, high, critical |
| `--file` | string | | File path for code-level critique |
| `--line` | integer | | Line number for code-level critique |
| `--reviewer` | string | | Mark as a review request from this reviewer (e.g., `@alice`) |

When `--reviewer` is specified, the critique is created as a review request. The reviewer field indicates which reviewer should address this critique. This unifies the concepts of review requests and critiques into a single model.

```bash,test
jjj init
jjj problem new "Fix login flow"
jjj solution new "Refactor auth handler" --problem "Fix login"
jjj critique new "Refactor auth" "Missing error handling" --severity high
jjj critique new "Refactor auth" "Unused import" --severity low --file src/auth.rs --line 3
jjj critique list
```

## `jjj critique list`

List critiques.

```
jjj critique list [OPTIONS]
```

| Flag | Type | Description |
|------|------|-------------|
| `--solution` | string | Filter by solution |
| `--status` | string | Filter by status (open, addressed, valid, dismissed) |
| `--reviewer` | string | Filter by reviewer (e.g., `@alice`) |
| `--search` | string | Search critiques by title text |
| `--json` | bool | Output in JSON format |

```bash,test
jjj critique list --solution "Refactor auth"
jjj critique list --status open
jjj critique list --json
```

## `jjj critique show`

Show critique details.

```
jjj critique show <critique_id> [OPTIONS]
```

| Flag | Type | Description |
|------|------|-------------|
| `--json` | bool | Output in JSON format |

```bash,test
jjj critique show "Missing error"
jjj critique show "Missing error" --json
```

## `jjj critique edit`

Edit critique details.

```
jjj critique edit <critique_id> [OPTIONS]
```

| Flag | Type | Description |
|------|------|-------------|
| `--title` | string | New title |
| `--severity` | string | New severity (low, medium, high, critical) |
| `--status` | string | New status (open, addressed, valid, dismissed) |

```bash
jjj critique edit "Missing error" --severity critical --title "Missing error handling in auth"
```

## `jjj critique address`

Mark a critique as addressed (the solution was modified to fix the issue).

```
jjj critique address <critique_id>
```

```bash,test
jjj critique address "Missing error"
```

## `jjj critique validate`

Validate a critique (confirm it is correct; the solution should be refuted).

```
jjj critique validate <critique_id>
```

```bash
jjj critique validate "Missing error"
```

## `jjj critique dismiss`

Dismiss a critique (incorrect or irrelevant).

```
jjj critique dismiss <critique_id>
```

```bash
jjj critique dismiss "Unused import"
```

## `jjj critique reply`

Reply to a critique.

```
jjj critique reply <critique_id> <body>
```

```bash
jjj critique reply "Missing error" "Fixed in latest change, added error handling for all auth paths"
```
