# Quick Start

Get up and running with jjj in 5 minutes. This walkthrough covers the full
Problem/Solution/Critique workflow.

## Initialize

Navigate to an existing Jujutsu repository and initialize jjj:

```bash,test
jjj init
```

This creates a shadow graph -- an orphaned commit history that stores all
project metadata separately from your code. The bookmark `jjj/meta` tracks it.

## Create a Problem

Problems describe things that need solving. Create one with a title and
priority:

```bash,test
jjj problem new "Search is slow" --priority P1
```

This creates `P-1`. Priorities range from P0 (critical) to P3 (low) and
affect how `jjj next` orders your work.

## Propose a Solution

Solutions are conjectures -- proposed ways to solve a problem. They start in
`proposed` status and must survive criticism before acceptance.

```bash,test
jjj solution new "Add search index" --problem P-1
```

This creates `S-1`, linked to problem `P-1`. A single problem can have
multiple competing solutions.

## Start Working

The `start` command creates a new jj change, attaches it to the solution, and
moves the solution to `testing` status:

```bash
jjj start S-1
```

Your working copy now has a change tracked by jjj. The parent problem
automatically moves to `in_progress`.

## Add a Critique

Critiques are explicit criticism of a solution. They block acceptance until
every critique is resolved (addressed, validated, or dismissed).

```bash
jjj critique new S-1 "Missing error handling" --severity medium
```

This creates `CQ-1` against solution `S-1`. Severities are `low`, `medium`,
`high`, and `critical`.

## Address the Critique

After modifying the solution to handle the criticism, mark the critique as
addressed:

```bash
jjj critique address CQ-1
```

Other resolution options:

- `jjj critique validate CQ-1` -- the critique is correct and the solution
  should be refuted
- `jjj critique dismiss CQ-1` -- the critique is incorrect or irrelevant

## Submit

When your work is ready, submit squashes the change and completes the
solution:

```bash
jjj submit
```

Submit auto-accepts the solution if all critiques are resolved and reviews
pass. If this is the only solution for the problem, it offers to mark the
problem as solved too.

## Check What's Next

The `next` command shows a prioritized list of actions you should take:

```bash
jjj next
```

Items are grouped by urgency:

| Category  | Meaning                                          |
|-----------|--------------------------------------------------|
| BLOCKED   | Solutions blocked by open critiques               |
| READY     | Solutions ready to accept (critiques resolved)    |
| REVIEW    | Solutions waiting for your review                 |
| WAITING   | Solutions in testing, waiting on others            |
| TODO      | Open problems without solutions yet                |

## View the Board

The board gives you a Kanban-style overview of all solutions grouped by
status:

```bash
jjj board
```

Press `q` to exit the TUI.

## Key Concepts

**Change IDs** -- jjj uses Jujutsu change IDs (not commit hashes) as stable
references. They survive rebases and history rewrites, so metadata links never
break.

**Shadow Graph** -- All jjj metadata lives in a separate orphaned commit
history (`jjj/meta`). It never pollutes your project history and can be
pushed or fetched independently.

**Critique Blocking** -- A solution cannot be accepted while it has open
critiques. All criticism must be explicitly addressed, validated, or dismissed.
This enforces intellectual honesty.

**Priority** -- Problems are prioritized P0 (critical) through P3 (low).
Priority affects how `jjj next` orders work items, ensuring the most important
problems surface first.

## Common Commands

| Command | Description |
|---------|-------------|
| `jjj init` | Initialize jjj in current repository |
| `jjj problem new "title" --priority P1` | Create a problem |
| `jjj problem list` | List all problems |
| `jjj problem show P-1` | Show problem details and solutions |
| `jjj solution new "title" --problem P-1` | Propose a solution |
| `jjj solution list` | List all solutions |
| `jjj start S-1` | Create a change and begin working |
| `jjj critique new S-1 "title" --severity medium` | Critique a solution |
| `jjj critique address CQ-1` | Mark critique as addressed |
| `jjj submit` | Squash, accept solution, solve problem |
| `jjj next` | Show prioritized next actions |
| `jjj board` | Show Kanban board |
| `jjj dashboard` | Show overview dashboard |
| `jjj review @alice` | Request review on current solution |
| `jjj lgtm` | Approve current solution |
| `jjj milestone new "v1.0" --date 2025-06-01` | Create a milestone |
| `jjj milestone roadmap` | Show milestone roadmap |

## Next Steps

- [Your First Project](first-project.md) -- Complete walkthrough of a real workflow
- [CLI Reference](../reference/cli.md) -- Full command documentation
