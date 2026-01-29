# Solution Commands

Solutions are conjectures proposed to solve problems. They go through a lifecycle: proposed, testing, accepted, or refuted. Solutions can have jj changes attached, be critiqued, and have reviewers assigned whose sign-offs gate acceptance.

## `jjj solution new`

Create a new solution.

```
jjj solution new <title> [OPTIONS]
```

| Flag | Type | Required | Description |
|------|------|----------|-------------|
| `--problem` | string | no | Problem this solution addresses (prompts interactively if not provided) |
| `--supersedes` | string | no | Solution this supersedes (e.g., s1) |
| `--review` | string (repeatable) | no | Assign reviewers at creation (e.g., `@alice`) |

When `--problem` is not provided, lists open problems and prompts you to select one interactively. After creation, automatically creates a jj change, attaches it to the solution, and moves the solution to `testing` status.

```bash,test
jjj init
jjj problem new "Login is too slow"
jjj solution new "Add connection pooling" --problem p1
jjj solution new "Use async auth" --problem p1 --supersedes s1
jjj solution list
```

Assign reviewers at creation:

```bash
jjj solution new "Add caching" --problem p1 --review @alice --review @bob
```

When reviewers are assigned, the solution requires all of them to sign off before it can be accepted. Review is not required by default -- it is enabled per-solution by assigning reviewers.

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
jjj solution list --problem p1
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
jjj solution show s1
jjj solution show s1 --json
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

```bash
jjj solution edit s1 --title "Add connection pooling with retry"
```

## `jjj solution attach`

Attach the current jj change to a solution.

```
jjj solution attach <solution_id>
```

```bash
jjj solution attach s1
```

## `jjj solution detach`

Detach a change from a solution.

```
jjj solution detach <solution_id> [change_id]
```

If no change ID is given, detaches the current change.

```bash
jjj solution detach s1
jjj solution detach s1 abc123
```

## `jjj solution test`

Move a solution to testing status.

```
jjj solution test <solution_id>
```

```bash
jjj solution test s1
```

## `jjj solution accept`

Accept a solution. Requires no open critiques and, if reviewers are assigned, all reviewers must have signed off.

```
jjj solution accept <solution_id> [OPTIONS]
```

| Flag | Type | Description |
|------|------|-------------|
| `--force` | bool | Force accept even with open critiques or missing sign-offs (sets `force_accepted` flag) |

The acceptance gate checks two conditions in order:

1. **No open critiques** -- all critiques must be resolved (addressed, dismissed, or validated).
2. **All assigned reviewers signed off** -- every reviewer in the `reviewers` list must have an LGTM sign-off.

Non-assigned sign-offs (from people not in the `reviewers` list) are recorded but do not affect the gate.

Using `--force` bypasses both checks and sets the `force_accepted` flag on the solution.

```bash
jjj solution accept s1
jjj solution accept s1 --force
```

## `jjj solution refute`

Refute a solution (criticism showed it will not work).

```
jjj solution refute <solution_id>
```

```bash
jjj solution refute s1
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
jjj solution assign s1 --to bob
```

## `jjj solution review`

Assign reviewers to a solution. Reviewers must sign off before the solution can be accepted.

```
jjj solution review <solution_id> <reviewers...>
```

Reviewers are specified as names (e.g., `@alice`, `@bob`). Adding reviewers makes the solution require sign-offs from all assigned reviewers.

```bash
jjj solution review s1 @alice @bob
```

## `jjj solution resume`

Resume working on an existing solution. Switches to the solution's most recent jj change, or creates a new change if none exists.

```
jjj solution resume <solution_id>
```

```bash
jjj solution resume s1
```

## `jjj solution lgtm`

Sign off on a solution (LGTM). Records a structured sign-off with the reviewer's name, timestamp, and optional comment.

```
jjj solution lgtm <solution_id> [OPTIONS]
```

| Flag | Type | Description |
|------|------|-------------|
| `--comment` | string | Optional comment to include with the sign-off |

If the reviewer is in the solution's `reviewers` list, the sign-off counts toward the acceptance gate. Sign-offs from non-assigned reviewers are recorded but do not affect the gate.

```bash
jjj solution lgtm s1
jjj solution lgtm s1 --comment "looks good"
```
