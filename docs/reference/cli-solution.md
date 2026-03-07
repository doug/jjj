---
title: Solution Commands
description: CLI reference for creating, listing, attaching, accepting, and refuting solutions
---

# Solution Commands

Solutions are conjectures proposed to solve problems. They go through a lifecycle: proposed, submitted, approved, or withdrawn. Solutions can have jj changes attached, be critiqued, and have reviewers assigned whose sign-offs gate acceptance.

## `jjj solution new`

Create a new solution.

```
jjj solution new <title> [OPTIONS]
```

| Flag | Type | Required | Description |
|------|------|----------|-------------|
| `--problem` | string | no | Problem this solution addresses (prompts interactively if not provided) |
| `--supersedes` | string | no | Solution this supersedes (title or UUID prefix) |
| `--reviewer` | string (repeatable) | no | Assign reviewers at creation (e.g., `@alice`) |
| `--force`, `-f` | flag | no | Create even if a similar solution already exists |

When `--problem` is not provided, lists open problems and prompts you to select one interactively. After creation, automatically creates a jj change and attaches it to the solution. The solution stays in `proposed` status until you explicitly run `solution submit`.

```bash,test
jjj init
jjj problem new "Login is too slow"
jjj solution new "Add connection pooling" --problem "Login is too slow"
jjj solution new "Use async auth" --problem "Login is too slow" --supersedes "Add connection"
jjj solution list
```

Assign reviewers at creation:

```bash
jjj solution new "Add caching" --problem "Login is too slow" --reviewer @alice --reviewer @bob
```

When reviewers are assigned, the solution requires all of them to sign off before it can be approved. Sign-offs are recorded via review-type critiques. Review is not required by default -- it is enabled per-solution by assigning reviewers.

## `jjj solution list`

List all solutions.

```
jjj solution list [OPTIONS]
```

| Flag | Type | Description |
|------|------|-------------|
| `--problem` | string | Filter by problem |
| `--status` | string | Filter by status (proposed, submitted, withdrawn, approved) |
| `--search` | string | Search solutions by title text |
| `--json` | bool | Output in JSON format |

```bash,test
jjj solution list --problem "Login is too slow"
jjj solution list --status submitted
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
jjj solution show "Add connection"
jjj solution show "Add connection" --json
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
jjj solution edit "Add connection" --title "Add connection pooling with retry"
```

## `jjj solution attach`

Attach the current jj change to a solution.

```
jjj solution attach <solution_id>
```

```bash
jjj solution attach "Add connection"
```

## `jjj solution detach`

Detach a change from a solution.

```
jjj solution detach <solution_id> [change_id]
```

If no change ID is given, detaches the current change.

```bash
jjj solution detach "Add connection"
jjj solution detach "Add connection" abc123
```

## `jjj solution submit`

Submit a solution for review — opens it for critique.

```
jjj solution submit <solution_id>
```

```bash
jjj solution submit "Add connection"
```

## `jjj solution approve`

Approve a solution — accept critique, integrate code, solve the problem. Requires no open critiques (including critique-based review).

```
jjj solution approve <solution_id> [OPTIONS]
```

| Flag | Type | Description |
|------|------|-------------|
| `--force` | bool | Force approve even with open critiques (sets `force_approved` flag) |
| `--rationale` | string | Reason for accepting |
| `--no-rationale` | flag | Skip the rationale prompt |

The acceptance gate checks that all critiques are resolved (addressed, dismissed, or validated). This includes:

1. **Regular critiques** -- issues raised about the solution's approach
2. **Review critiques** -- review requests (critiques with `--reviewer` flag) that must be addressed by the assigned reviewer

Using `--force` bypasses the check and sets the `force_approved` flag on the solution.

```bash
jjj solution approve "Add connection"
jjj solution approve "Add connection" --force
```

## `jjj solution withdraw`

Refute a solution (criticism showed it will not work).

```
jjj solution withdraw <solution_id> [OPTIONS]
```

| Flag | Type | Description |
|------|------|-------------|
| `--rationale` | string | Reason for refuting |
| `--no-rationale` | flag | Skip the rationale prompt |

```bash
jjj solution withdraw "Add connection"
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
jjj solution assign "Add connection" --to bob
```

## `jjj solution resume`

Resume working on an existing solution. Switches to the solution's most recent jj change, or creates a new change if none exists.

```
jjj solution resume <solution_id>
```

```bash
jjj solution resume "Add connection"
```

## `jjj solution diff`

Show the `jj diff` output for all change IDs attached to a solution.

```
jjj solution diff <solution_id>
```

Prints `=== Change: <id> ===` before each diff. If a change is not present in the local repo, a note is shown and the next change is tried. If no change IDs are attached, prints an informative message.

```bash
jjj solution diff "Add connection"
```

