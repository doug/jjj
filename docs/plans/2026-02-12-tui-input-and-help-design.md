# TUI Input and Help Design

## Overview

Add help overlay, item creation, and editing capabilities to the jjj TUI. Currently the TUI only supports navigation and simple status changes (accept/refute/address/dismiss). This design adds:

1. Context-sensitive help overlay (`?`)
2. Inline item creation (`n`)
3. Quick title editing (`e`)
4. Full $EDITOR integration (`E`)

## Design Decisions

- **Hybrid input approach**: Quick single-line input for titles, $EDITOR for full editing
- **Context-sensitive help**: Shows only relevant keybindings for selected item type
- **Contextual creation**: `n` creates a child of the selected item (solution for problem, critique for solution)

## App State Changes

### New Types

```rust
pub enum InputMode {
    Normal,
    Help,
    Input {
        prompt: String,      // "New problem title:"
        buffer: String,      // What user has typed
        action: InputAction, // What to do on Enter
    },
}

pub enum InputAction {
    NewProblem { milestone_id: Option<String> },
    NewSolution { problem_id: String },
    NewCritique { solution_id: String },
    EditTitle { entity_type: EntityType, entity_id: String },
}
```

### UiState Addition

Add field: `input_mode: InputMode` (default: `Normal`)

## Help Overlay

Centered popup showing context-sensitive keybindings:

```
┌─ Help ─────────────────────────┐
│                                │
│  Navigation                    │
│    ↑/↓     Move selection      │
│    ←/→     Collapse/Expand     │
│    Tab     Switch pane         │
│    j/k     Scroll detail       │
│                                │
│  Actions (Problem)             │
│    n       New solution        │
│    s       Mark solved         │
│    d       Dissolve            │
│    e       Edit title          │
│    E       Edit in $EDITOR     │
│                                │
│  Press any key to close        │
└────────────────────────────────┘
```

Actions section changes based on selection:
- **Problem**: n, s, d, e, E
- **Solution**: n, a, r, e, E
- **Critique**: a, d, v, e, E
- **Milestone/Backlog**: n, e, E

## Input Line

Single-line text field at bottom of screen when in Input mode:

```
┌─ Next Actions ─┐┌─ Project Tree ─┐┌─ Details ──────┐
│                ││                ││                │
│   (normal UI)  ││   (normal UI)  ││   (normal UI)  │
│                ││                ││                │
├────────────────┴┴────────────────┴┴────────────────┤
│ New solution title: my new feature█                │
└────────────────────────────────────────────────────┘
```

Behavior:
- Prompt in dim, user input in bright white
- Block cursor at end of input
- Typing appends, Backspace removes
- Enter submits (if non-empty), Esc cancels
- After submit: create entity, flash message, refresh, select new item

## $EDITOR Integration

When `E` pressed on any entity:

1. Write entity to temp file (YAML frontmatter + markdown)
2. Suspend terminal
3. Run `$EDITOR` (or `$VISUAL`, fallback `vi`)
4. Wait for exit
5. Parse edited file, update store
6. Resume terminal, refresh, flash message

Temp file format:
```yaml
---
title: Search is slow
status: open
priority: P1
---

## Description

Users report search taking 5+ seconds.
```

Error handling:
- Non-zero exit: discard, "Edit cancelled"
- Parse failure: discard, "Invalid format"
- No changes: "No changes"

## Key Bindings

### Normal Mode

| Key | Action |
|-----|--------|
| `?` | Enter Help mode |
| `n` | New child item (Input mode) |
| `e` | Edit title (Input mode) |
| `E` | Edit in $EDITOR |
| `s` | Solve problem |
| `v` | Validate critique |
| `a` | Accept solution / Address critique |
| `r` | Refute solution |
| `d` | Dissolve problem / Dismiss critique |

### Input Mode

| Key | Action |
|-----|--------|
| Chars | Append to buffer |
| Backspace | Remove last char |
| Enter | Submit |
| Esc | Cancel |

### Help Mode

| Key | Action |
|-----|--------|
| Any | Return to Normal |

## Implementation Files

- `src/tui/app.rs` - Add InputMode, InputAction, key handlers
- `src/tui/ui.rs` - Add help overlay and input line rendering
- `src/tui/mod.rs` - Export new types

## Future Considerations

- Tab completion for parent selection
- Undo support
- Multi-select for bulk operations
