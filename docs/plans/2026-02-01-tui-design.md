# TUI Design: `jjj ui`

## Overview

A full-screen terminal interface for jjj that provides an interactive experience matching the VS Code extension. Replaces the old `jjj board` command.

## Layout

Three-column split view:

```
┌───────────┬─────────────────┬────────────────────────┐
│ Next      │ Project Tree    │ Detail                 │
│ Actions   │                 │                        │
│           │ ▼ Milestone 1   │ Problem P-42           │
│ • P-42    │   ▼ P-42: Auth  │ ────────────────────   │
│   blocked │     ○ S-15      │ Status: in_progress    │
│           │       • C-7     │ Priority: high         │
│ • S-15    │                 │                        │
│   ready   │ ▼ Backlog       │ Description:           │
│           │   ○ P-50        │ Users need to...       │
├───────────┴─────────────────┴────────────────────────┤
│ P-42 (in_progress): [n]ew solution [s]olve [d]issolve│
│ [Tab] pane │ [/] commands │ [j/k] scroll │ [q] quit  │
└──────────────────────────────────────────────────────┘
```

- **Next Actions** (narrow left sidebar): Prioritized actionable items - blocked, ready for review, waiting, etc.
- **Project Tree** (center): Hierarchical view - Milestones → Problems → Solutions → Critiques
- **Detail Pane** (right): Full info for the selected item - updates as you navigate

## Navigation

### Pane Focus
- `Tab` toggles focus between Next Actions and Project Tree (highlighted border shows focus)
- Detail Pane never has focus - it passively displays the selected item

### Within Focused Pane
- `↑/↓` move selection up/down

### Tree Expand/Collapse (Project Tree only)
- `→` expands node, or moves to first child if already expanded
- `←` collapses node, or moves to parent if already collapsed

### Detail Pane Scrolling
- `j/k` scroll line by line
- `Space` scrolls page down
- Works regardless of which pane has focus

### Cross-Pane Sync
- Selecting an item in Next Actions auto-expands the Project Tree to reveal and highlight that same item
- Navigating Project Tree does not affect Next Actions (one-way sync)

## Actions & Hotkeys

### Context-Sensitive Hotkeys

| Key | Problem | Solution | Critique |
|-----|---------|----------|----------|
| `n` | new solution | new critique | — |
| `a` | — | accept | address |
| `r` | — | refute | — |
| `d` | dissolve | — | dismiss |
| `s` | solve | — | — |
| `e` | edit | edit | edit |
| `Enter` | action menu | action menu | action menu |

### Global Hotkeys
- `p` - new problem
- `m` - new milestone
- `/` - open command palette
- `?` - help overlay
- `q` - quit

### Command Palette (`/`)
- Fuzzy-searchable list of commands
- Scoped sections: actions for current selection at top, global commands below
- Arrow keys to navigate, Enter to execute, Escape to close

### Two-Line Footer
- Top line: context-sensitive actions for selected item
- Bottom line: global navigation shortcuts (always visible)

## Input & Feedback

### Text Input (Bottom Input Line)
- Appears at the bottom of the screen, above the footer
- Content stays visible for reference while typing
- Single-line inputs: type and press Enter
- Multi-field inputs (e.g., new critique): sequential prompts

```
─────────────────────────────────────────────────────────────
 New problem title: Implement user authentication_
 [Enter] create │ [Esc] cancel
```

For multi-step input like new critique:
```
 Critique title: Missing error handling_     → Enter
 Severity [l]ow [m]edium [h]igh [c]ritical: _ → press key
 → Created C-12
```

### Feedback
- **Success**: Flash message in the context line area (~2 seconds), then restores normal hints
- **Errors**: Modal dialog that must be dismissed

```
┌─ Error ───────────────────────────────────┐
│                                           │
│  Cannot accept solution S-15:             │
│  2 unresolved critical critiques          │
│                                           │
│  [Enter] dismiss                          │
└───────────────────────────────────────────┘
```

## Colors & Visual Styling

### Semantic Colors (Matching VS Code Extension)

| Status | Color | Examples |
|--------|-------|----------|
| Green | Done/Good | Accepted, Solved, Addressed |
| Red | Blocked/Bad | Refuted, Blocked, Critical severity |
| Yellow | In Progress | Testing, In Progress, High severity |
| Cyan | Open/Pending | Proposed, Open, Medium severity |
| Default (dim) | Low priority | Pending, Low severity |

### Selection Styling
- Focused pane: bright border
- Selected item: reverse video or bold with background highlight
- Unfocused pane: dim border

### Tree Indicators
- `▼` expanded node
- `▶` collapsed node
- `○` leaf node (no children)
- `•` leaf with content/activity

Uses standard ANSI colors so terminal themes can adjust them. Bold and dim used as fallbacks for emphasis.

## Implementation Notes

- Uses `ratatui` and `crossterm` (already in project dependencies)
- Command: `jjj ui`
- Removes: `jjj board` (replaced by this)
- Existing `tui.rs` provides a starting point but needs significant expansion
