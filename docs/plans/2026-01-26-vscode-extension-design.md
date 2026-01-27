# VS Code Extension Design

## Overview

A clean rebuild of the JJJ VS Code extension around the Popperian model (Problems, Solutions, Critiques). The extension is a pure UI layer over the `jjj` CLI, communicating via `--json` flags. It targets developers who prefer GUI-based interaction over command-line workflows.

## Goals

1. **Answer "what should I do?"** - Next Actions as the primary view
2. **Inline code critiques** - Review and comment directly in the editor, like GitHub PR comments
3. **Organize visually** - Drag-and-drop milestone planning in a project tree
4. **Stay native** - Use VS Code primitives (QuickPick, CodeLens, virtual documents, tree views) — no custom frameworks

## Architecture

```
VS Code Extension (TypeScript)
    │
    ├── Tree Views ──── Next Actions, Project Tree
    ├── Virtual Docs ── Problem/Solution/Critique detail views
    ├── Inline UI ───── Gutter icons, comment threads, CodeLens
    ├── QuickPicks ──── Autocomplete for assignees, tags, reviewers
    │
    └── CLI Wrapper ─── Shells out to `jjj <command> --json`
                        Parses JSON responses into typed interfaces
```

All data flows through the CLI. The extension never reads `.jjj/` files directly. Auto-refreshes on file save, after command execution, and on a 30-second polling interval.

## Sidebar (2 Views)

The JJJ Activity Bar icon opens two views.

### 1. Next Actions (Primary)

Renders `jjj next --json` as a prioritized action list. Each item shows:

- Category badge: BLOCKED (red), READY (green), REVIEW (blue), WAITING (gray), TODO (white)
- Entity ID and title
- One-line summary (e.g., "2 open critiques", "LGTM from @bob")
- Inline action icon per category:
  - BLOCKED: navigates to top critique's code location
  - READY: one-click accept
  - REVIEW: opens solution detail
  - TODO: starts a new solution

### 2. Project Tree

Hierarchical tree: Milestones > Problems > Solutions > Critiques.

```
▼ v0.2 — Release (3/5 solved)
  ▼ P-1: Search is slow [open]
    └─ S-1: Add search index [testing]
         └─ CQ-3: SQL injection risk [high]
  ▶ P-2: Auth broken [solved]
▼ v0.3 — Performance (0/1 solved)
  ▶ P-10: Slow page loads [open]
▼ Backlog (4 problems)
  ▶ P-11: Dark mode [open]
  ▶ P-12: Mobile layout [open]
```

**Backlog** is a virtual catch-all milestone at the bottom containing problems not assigned to any milestone.

**Drag-and-drop:** Drag problems between milestones (including to/from Backlog). Runs `jjj milestone add-problem` / `remove-problem` under the hood.

**Status icons:** Colored dots or icons on each node indicating status (open, in-progress, solved, testing, accepted, refuted).

**Progress:** Each milestone header shows "N/M solved" fraction.

## Status Bar

A persistent status bar item: `JJJ: 2 blocked, 1 ready`

- Red when blocked items exist
- Yellow when review requests pending
- Green when only ready/todo items
- Click opens the Next Actions view

## Virtual Documents

Clicking any entity in the tree opens a virtual document in an editor tab. The document has two zones:

### Editable Zone (type to change, Cmd+S to save)

- **Title** — the heading line
- **Description** — the body text (markdown)

Saving runs `jjj problem edit` / `jjj solution edit` / `jjj critique edit` with the updated content.

### Read-Only Zone (changed via actions)

A styled header block showing metadata: status, assignee, tags, reviewers, timestamps. Changed through CodeLens action buttons or QuickPick commands.

### Problem Document

```
P-1: Search is slow                                    ← editable
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Status: open  │  Milestone: v0.2  │  Assignee: @doug
Tags: performance, database
[Solve] [Dissolve] [New Solution] [Assign...] [Tag...]
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

## Description                                         ← editable

Search queries take 3+ seconds on datasets over
100k rows. Users are reporting timeouts...

## Solutions (2)
▸ S-1: Add search index [testing] — 2 critiques
▸ S-4: Switch to ElasticSearch [proposed] — 0 critiques
```

### Solution Document

```
S-1: Add search index                                  ← editable
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Status: testing  │  Problem: P-1  │  Assignee: @doug
Reviewers: @alice (requested), @bob (LGTM)
Tags: performance, db
[Accept] [Refute] [Request Review] [LGTM] [Assign...] [Tag...]
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

## Description                                         ← editable

Add a B-tree index on the search column to
improve query performance from O(n) to O(log n)...

## Critiques (2 open)
- CQ-3: SQL injection risk [high] — src/db.rs:42
- CQ-5: Missing validation [medium]

## Changes
- kxq2p: Add index migration
- mw8v3: Update query planner
```

### Critique Document

