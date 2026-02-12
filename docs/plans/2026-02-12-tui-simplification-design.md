# TUI Simplification Design

## Overview

Simplify the TUI from three panes to two by merging Next Actions into the Project Tree. Action items are highlighted with symbols, and Tab cycles through them.

## Goals

- Reduce confusion between duplicate views (Next Actions vs Tree)
- Improve visual clarity with consistent colors and symbols
- Make navigation more intuitive with Tab jumping to action items

## Layout Change

**Before:** Next Actions (20%) | Tree (35%) | Detail (45%)

**After:** Tree (40%) | Detail (60%)

## Action Item Symbols

Items requiring attention get a prefix symbol:

| Symbol | Category | Meaning |
|--------|----------|---------|
| `⚡` | Ready | Solution with all critiques resolved |
| `🚫` | Blocked | Solution with open critiques |
| `⏳` | Waiting | Assigned to someone else |
| `📋` | Todo | Problem with no solutions |
| `👀` | Review | Critique assigned to you |

Non-action items have no prefix.

## Color System

### Status Colors

**Problems:**
- Open → White
- InProgress → Yellow
- Solved → Green
- Dissolved → DarkGray

**Solutions:**
- Proposed → Cyan
- Testing → Yellow
- Accepted → Green
- Refuted → Red

**Critiques:**
- Open → Yellow
- Valid → Red
- Addressed → Green
- Dismissed → DarkGray

### Priority Indication (Problems)

- P0 → Red `🔴` prefix
- P1 → Yellow `🟡` prefix
- P2 → No prefix (default)
- P3 → Dimmed text

## Tree Visualization

Indent guides show hierarchy:

```
▼ Milestone Alpha
│ ├─ 🔴 ⚡ Fix auth bug              ← P0 priority + ready
│ │  └─ Proposed solution
│ │     └─ Open critique
│ └─ 📋 Add logging                  ← todo, P2
▼ Backlog
│ └─ ⏳ Refactor API                 ← waiting
```

## Selection Highlight

- Selected row: reverse video (white on cyan)
- Clear visual distinction from non-selected items

## Tree Filter Mode

Toggle between two views with `f`:

**Full Tree (default):** Shows all items with action symbols highlighting what needs attention.

**Actions Only:** Shows only items requiring attention plus their parent nodes (to maintain hierarchy). Non-actionable items are hidden.

```
Full Tree Mode:                    Actions Only Mode:
▼ Milestone Alpha                  ▼ Milestone Alpha
│ ├─ 🔴 ⚡ Fix auth bug            │ └─ 🔴 ⚡ Fix auth bug
│ │  └─ Proposed solution          ▼ Backlog
│ │     └─ Open critique           │ └─ ⏳ Refactor API
│ └─ 📋 Add logging
│ └─ Completed task (no symbol)
▼ Backlog
│ └─ ⏳ Refactor API
```

Title bar shows current mode: `Project Tree` or `Project Tree [Actions]`

## Navigation

| Key | Action |
|-----|--------|
| Tab | Jump to next action item (wraps) |
| Shift+Tab | Jump to previous action item |
| ↑/↓ | Normal tree navigation |
| ←/→ | Collapse/expand nodes |
| f | Toggle tree filter (full/actions only) |

All other keys unchanged (n/e/E/s/o/a/r/d/v/?/q).

## Implementation

**Modify:**
- `src/tui/ui.rs` - Remove next actions pane, update tree rendering
- `src/tui/app.rs` - Remove FocusedPane, implement Tab jumping
- `src/tui/tree.rs` - Add action_symbol to FlatTreeItem

**Remove:**
- `draw_next_actions` function
- `FocusedPane` enum
- `next_actions_index` field

**Keep:**
- `src/tui/next_actions.rs` logic (reused for symbol annotation)
