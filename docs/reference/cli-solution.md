# Solution Commands

Solutions are conjectures proposed to solve problems. They go through a lifecycle: proposed, testing, accepted, or refuted. Solutions can have jj changes attached, be reviewed, and be critiqued.

## `jjj solution new`

Create a new solution.

```
jjj solution new <title> --problem <problem_id> [OPTIONS]
```

| Flag | Type | Required | Description |
|------|------|----------|-------------|
| `--problem` | string | yes | Problem this solution addresses |
| `--supersedes` | string | no | Solution this supersedes (e.g., S-1) |
| `--tag` | string (repeatable) | no | Tags to apply |

```bash,test
jjj init
jjj problem new "Login is too slow"
jjj solution new "Add connection pooling" --problem P-1
jjj solution new "Use async auth" --problem P-1 --supersedes S-1
jjj solution list
```

## `jjj solution list`

List all solutions.

```
jjj solution list [OPTIONS]
```

| Flag | Type | Description |
|------|------|-------------|
| `--problem` | string | Filter by problem |
| `--status` | string | Filter by status (proposed, testing, refuted, accepted) |
| `--json` | bool | Output in JSON format |

```bash,test
jjj solution list --problem P-1
jjj solution list --status testing
jjj solution list --json
```

## `jjj solution show`

Show solution details.

```
jjj solution show <solution_id> [OPTIONS]
```

| Flag | Type | Description |
|------|------|-------------|
| `--json` | bool | Output in JSON format |

```bash,test
jjj solution show S-1
jjj solution show S-1 --json
```

## `jjj solution edit`

Edit solution details.

```
jjj solution edit <solution_id> [OPTIONS]
```

| Flag | Type | Description |
|------|------|-------------|
| `--title` | string | New title |
| `--status` | string | New status |
| `--add-tag` | string (repeatable) | Add tags |
| `--remove-tag` | string (repeatable) | Remove tags |

```bash
jjj solution edit S-1 --title "Add connection pooling with retry"
```

## `jjj solution attach`

Attach the current jj change to a solution.

```
jjj solution attach <solution_id>
```

```bash
jjj solution attach S-1
```

## `jjj solution detach`

Detach a change from a solution.

```
jjj solution detach <solution_id> [change_id]
```

If no change ID is given, detaches the current change.

```bash
jjj solution detach S-1
jjj solution detach S-1 abc123
```

## `jjj solution test`

Move a solution to testing status.

```
jjj solution test <solution_id>
```

```bash
jjj solution test S-1
```

## `jjj solution accept`

Accept a solution. By default, requires no open critiques.

```
jjj solution accept <solution_id> [OPTIONS]
```

| Flag | Type | Description |
|------|------|-------------|
| `--force` | bool | Force accept even with open critiques |

```bash
jjj solution accept S-1
jjj solution accept S-1 --force
```

## `jjj solution refute`

Refute a solution (criticism showed it will not work).

```
jjj solution refute <solution_id>
```

```bash
jjj solution refute S-1
```

## `jjj solution assign`

Assign a solution to a person.

```
jjj solution assign <solution_id> [OPTIONS]
```

| Flag | Type | Description |
|------|------|-------------|
| `--to` | string | Assignee name (defaults to self) |

```bash
jjj solution assign S-1 --to bob
```

## `jjj solution review`

Request review on a solution.

```
jjj solution review <solution_id> <reviewers...>
```

Reviewers are specified as names (e.g., `@alice`, `@bob`).

```bash
jjj solution review S-1 @alice @bob
```

## `jjj solution lgtm`

Mark a solution as reviewed (LGTM).

```
jjj solution lgtm <solution_id>
```

```bash
jjj solution lgtm S-1
```
