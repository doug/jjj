# VS Code Extension: Unified Tree View

## Problem

The current VS Code extension has two separate panels:
- **Next Actions** - Flat list of items needing attention (blocked, ready, review, etc.)
- **Project Tree** - Hierarchical view (Milestones → Problems → Solutions → Critiques)

This is confusing because both show the same entities. Users must mentally map between them ("this blocked solution in Next Actions... where is it in the tree?").

## Solution

Replace both panels with a single **Project** tree view that supports two modes:
- **"All"** - Full hierarchy showing everything
- **"Open"** - Same hierarchy, filtered to hide completed/closed items

Tab navigation always jumps to the next open item, enabling quick triage regardless of mode.

## Design

### Filter Modes

**"All" mode:** Shows complete hierarchy including solved problems, accepted solutions, addressed critiques.

**"Open" mode:** Hides items where:
- Problems: `status === "solved" || status === "dissolved"`
- Solutions: `status === "accepted" || status === "refuted"`
- Critiques: `status === "addressed" || status === "valid" || status === "dismissed"`

Milestones are hidden if they contain no open items.

### Toggle UI

**Header button:**
- Filter icon in tree view title bar actions
- Filled funnel (`$(filter-filled)`) when "Open" mode active
- Outline funnel (`$(filter)`) when "All" mode active
- Tooltip: "Show Open Only" / "Show All"

**Keyboard shortcut:**
- `Cmd+Shift+O` (macOS) / `Ctrl+Shift+O` (Windows/Linux)
- Command: `jjj.toggleTreeFilter`

**Tree title:**
- "Project" when showing all
- "Project (Open)" when filtered

**Persistence:**
- Filter state saved to workspace settings
- Default: "Open" mode for new workspaces

### Tab Navigation

- `Tab` moves selection to next open/actionable item
- `Shift+Tab` moves to previous open item
- Works in both modes (in "All" mode, skips completed items)
- Follows visual tree order (depth-first)
- Skips milestones (containers, not actionable)
- Wraps around at end/beginning
- No-op if no open items exist

### Auto-Expand Behavior

**"Open" mode:**
- Auto-expands milestones containing open problems
- Auto-expands problems with open solutions/critiques
- Collapse state not persisted (always re-expands)

**"All" mode:**
- User-controlled collapse/expand state respected and persisted

### Status Bar

Remove the status bar. The tree view with filter toggle serves this purpose.

## Implementation

### Files to Delete

- `vscode/src/views/nextActionsProvider.ts`

### Files to Modify

**`vscode/src/views/projectTreeProvider.ts`:**
- Add `filterMode: "all" | "open"` state
- Add `toggleFilter()` method
- Update `getChildren()` to filter based on mode
- Add `getNextOpenItem()` / `getPrevOpenItem()` for Tab navigation

**`vscode/src/extension.ts`:**
- Remove `NextActionsProvider` registration
- Remove `StatusBar` registration
- Register `jjj.toggleTreeFilter` command
- Register `jjj.nextOpenItem` and `jjj.prevOpenItem` commands

**`vscode/package.json`:**
- Remove `jjj-next-actions` view
- Add commands: `toggleTreeFilter`, `nextOpenItem`, `prevOpenItem`
- Add keybindings (Tab/Shift+Tab when tree focused, Cmd+Shift+O global)
- Add menu contribution for filter button in tree title

### Files to Delete References From

- `vscode/src/statusBar.ts` - delete entirely

## Future Extensibility

This design supports adding inline action buttons to tree items. For example, a "jump to change" icon on solutions:

```json
"menus": {
  "view/item/context": [
    {
      "command": "jjj.openInJj",
      "when": "viewItem == solution",
      "group": "inline"
    }
  ]
}
```

Solutions already store `change_id`, enabling `jj edit <change_id>` or similar actions.

## Not Changing

- CLI commands (`jjj status`, etc.)
- Rust TUI
- Entity documents, critique decorations, cache layer