```
CQ-3: SQL injection risk                               ← editable
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Status: open  │  Solution: S-1  │  Severity: high
Location: src/db.rs:42  │  Author: @alice
[Address] [Dismiss] [Validate] [Severity...]
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

## Argument                                            ← editable

The query on line 42 concatenates user input
directly into the SQL string...

## Evidence                                            ← editable

```rust
let query = format!("SELECT * FROM users WHERE id = {}", user_input);
```

## Discussion (2 replies)

@bob — Jan 25, 10:30
Good catch, I'll use parameterized queries.

@alice — Jan 25, 10:45
Thanks! Marking as addressed once I see the fix.

[Reply...]
```

## Inline Critiques in the Editor

When a source file has critiques with code locations:

### Gutter Icons

- Yellow icon: medium/low severity critique
- Red icon: high/critical severity critique
- Gray icon: addressed/dismissed critique
- Hover shows critique title and severity

### Inline Comment Thread

Click a gutter icon to expand a comment thread below the line:

```
 41 │ fn get_user(id: &str) -> User {
 42 │ ●  let query = format!("SELECT * ...", id);
    │ ┌─── CQ-3: SQL injection risk [high] ── @alice ───┐
    │ │ The query concatenates user input directly into  │
    │ │ the SQL string. Use parameterized queries.       │
    │ │                                                  │
    │ │ @bob — Jan 25, 10:30                             │
    │ │ Good catch, I'll use parameterized queries.      │
    │ │                                                  │
    │ │ ┌──────────────────────────────────────────────┐ │
    │ │ │ Type a reply...                              │ │
    │ │ └──────────────────────────────────────────────┘ │
    │ │                    [Address] [Dismiss] [Reply]   │
    │ └──────────────────────────────────────────────────┘
 43 │     db.execute(&query)?;
```

### Creating a New Critique

Right-click a line in the editor > "JJJ: Add Critique Here":

```
┌─ New Critique on src/api/errors.rs:58 ──────────────┐
│ Title:    [                                    ]     │
│ Severity: [medium ▾]                                 │
│ Solution: [S-9: Update API endpoints ▾]              │
│                                                      │
│ Argument:                                            │
│ ┌──────────────────────────────────────────────────┐ │
│ │                                                  │ │
│ │                                                  │ │
│ └──────────────────────────────────────────────────┘ │
│                              [Cancel] [Create]       │
└──────────────────────────────────────────────────────┘
```

The Solution picker defaults to the solution associated with the current jj change (same logic as `jjj lgtm` shorthand).

## QuickPick Autocomplete

Metadata fields use VS Code QuickPick for selection with type-to-filter search.

### Assignee / Reviewers

Source: committer list from `jj log --no-graph -r 'all()' -T 'author\n'`, deduplicated and cached on activation.

```
┌─ Assign to...                                   ┐
│ 🔍 ali                                           │
│ ──────────────────────────────────────────────── │
│ Alice Smith (alice@example.com)              ✓   │
│ Alicia Jones (alicia@company.com)                │
└──────────────────────────────────────────────────┘
```

### Tags

Source: `jjj tag list --json`. Multi-select enabled. Existing tags pre-checked. Option to create new tags inline.

```
┌─ Tags (select multiple)                          ┐
│ 🔍 perf                                          │
│ ──────────────────────────────────────────────── │
│ ☑ performance                                    │
│ ☐ perftest                                       │
│ ──────────────────────────────────────────────── │
│ + Create "perf" as new tag                       │
└──────────────────────────────────────────────────┘
```

### Severity

Simple single-select: low, medium, high, critical.

### Status

Not a picker. Status changes through semantic action buttons: [Accept], [Refute], [Solve], [Dissolve], [Address], [Dismiss]. These are verbs, not a dropdown of states.

## Context Menus

### Tree View Context Menus

| On a... | Actions |
|---------|---------|
| Milestone | New Problem, Edit, Delete |
| Problem | New Solution, Edit, Solve, Dissolve, Move to Milestone..., Assign..., Tag... |
| Solution | New Critique, Edit, Accept, Refute, Request Review, LGTM, Start Working, Assign..., Tag... |
| Critique | Address, Dismiss, Validate, Reply, Edit |
| Backlog | New Problem, New Milestone |

### Editor Context Menu

Right-click on a line in any source file:

- **JJJ: Add Critique Here** — Opens inline critique form for that line

Right-click on a gutter icon:

- **Address** / **Dismiss** / **Validate** — Quick status changes
- **Reply** — Focus the reply text box
- **Open Critique** — Opens the full critique virtual document

## Command Palette

All actions available via Cmd+Shift+P, prefixed with "JJJ:":

| Command | Behavior |
|---------|----------|
| JJJ: New Problem | Input box for title, creates problem |
| JJJ: New Solution | QuickPick to select problem, input for title |
| JJJ: New Critique | QuickPick for solution, input for title/severity |
| JJJ: Accept Solution | QuickPick of testing solutions |
| JJJ: Request Review | QuickPick for reviewers |
| JJJ: LGTM | Applies to current change's solution |
| JJJ: Start Working | QuickPick of open problems, input for solution title |
| JJJ: Show Next Actions | Opens Next Actions view |
| JJJ: New Milestone | Input box for title |

