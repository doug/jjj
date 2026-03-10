---
name: jjj
description: Use when working in Jujutsu repositories for problem tracking, solution management, and code review — implements Popperian epistemology (problems → solutions → critiques) directly in version control
---

# jjj (Jujutsu Juggler)

Distributed project management and code review built on Jujutsu. No server, no database — metadata lives in an orphaned `jjj` bookmark and syncs with `jj git push/pull`.

**Key insight:** Jujutsu's stable Change IDs survive rebases and squashes, so metadata stays attached to work even after history rewrites.

## Core Model

```
Problems → Solutions → Critiques
   │           │           │
   │           │           └── Error elimination (blocks approval)
   │           └── Conjectures linked to jj Change IDs
   └── Things that need solving (can nest via --parent)
```

State machines:
- **Problems**: `open` → `in_progress` → `solved` / `dissolved`
- **Solutions**: `proposed` → `submitted` → `approved` / `withdrawn`
- **Critiques**: `open` → `addressed` / `validated` / `dismissed`

## Quick Reference

| Task | Command |
|------|---------|
| Initialize | `jjj init` |
| Status | `jjj status` |
| Next action | `jjj next [--top N] [--mine] [--claim]` |
| Interactive TUI | `jjj ui` |
| Sync | `jjj push` / `jjj fetch` / `jjj sync` |
| Full-text search | `jjj search "query"` |
| Timeline | `jjj timeline "title"` |
| File overlaps | `jjj overlaps` |
| Project insights | `jjj insights` |

### Problems

| Task | Command |
|------|---------|
| Create | `jjj problem new "Title" [--priority critical\|high\|medium\|low]` |
| List | `jjj problem list [--status open\|in_progress\|solved\|dissolved]` |
| Show | `jjj problem show "title or id"` |
| Tree view | `jjj problem tree` |
| Edit | `jjj problem edit "title" --title "New" --priority high` |
| Solve | `jjj problem solve "title"` |
| Dissolve | `jjj problem dissolve "title" --reason "..."` |
| Reopen | `jjj problem reopen "title"` |
| Sub-problem | `jjj problem new "Sub" --parent "parent title"` |
| Assign | `jjj problem assign "title" --to user@example.com` |

### Solutions

| Task | Command |
|------|---------|
| Create | `jjj solution new "Title" --problem "title or id"` |
| List | `jjj solution list [--problem "title"] [--status proposed]` |
| Submit for review | `jjj solution submit "title"` |
| Approve | `jjj solution approve "title" [--rationale "..."] [--force]` |
| Withdraw | `jjj solution withdraw "title" --rationale "..."` |
| Assign | `jjj solution assign "title" --to user@example.com` |

### Critiques

| Task | Command |
|------|---------|
| Add | `jjj critique new "solution" "Issue description" --severity critical\|high\|medium\|low` |
| List | `jjj critique list [--solution "title"] [--status open]` |
| Show | `jjj critique show "title"` |
| Address | `jjj critique address "title"` |
| Validate (confirmed flaw) | `jjj critique validate "title"` |
| Dismiss | `jjj critique dismiss "title"` |
| Reply | `jjj critique reply "title" "comment"` |
| With location | `jjj critique new "sol" "Issue" --file src/foo.rs --line 42` |

### Milestones

| Task | Command |
|------|---------|
| Create | `jjj milestone new "Title" [--date 2026-06-01]` |
| List | `jjj milestone list` |
| Add problem | `jjj milestone add-problem "milestone" "problem"` |
| Roadmap | `jjj milestone roadmap` |
| Status | `jjj milestone status "title"` |
| Assign | `jjj milestone assign "title" --to user@example.com` |

### Events & Audit

| Task | Command |
|------|---------|
| All events | `jjj events` |
| Filter by type | `jjj events --event-type solution_approved` |
| Search rationales | `jjj events --search "RAII"` |
| Scoped to entity | `jjj events --problem "title"` |
| Timeline | `jjj timeline "problem title"` |

## Entity Resolution

All commands accept: full UUID, short prefix (min 6 chars), or fuzzy title match.

```bash
jjj problem show "auth"          # fuzzy match
jjj problem show "019caa6c"      # UUID prefix
jjj solution approve "nil check" # partial title
```

## Core Workflow

```bash
# 1. Identify a problem
jjj problem new "Login crashes on empty password" --priority critical

# 2. Propose a solution (auto-links to current jj change)
jjj solution new "Add nil guard to auth handler" --problem "Login crashes"

# 3. Submit for review
jjj solution submit "nil guard"

# 4. Critique (reviewer)
jjj critique new "nil guard" "Missing test for empty string vs nil" --severity medium

# 5. Address critique (author)
jjj critique address "missing test"

# 6. Approve
jjj solution approve "nil guard" --rationale "Nil guard + tests added"
# Problem auto-transitions to solved when its only solution is approved

# 7. Sync
jjj push
```

## Blocking Rules

- `solution approve` is blocked by Open or Validated critiques
- Use `--force` to override critique check
- `solution approve` requires `solution submit` first (Proposed → Submitted → Approved)
- A Validated critique means the flaw is confirmed; address or dismiss before approving

## JSON Output

Every list/show command accepts `--json` for structured output:

```bash
jjj problem list --json
jjj solution show "title" --json
jjj milestone roadmap --json
jjj events --json
```

## GitHub Integration

```bash
jjj github import 42           # Import PR #42 as solution
jjj github push                # Refresh PR bodies, sync issue state
jjj sync                       # fetch + push jjj metadata
```

## When to Use

Use jjj for all problem tracking, decision records, and code review when the project uses Jujutsu (jj). The metadata persists across rebases, making it reliable for long-running work.

- Check `jjj status` first — it shows what needs attention
- Use `jjj next` to find the highest-priority open item, or `jjj next --claim` to grab it
- Use `jjj overlaps` to check for file conflicts between active solutions
- Use `jjj insights` to review project health and cycle times
- Add critiques liberally; they're cheap to address and create a clear audit trail
- Rationales on `solution approve` and `solution withdraw` are searchable via `jjj events --search`
