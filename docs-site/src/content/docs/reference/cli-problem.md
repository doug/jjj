---
title: Problem Commands
description: CLI reference for creating, listing, editing, and resolving problems in jjj
---

# Problem Commands

Problems represent what needs to be solved. They can be organized hierarchically with parent-child relationships and assigned to milestones.

## Entity Resolution

All commands that take a problem ID support multiple resolution methods:

- **Fuzzy title match**: `"App crashes"` or `"auth module"` -- matches against problem titles
- **Truncated prefix**: `01957d` -- minimum 6 hex characters from the UUID
- **Full UUID**: `01957d3e-a8b2-7def-8c3a-9f4e5d6c7b8a`

If multiple entities match, an interactive picker appears (TTY) or suggestions are shown (non-TTY).

## `jjj problem new`

Create a new problem.

```
jjj problem new <title> [OPTIONS]
```

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--priority` | string | `medium` | Priority: P0/critical, P1/high, P2/medium, P3/low |
| `--parent` | string | | Parent problem (title, prefix, or UUID) |
| `--milestone` | string | | Milestone to target (title, prefix, or UUID) |

```bash,test
jjj init
jjj problem new "App crashes on startup" --priority high
jjj problem new "Null pointer in auth module" --parent "App crashes"
jjj problem new "Improve test coverage"
```

## `jjj problem list`

List all problems.

```
jjj problem list [OPTIONS]
```

| Flag | Type | Description |
|------|------|-------------|
| `--status` | string | Filter by status (open, in_progress, solved, dissolved) |
| `--tree` | bool | Show hierarchical tree view |
| `--milestone` | string | Filter by milestone |
| `--json` | bool | Output in JSON format |

```bash,test
jjj problem list
jjj problem list --status open
jjj problem list --tree
jjj problem list --json
```

## `jjj problem show`

Show problem details.

```
jjj problem show <problem> [OPTIONS]
```

| Flag | Type | Description |
|------|------|-------------|
| `--json` | bool | Output in JSON format |

```bash,test
jjj problem show "App crashes"
jjj problem show "App crashes" --json
```

## `jjj problem edit`

Edit problem details.

```
jjj problem edit <problem> [OPTIONS]
```

| Flag | Type | Description |
|------|------|-------------|
| `--title` | string | New title |
| `--status` | string | New status (open, in_progress, solved, dissolved) |
| `--priority` | string | Set priority (P0/critical, P1/high, P2/medium, P3/low) |
| `--parent` | string | Set parent problem |

```bash,test
jjj problem edit "App crashes" --title "App crashes on cold startup" --priority critical
```

## `jjj problem tree`

Show problem hierarchy as a tree.

```
jjj problem tree [problem]
```

If no problem is given, shows all root problems and their children.

```bash,test
jjj problem tree
jjj problem tree "App crashes"
```

## `jjj problem solve`

Mark a problem as solved. Requires an accepted solution.

```
jjj problem solve <problem>
```

```bash
jjj problem solve "App crashes"
```

## `jjj problem dissolve`

Mark a problem as dissolved (based on false premises).

```
jjj problem dissolve <problem> [OPTIONS]
```

| Flag | Type | Description |
|------|------|-------------|
| `--reason` | string | Reason for dissolving |

```bash
jjj problem dissolve "test coverage" --reason "Turned out to be expected behavior"
```

## `jjj problem assign`

Assign a problem to a person.

```
jjj problem assign <problem> [OPTIONS]
```

| Flag | Type | Description |
|------|------|-------------|
| `--to` | string | Assignee name (defaults to self) |

```bash
jjj problem assign "App crashes" --to alice
```
