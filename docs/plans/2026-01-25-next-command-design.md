# `jjj next` Command Design

## Overview

A guided workflow command that answers "what should I work on?" with prioritized, actionable suggestions.

## Goals

1. **Reduce cognitive load** - Don't make users hunt through lists
2. **Prioritize blockers** - Surface unaddressed criticism first (Popperian)
3. **Actionable output** - Every item includes a suggested command
4. **Tooling-friendly** - JSON output for VS Code plugin integration

## Priority Order

Items are shown in this order:

1. **BLOCKED** - Your solutions with open critiques
2. **READY** - Solutions ready to accept (critiques resolved + LGTM)
3. **REVIEW** - Solutions where you're a requested reviewer (haven't LGTM'd)
4. **WAITING** - Your solutions awaiting review from others
5. **TODO** - Open problems with no solutions (or only refuted ones)

### Within-Category Sorting

| Category | Sort Order |
|----------|------------|
| BLOCKED | Highest severity critique first |
| READY | Oldest first (clear the backlog) |
| REVIEW | Oldest request first |
| WAITING | Oldest first (informational) |
| TODO | Current milestone first, then creation date |

## CLI Interface

```bash
jjj next [OPTIONS]

Options:
  --all          Show all items (no limit)
  --mine         Only my authored work (exclude review requests)
  --limit <N>    Show top N items (default: 5)
  --json         Output as JSON for tooling/plugins
```

### Default Behavior

