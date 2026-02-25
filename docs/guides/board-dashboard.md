---
title: TUI and Status
description: Using the terminal dashboard and status commands to monitor project progress
---

# TUI and Status

jjj provides two views for understanding your project's state: the **TUI** shows an interactive view of next actions and the project tree, and **status** shows your personal context and prioritized next actions on the command line.

## The TUI

Launch the interactive terminal UI:

```bash
jjj ui
```

The TUI provides two panes:

**Next Actions** (left pane) — A prioritized list of items requiring attention, grouped by urgency:

| Category | Meaning |
|----------|---------|
| BLOCKED | Solutions blocked by open critiques |
| READY | Solutions ready to accept (critiques resolved) |
| REVIEW | Solutions waiting for your review |
| WAITING | Solutions in review, waiting on others |
| TODO | Open problems without solutions yet |

**Project Tree** (right pane) — A hierarchical view of the project:

```
Milestones
├── 01957d: Q1 Release
│   ├── 01958a: Search is slow [open]
│   │   └── 01958b: Add search index [review]
│   │       └── 01958c: Missing error handling [open]
│   └── 01958d: Login fails [solved]
└── Backlog
    └── 01958e: Add dark mode [open]
```

### TUI Navigation

| Key | Action |
|-----|--------|
| `Tab` | Switch between panes |
| `↑/↓` | Navigate within pane |
| `←` | Collapse tree node |
| `→` | Expand tree node |
| `j/k` | Scroll detail pane |
| `Space` | Page down in detail |
| `a` | Accept solution |
| `r` | Refute solution |
| `d` | Address critique / Dismiss |
| `q` | Quit |

## Status

The `status` command shows your personal context on the command line: the active solution for your current change, prioritized next actions, and a summary of project health.

### Viewing Status

```bash
jjj status
```

Output:

```
Active: s/01959a "Add Redis caching" -> p/01958f [review]
  Open critiques: 2
    c/01959b: Cache invalidation not handled [high]
    c/01959c: Redis single point of failure [medium]

Next actions:

1. [BLOCKED] s/01959a: Add Redis caching -- 2 open critique(s)
   c/01959b: Cache invalidation not handled [high]
   c/01959c: Redis single point of failure [medium]
   -> jjj critique show "invalidation"

2. [TODO] p/01959d: API rate limiting needed -- No solutions proposed
   -> jjj solution new "title" --problem "rate limiting"

Summary: 4 open problems, 3 in review, 5 open critiques
```

### Status Flags

| Flag | Description |
|------|-------------|
| `--all` | Show all items (no limit) |
| `--mine` | Only show your authored work |
| `--limit N` | Show top N items (default: 5) |
| `--json` | Output as JSON |

```bash
jjj status --all
jjj status --mine --limit 10
jjj status --json
```

### Acting on Status

1. **Open critiques?** Address them first. They are blocking your solutions.
   ```bash
   jjj critique show "invalidation"
   # Understand the concern, then address, dismiss, or validate
   jjj critique address "invalidation"
   ```

2. **Solutions in review?** Address critiques or wait for reviewer sign-off.
   ```bash
   jjj solution new "title" --problem "search slow" --reviewer @bob
   ```

3. **Problems assigned but no solutions?** Propose a solution.
   ```bash
   jjj solution new "Rate limit with token bucket algorithm" --problem "rate limiting"
   ```

4. **Nothing assigned?** Run `jjj status --all` to see all items across the project.

## Combining TUI and Status

- **Quick check**: Use `jjj status` for a command-line summary
- **Interactive work**: Use `jjj ui` for browsing and taking actions
- **Scripting**: Use `jjj status --json` for automation

## Next Steps

- [Problem Solving](problem-solving.md) — Creating and managing problems
- [Critique Guidelines](critique-guidelines.md) — Working with critiques
- [Code Review](code-review.md) — The reviewer workflow
