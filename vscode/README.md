# JJJ -- Popperian Project Management for Jujutsu

Manage problems, solutions, and critiques directly in VS Code. All metadata is stored in an orphaned jj bookmark and synced via `jj git push` -- no server, no database, fully offline.

## Features

### Project Tree View

A sidebar tree displays your project hierarchy: milestones, problems, solutions, and critiques. Expand any node to see its children. Context menus provide actions appropriate to each entity type and state.

### Inline Code Critiques

Critique comment threads are anchored to source lines. When code is edited, anchors fuzzy re-attach to the correct location. Create a critique from the editor context menu ("Add Critique Here") or from the tree. Address, dismiss, or validate critiques directly from the inline thread controls.

### Solution Tracking

Full lifecycle management for solutions: propose, submit for review, approve, or withdraw. Each solution is linked to a jj Change ID, so it survives rebases and squashes. Resume working on any solution from the context menu.

### Status Bar

The status bar shows your active solution and summary counts of open problems and critiques across the project.

### Drag-and-Drop

Move problems between milestones by dragging them in the project tree.

### GitHub Sync

Import GitHub issues as problems, create pull requests from solutions, and sync review comments as critiques. Available through the command palette and tree view toolbar.

### Tab Navigation

When the project tree is focused, press Tab and Shift+Tab to jump between open items without reaching for the mouse.

### Filter Toggle

Switch between showing all items or only open/active ones. The filter state is reflected in the tree view toolbar icon.

## Getting Started

### 1. Install Jujutsu (jj)

Follow the [jj installation guide](https://github.com/jj-vcs/jj#installation).

### 2. Install jjj

```
cargo install jjj
```

Or download a pre-built binary from [releases](https://github.com/doug/jjj/releases). See the full [installation guide](https://github.com/doug/jjj#installation) for more options.

### 3. Initialize in your repository

```
cd your-jj-repo
jjj init
```

The extension will automatically detect jjj and populate the project tree.

## Requirements

- [Jujutsu (jj)](https://github.com/jj-vcs/jj) version control
- [jjj CLI](https://github.com/doug/jjj) on PATH (or set `jjj.path` in settings)
- A jj repository with `jjj init` run

## Configuration

| Setting | Description | Default |
|---------|-------------|---------|
| `jjj.path` | Path to the jjj executable | `"jjj"` |
| `jjj.filterMode` | Tree view filter mode: `"all"` shows everything, `"open"` hides completed items | `"open"` |

## Keyboard Shortcuts

| Shortcut | Action | Context |
|----------|--------|---------|
| Cmd+Shift+O (Ctrl+Shift+O on Windows/Linux) | Toggle filter between all and open | Global |
| Tab | Next open item | Project tree focused |
| Shift+Tab | Previous open item | Project tree focused |

## License

Apache-2.0
