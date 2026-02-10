# Solution Commands

Solutions are conjectures proposed to solve problems. They go through a lifecycle: proposed, testing, accepted, or refuted. Solutions can have jj changes attached, be critiqued, and have reviewers assigned whose sign-offs gate acceptance.

## Entity Resolution

All commands that take a solution or problem reference support multiple resolution methods:

- **Fuzzy title match**: `"connection pooling"` or `"async auth"` -- matches against titles
- **Truncated prefix**: `01958a` -- minimum 6 hex characters from the UUID
- **Full UUID**: `01958a2b-c3d4-7e5f-6a7b-8c9d0e1f2a3b`

If multiple entities match, an interactive picker appears (TTY) or suggestions are shown (non-TTY).

## `jjj solution new`

Create a new solution.

```
jjj solution new <title> [OPTIONS]
```

| Flag | Type | Required | Description |
|------|------|----------|-------------|
| `--problem` | string | no | Problem this solution addresses (prompts interactively if not provided) |
| `--supersedes` | string | no | Solution this supersedes (title, prefix, or UUID) |
| `--reviewer` | string (repeatable) | no | Assign reviewers at creation (e.g., `@alice`) |

When `--problem` is not provided, lists open problems and prompts you to select one interactively. After creation, automatically creates a jj change, attaches it to the solution, and moves the solution to `testing` status.

```bash,test
jjj init
jjj problem new "Login is too slow"
jjj solution new "Add connection pooling" --problem "Login is too slow"
jjj solution new "Use async auth" --problem "Login" --supersedes "connection pooling"
jjj solution list
```

Assign reviewers at creation:

```bash
jjj solution new "Add caching" --problem "Login" --reviewer @alice --reviewer @bob
```

When reviewers are assigned, the solution requires all of them to sign off before it can be accepted. Sign-offs are recorded via review-type critiques. Review is not required by default -- it is enabled per-solution by assigning reviewers.

## `jjj solution list`

List all solutions.

```
jjj solution list [OPTIONS]
```

| Flag | Type | Description |
|------|------|-------------|
| `--problem` | string | Filter by problem (title, prefix, or UUID) |
| `--status` | string | Filter by status (proposed, testing, refuted, accepted) |
| `--json` | bool | Output in JSON format |

```bash,test
jjj solution list --problem "Login"
jjj solution list --status testing
jjj solution list --json
```

## `jjj solution show`

Show solution details.

```
jjj solution show <solution> [OPTIONS]
```

| Flag | Type | Description |
|------|------|-------------|
| `--json` | bool | Output in JSON format |

```bash,test
jjj solution show "connection pooling"
jjj solution show "pooling" --json
```

## `jjj solution edit`

Edit solution details.

```
jjj solution edit <solution> [OPTIONS]
```

| Flag | Type | Description |
|------|------|-------------|
| `--title` | string | New title |
| `--status` | string | New status |

```bash
jjj solution edit "pooling" --title "Add connection pooling with retry"
```

## `jjj solution attach`

Attach the current jj change to a solution.

```
jjj solution attach <solution>
```

```bash
jjj solution attach "connection pooling"
```

## `jjj solution detach`

Detach a change from a solution.

```
jjj solution detach <solution> [change_id]
```

If no change ID is given, detaches the current change.

```bash
jjj solution detach "pooling"
jjj solution detach "pooling" abc123
```

## `jjj solution test`

Move a solution to testing status.

```
jjj solution test <solution>
```

```bash
jjj solution test "connection pooling"
```

## `jjj solution accept`

Accept a solution. Requires no open critiques (including review critiques).

```
jjj solution accept <solution> [OPTIONS]
```

| Flag | Type | Description |
|------|------|-------------|
| `--force` | bool | Force accept even with open critiques (sets `force_accepted` flag) |

The acceptance gate checks that all critiques are resolved (addressed, dismissed, or validated). This includes:

1. **Regular critiques** -- issues raised about the solution's approach
2. **Review critiques** -- review requests (critiques with `--reviewer` flag) that must be addressed by the assigned reviewer

Using `--force` bypasses the check and sets the `force_accepted` flag on the solution.

```bash
jjj solution accept "pooling"
jjj solution accept "pooling" --force
```

## `jjj solution refute`

Refute a solution (criticism showed it will not work).

```
jjj solution refute <solution>
```

```bash
jjj solution refute "async auth"
```

## `jjj solution assign`

Assign a solution to a person.

```
jjj solution assign <solution> [OPTIONS]
```

| Flag | Type | Description |
|------|------|-------------|
| `--to` | string | Assignee name (defaults to self) |

```bash
jjj solution assign "pooling" --to bob
```

## `jjj solution resume`

Resume working on an existing solution. Switches to the solution's most recent jj change, or creates a new change if none exists.

```
jjj solution resume <solution>
```

```bash
jjj solution resume "connection pooling"
```