- Shows top 5 items
- Includes "my work" (solutions I authored, problems assigned to me)
- Includes "things needing my review" (solutions where I'm requested reviewer)
- Uses jj identity (`jj config get user.email`) to determine "me"

## Output Format

### Human-Readable (Default)

```
$ jjj next

Next actions:

1. [BLOCKED] S-3: Fix authentication - 2 open critiques
   CQ-5: SQL injection risk [high]
   → jjj critique show CQ-5

2. [BLOCKED] S-7: Add caching - 1 open critique
   CQ-8: Cache invalidation unclear [medium]
   → jjj critique show CQ-8

3. [READY] S-1: Refactor database layer
   All critiques resolved, LGTM from @bob
   → jjj solution accept S-1

4. [REVIEW] S-9: Update API endpoints (by @charlie)
   Requested 2 days ago, no critiques yet
   → jjj solution show S-9

5. [TODO] P-4: Search is slow on large datasets
   No solutions proposed
   → jjj start "solution title" --problem P-4

Showing 5 of 12 items. Use --all to see everything.
```

### JSON Output

```bash
$ jjj next --json
```

```json
{
  "items": [
    {
      "priority": 1,
      "category": "blocked",
      "entity_id": "S-3",
      "entity_type": "solution",
      "title": "Fix authentication",
      "summary": "2 open critiques",
      "suggested_command": "jjj critique show CQ-5",
      "details": [
        { "id": "CQ-5", "text": "SQL injection risk", "severity": "high" },
        { "id": "CQ-6", "text": "Missing validation", "severity": "medium" }
      ]
    },
    {
      "priority": 2,
      "category": "ready",
      "entity_id": "S-1",
      "entity_type": "solution",
      "title": "Refactor database layer",
      "summary": "All critiques resolved, LGTM from @bob",
      "suggested_command": "jjj solution accept S-1",
      "details": []
    },
    {
      "priority": 3,
      "category": "review",
      "entity_id": "S-9",
      "entity_type": "solution",
      "title": "Update API endpoints",
      "summary": "Review requested by @charlie",
      "suggested_command": "jjj solution show S-9",
      "author": "charlie@example.com",
      "requested_at": "2026-01-23T10:00:00Z",
      "details": []
    },
    {
      "priority": 4,
      "category": "waiting",
      "entity_id": "S-2",
      "entity_type": "solution",
      "title": "Update user model",
      "summary": "Awaiting review from @alice",
      "suggested_command": null,
      "waiting_on": ["alice"],
      "details": []
    },
    {
      "priority": 5,
      "category": "todo",
      "entity_id": "P-4",
      "entity_type": "problem",
      "title": "Search is slow on large datasets",
      "summary": "No solutions proposed",
      "suggested_command": "jjj start \"solution title\" --problem P-4",
      "milestone_id": "M-2",
      "details": []
    }
  ],
  "total_count": 12,
  "shown_count": 5,
  "user": "doug@example.com"
}
```

## Data Model

```rust
pub struct NextItem {
    pub priority: usize,
    pub category: NextCategory,
    pub entity_id: String,
    pub entity_type: EntityType,
    pub title: String,
    pub summary: String,
    pub suggested_command: Option<String>,
    pub details: Vec<NextItemDetail>,
}

pub enum NextCategory {
    Blocked,
    Ready,
    Review,
    Waiting,
    Todo,
}

pub enum EntityType {
    Solution,
    Problem,
}

pub struct NextItemDetail {
    pub id: String,
    pub text: String,
    pub severity: Option<CritiqueSeverity>,
}
```

## Implementation Logic

```rust
pub fn compute_next_items(
    user: &str,
    problems: &[Problem],
    solutions: &[Solution],
    critiques: &[Critique],
    config: &Config,
    options: &NextOptions,
) -> Vec<NextItem> {
    let mut items = Vec::new();

    // 1. BLOCKED: My solutions with open critiques
    for solution in solutions.iter().filter(|s| s.author == user) {
        let open_critiques: Vec<_> = critiques
            .iter()
            .filter(|c| c.solution_id == solution.id && c.status == CritiqueStatus::Open)
            .collect();

        if !open_critiques.is_empty() {
            items.push(NextItem {
                category: NextCategory::Blocked,
                entity_id: solution.id.clone(),
                // ... build item with highest severity critique
            });
        }
    }

    // 2. READY: My solutions with all critiques resolved + LGTM
    for solution in solutions.iter().filter(|s| s.author == user) {
        if can_accept(solution, critiques, config).is_ok() {
            items.push(NextItem {
                category: NextCategory::Ready,
                entity_id: solution.id.clone(),
                suggested_command: Some(format!("jjj solution accept {}", solution.id)),
                // ...
            });
        }
    }

    // 3. REVIEW: Solutions where I'm requested but haven't LGTM'd
    if !options.mine_only {
        for solution in solutions.iter() {
            if solution.requested_reviewers.contains(&user.to_string())
                && !solution.reviewed_by.contains(&user.to_string())
            {
                items.push(NextItem {
                    category: NextCategory::Review,
                    entity_id: solution.id.clone(),
                    // ...
                });
            }
        }
    }

    // 4. WAITING: My solutions awaiting review
    for solution in solutions.iter().filter(|s| s.author == user) {
        let requires_review = solution.requires_review
            .unwrap_or(config.review.default_required);

        if requires_review && !has_valid_lgtm(solution) {
            items.push(NextItem {
                category: NextCategory::Waiting,
                entity_id: solution.id.clone(),
                // ...
            });
        }
    }

    // 5. TODO: Open problems with no active solutions
    for problem in problems.iter().filter(|p| p.is_open()) {
        let has_active_solution = solutions
            .iter()
            .any(|s| s.problem_id == problem.id && !s.is_resolved());

        if !has_active_solution {
            items.push(NextItem {
                category: NextCategory::Todo,
                entity_id: problem.id.clone(),
                // ...
            });
        }
    }

    // Sort by priority, apply limit
    items.sort_by_key(|i| (i.category.priority(), i.secondary_sort_key()));

    if let Some(limit) = options.limit {
        items.truncate(limit);
    }

    items
}
```

## VS Code Plugin Integration

### 1. Sidebar Panel - "Next Actions"

- Calls `jjj next --json` on workspace open and file save
- Renders prioritized list with category icons
- Click item → runs suggested command or opens relevant file

```
┌─ JJJ: NEXT ACTIONS ─────────────────┐
│ 🔴 S-3: Fix auth (2 critiques)      │
│    └─ CQ-5: SQL injection [high]    │
│ ✅ S-1: Refactor DB (ready)         │
│ 👁 S-9: Review API update (@charlie)│
│ ⏳ S-2: Waiting on @alice           │
│ 📝 P-4: Search is slow (no solution)│
└─────────────────────────────────────┘
```

### 2. Status Bar Item

- Shows summary: "jjj: 2 blocked, 1 ready"
- Click → opens sidebar panel
- Color coding:
  - Red: blocked items exist
  - Yellow: review requests pending
  - Green: only ready/todo items

### 3. Inline Code Annotations

- When viewing a file with code-level critiques:
  - Gutter icons at critique locations
  - Hover → shows critique text and reply thread
  - Click → opens critique detail or reply input

### 4. Command Palette

| Command | Action |
|---------|--------|
| JJJ: Show Next Actions | Opens sidebar panel |
| JJJ: Accept Ready Solution | Quick-accept if exactly one ready |
| JJJ: Address Top Critique | Jump to highest priority blocked critique |
| JJJ: Refresh Next Actions | Force refresh of sidebar |

### 5. Notifications

- New review request → toast notification with "Show" button
- Solution becomes ready → notify "S-1 ready to accept"
- New critique on your solution → notify with severity indicator

### 6. Tree View Data Provider

```typescript
interface NextActionsProvider extends TreeDataProvider<NextItem> {
  refresh(): void;
  getChildren(element?: NextItem): NextItem[];
  getTreeItem(element: NextItem): TreeItem;
}

// Refresh triggers
vscode.workspace.onDidSaveTextDocument(() => provider.refresh());
setInterval(() => provider.refresh(), 30000); // Every 30s
```

## Files to Create/Modify

### New Files

- `src/commands/next.rs` - Command implementation

### Modified Files

- `src/cli.rs` - Add `NextAction` enum and CLI parsing
- `src/commands/mod.rs` - Add next module

## Summary

| Aspect | Decision |
|--------|----------|
| Priority order | Blocked → Ready → Review → Waiting → Todo |
| Default scope | My work + things needing my review |
| Output format | Numbered action list with suggested commands |
| Default limit | Top 5, with `--all` and `--limit N` |
| Identity | Use jj identity |
| Tooling | `--json` flag for VS Code plugin |

The `jjj next` command makes the Popperian workflow practical by surfacing what matters most: unaddressed criticism first, then quick wins, then new work.
