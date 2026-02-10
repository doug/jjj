# Quick Start

Get up and running with jjj in 5 minutes. This walkthrough covers the full
Problem/Solution/Critique workflow.

## Initialize

Navigate to an existing Jujutsu repository and initialize jjj:

```bash,test
jjj init
```

This creates a shadow graph -- an orphaned commit history that stores all
project metadata separately from your code. The bookmark `jjj` tracks it.

## Create a Problem

Problems describe things that need solving. Create one with a title and
priority:

```bash,test
jjj problem new "Search is slow" --priority P1
```

This creates a problem with a UUID7 identifier (e.g., `01957d3e-a8b2-7def-8c3a-9f4e5d6c7b8a`).
In listings, you'll see a truncated prefix like `01957d`. Priorities range from
P0 (critical) to P3 (low) and affect how `jjj status` orders your work.

## Propose a Solution

Solutions are conjectures -- proposed ways to solve a problem. They start in
`proposed` status and must survive criticism before acceptance.

```bash,test
jjj solution new "Add search index" --problem "Search is slow"
```

This creates a solution linked to the problem. You can reference entities by:
- **Fuzzy title match**: `"Search is slow"` or even `"search slow"`
- **Truncated prefix**: `01957d` (minimum 6 hex characters)
- **Full UUID**: `01957d3e-a8b2-7def-8c3a-9f4e5d6c7b8a`

A single problem can have multiple competing solutions.

## Start Working

The `solution new` command creates a solution, a new jj change, attaches it,
and moves the solution to `testing` status. To resume work on an existing
solution:

```bash
jjj solution resume "Add search index"
```

Your working copy now has a change tracked by jjj. The parent problem
automatically moves to `in_progress`.

## Add a Critique

Critiques are explicit criticism of a solution. They block acceptance until
every critique is resolved (addressed, validated, or dismissed).

```bash
jjj critique new "Add search index" "Missing error handling" --severity medium
```

This creates a critique against the solution. Severities are `low`, `medium`,
`high`, and `critical`.

## Address the Critique

After modifying the solution to handle the criticism, mark the critique as
addressed:

```bash
jjj critique address "Missing error"
```

Other resolution options:

- `jjj critique validate "Missing error"` -- the critique is correct and the solution
  should be refuted
- `jjj critique dismiss "Missing error"` -- the critique is incorrect or irrelevant

## Submit

When your work is ready, submit squashes the change and completes the
solution:

```bash
jjj submit
```

Submit auto-accepts the solution if all critiques are resolved and all
assigned reviewers have signed off. If this is the only solution for the
problem, it offers to mark the problem as solved too.

## Check What's Next

The `status` command shows a prioritized list of actions you should take:

```bash
jjj status
```

Items are grouped by urgency:

| Category  | Meaning                                          |
|-----------|--------------------------------------------------|
| BLOCKED   | Solutions blocked by open critiques               |
| READY     | Solutions ready to accept (critiques resolved)    |
| REVIEW    | Solutions waiting for your review                 |
| WAITING   | Solutions in testing, waiting on others            |
| TODO      | Open problems without solutions yet                |

## View the TUI

The interactive TUI gives you an overview of next actions and the project tree:

```bash
jjj ui
```

Press `q` to exit.

## Key Concepts

**Entity IDs** -- All entities use UUID7 identifiers that are time-ordered for
natural chronological sorting. You can reference entities by full UUID,
truncated hex prefix (minimum 6 characters), or fuzzy title match.

**Fuzzy Search** -- The primary way to reference entities. Type part of the
title and jjj finds matches. If multiple entities match, an interactive picker
appears (in TTY) or you get a list of suggestions (in scripts).

**Change IDs** -- jjj uses Jujutsu change IDs (not commit hashes) as stable
references. They survive rebases and history rewrites, so metadata links never
break.

**Shadow Graph** -- All jjj metadata lives in a separate orphaned commit
history (`jjj`). It never pollutes your project history and can be
pushed or fetched independently.

**Critique Blocking** -- A solution cannot be accepted while it has open
critiques. All criticism must be explicitly addressed, validated, or dismissed.
This enforces intellectual honesty.

**Priority** -- Problems are prioritized P0 (critical) through P3 (low).
Priority affects how `jjj status` orders work items, ensuring the most important
problems surface first.

## Common Commands

| Command | Description |
|---------|-------------|
| `jjj init` | Initialize jjj in current repository |
| `jjj status` | Show status and prioritized next actions |
| `jjj ui` | Launch interactive TUI |
| `jjj problem new "title" --priority high` | Create a problem |
| `jjj problem list` | List all problems |
| `jjj problem show "auth bug"` | Show problem details (fuzzy search) |
| `jjj solution new "title" --problem "auth bug"` | Propose a solution |
| `jjj solution new "title" --problem "auth" --reviewer @alice` | Propose with reviewer |
| `jjj solution list` | List all solutions |
| `jjj solution resume "pooling"` | Switch to a solution's change |
| `jjj critique new "pooling" "title" --severity medium` | Critique a solution |
| `jjj critique address "error handling"` | Mark critique as addressed |
| `jjj submit` | Squash, accept solution, solve problem |
| `jjj push` | Push code and metadata to remote |
| `jjj fetch` | Fetch code and metadata from remote |
| `jjj milestone new "v1.0" --date 2025-06-01` | Create a milestone |
| `jjj milestone roadmap` | Show milestone roadmap |

## Next Steps

- [Problem Solving Guide](../guides/problem-solving.md) -- Deep dive into the problem lifecycle
- [Critique Guidelines](../guides/critique-guidelines.md) -- Writing and responding to critiques
- [CLI Reference](../reference/cli-workflow.md) -- Full command documentation