## Auto-Refresh

The extension refreshes data from the CLI on:

- **File save** — any file in the workspace
- **After command execution** — any JJJ command that modifies state
- **30-second polling interval** — catches external changes (teammate ran CLI)
- **Manual refresh** — button in each view header

## User Journeys

### Journey 1: Developer Starting Their Day

1. Open VS Code. JJJ activity bar icon shows red badge (2 blocked items).
2. Click icon. Next Actions loads: 2 blocked, 1 ready, 1 todo.
3. Click accept on the READY item. Toast confirms. View refreshes.
4. Click the top BLOCKED critique. Source file opens at the critique's line. Inline thread expands.
5. Fix the code. Right-click gutter > "Mark Addressed". Critique turns gray.
6. Next Actions refreshes. Solution drops from BLOCKED to READY.
7. Click the TODO item. Input box prompts for solution title. New solution created, `jjj start` runs.

### Journey 2: Reviewing Someone's Solution

1. Next Actions shows a REVIEW item from @charlie.
2. Click it. Solution virtual document opens showing description, changes, and critiques.
3. Click a change ID to open the changed files.
4. Spot an issue. Right-click line > "JJJ: Add Critique Here". Fill in title, severity, argument. Click Create.
5. Gutter icon appears. Critique shows in the solution document.
6. Everything else looks good. Click [LGTM] in the solution document. Toast confirms.
7. @charlie's Next Actions updates: S-9 now shows as BLOCKED with the new critique.

### Journey 3: Organizing Work into Milestones

1. Open Project Tree. Milestones show with progress fractions.
2. Drag problems from Backlog into milestones for sprint planning.
3. Drag problems between milestones to reprioritize.
4. Right-click Backlog > "New Milestone" to create a new cycle.
5. Right-click a milestone > "New Problem" to add work directly.
6. Expand a problem > right-click > "New Solution" to propose an approach.

## Technology

- **Language:** TypeScript
- **VS Code API:** TreeDataProvider (with drag-and-drop), TextDocumentContentProvider (virtual docs), DecorationProvider (gutter icons), CodeLensProvider, WebviewProvider (inline critique forms)
- **CLI communication:** Child process spawning `jjj <command> --json`, JSON.parse responses
- **Data caching:** In-memory cache refreshed on triggers above
- **Dependencies:** Minimal — js-yaml for parsing, no frameworks

## Files to Create

```
vscode-jjj/
├── package.json                 # Extension manifest
├── tsconfig.json
├── src/
│   ├── extension.ts             # Activation, command registration
│   ├── cli.ts                   # CLI wrapper (spawn, parse JSON)
│   ├── cache.ts                 # Data cache with refresh logic
│   ├── views/
│   │   ├── nextActionsProvider.ts    # Next Actions tree view
│   │   └── projectTreeProvider.ts    # Project Tree (milestones/problems/solutions/critiques)
│   ├── documents/
│   │   ├── entityDocumentProvider.ts # Virtual document content provider
│   │   └── entityDocument.ts         # Document rendering (Problem, Solution, Critique)
│   ├── editor/
│   │   ├── critiqueDecorationProvider.ts  # Gutter icons for critiques
│   │   ├── critiqueCodeLensProvider.ts    # Action buttons in virtual docs
│   │   ├── inlineCritiqueThread.ts        # Inline comment thread webview
│   │   └── newCritiqueForm.ts             # Inline new-critique form
│   ├── quickpicks/
│   │   ├── assigneePicker.ts        # Committer autocomplete
│   │   ├── tagPicker.ts             # Tag multi-select with create
│   │   ├── reviewerPicker.ts        # Reviewer multi-select
│   │   └── entityPicker.ts          # Generic entity picker (problems, solutions)
│   ├── commands/
│   │   ├── problemCommands.ts       # New, solve, dissolve, assign, tag
│   │   ├── solutionCommands.ts      # New, accept, refute, review, lgtm, start
│   │   ├── critiqueCommands.ts      # New, address, dismiss, validate, reply
│   │   └── milestoneCommands.ts     # New, edit, delete, add/remove problem
│   └── statusBar.ts                 # Status bar item
└── test/
    ├── suite/
    │   ├── cli.test.ts
    │   ├── nextActions.test.ts
    │   ├── projectTree.test.ts
    │   └── entityDocument.test.ts
    └── runTests.ts
```

## Summary

| Aspect | Decision |
|--------|----------|
| Sidebar | 2 views: Next Actions + Project Tree |
| Project Tree | Milestones > Problems > Solutions > Critiques, with Backlog catch-all |
| Detail views | Virtual documents (editable title/description, action buttons for metadata) |
| Code critiques | Gutter icons + inline comment threads + right-click to create |
| Metadata editing | QuickPick with autocomplete (assignees from jj log, tags from jjj tag list) |
| Milestone organization | Drag-and-drop in Project Tree |
| Data source | CLI `--json` output, never reads .jjj/ directly |
| Refresh | File save + post-command + 30s polling + manual |
