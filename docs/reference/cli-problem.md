# Problem Commands

Problems represent what needs to be solved. They can be organized hierarchically with parent-child relationships, assigned to milestones, and tagged.

## `jjj problem new`

Create a new problem.

```
jjj problem new <title> [OPTIONS]
```

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--priority` | string | `medium` | Priority: P0/critical, P1/high, P2/medium, P3/low |
| `--parent` | string | | Parent problem ID for sub-problems |
| `--milestone` | string | | Milestone to target |
| `--tag` | string (repeatable) | | Tags to apply |

```bash,test
jjj init
jjj problem new "App crashes on startup" --priority high
jjj problem new "Null pointer in auth module" --parent P-1
jjj problem new "Improve test coverage" --tag testing --tag backend
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
jjj problem show <problem_id> [OPTIONS]
```

| Flag | Type | Description |
|------|------|-------------|
| `--json` | bool | Output in JSON format |

```bash,test
jjj problem show P-1
jjj problem show P-1 --json
```

## `jjj problem edit`

Edit problem details.

```
jjj problem edit <problem_id> [OPTIONS]
```

| Flag | Type | Description |
|------|------|-------------|
| `--title` | string | New title |
| `--status` | string | New status (open, in_progress, solved, dissolved) |
| `--priority` | string | Set priority (P0/critical, P1/high, P2/medium, P3/low) |
| `--parent` | string | Set parent problem |
| `--add-tag` | string (repeatable) | Add tags |
| `--remove-tag` | string (repeatable) | Remove tags |

```bash,test
jjj problem edit P-1 --title "App crashes on cold startup" --priority critical
jjj problem edit P-1 --add-tag urgent --remove-tag testing
```

## `jjj problem tree`

Show problem hierarchy as a tree.

```
jjj problem tree [problem_id]
```

If no problem ID is given, shows all root problems and their children.

```bash,test
jjj problem tree
jjj problem tree P-1
```

## `jjj problem solve`

Mark a problem as solved. Requires an accepted solution.

```
jjj problem solve <problem_id>
```

```bash
jjj problem solve P-1
```

## `jjj problem dissolve`

Mark a problem as dissolved (based on false premises).

```
jjj problem dissolve <problem_id> [OPTIONS]
```

| Flag | Type | Description |
|------|------|-------------|
| `--reason` | string | Reason for dissolving |

```bash
jjj problem dissolve P-2 --reason "Turned out to be expected behavior"
```

## `jjj problem assign`

Assign a problem to a person.

```
jjj problem assign <problem_id> [OPTIONS]
```

| Flag | Type | Description |
|------|------|-------------|
| `--to` | string | Assignee name (defaults to self) |

```bash
jjj problem assign P-1 --to alice
```
