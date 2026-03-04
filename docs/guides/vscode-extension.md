---
title: VS Code Extension
description: Installing and using the jjj VS Code extension for IDE integration
---

# VS Code Extension

The jjj VS Code extension provides IDE integration for managing problems, solutions, critiques, and milestones directly within Visual Studio Code. It surfaces the full jjj workflow in a unified sidebar tree, with context menu actions, inline critique display, and GitHub sync commands.

## Overview

The extension integrates jjj's Popperian project management model into your editor:

- **Unified Project Tree** — milestones, problems, solutions, and critiques in a single hierarchical view
- **Assignee visibility** — problems, solutions, and milestones show `· @name` in the sidebar
- **Context menu actions** — create, transition, assign, and move items without leaving the tree
- **Inline critiques** — PR review comments appear as critique threads anchored to file locations
- **Drag-and-drop milestone management** — drag problems between milestones directly in the tree
- **GitHub sync** — import issues, sync PR reviews, and push metadata from the command palette
- **Filter mode** — toggle between "Open only" and "All" views with a single button or keyboard shortcut

## Installation

### Prerequisites

- **Visual Studio Code** 1.85.0 or later
- **jjj CLI** installed and accessible in your PATH
- A **jjj-initialized repository** (`jjj init`)

### Build from Source

```bash
cd jjj/vscode
npm install
npm run package
# Creates jjj-vscode-0.1.0.vsix
```

### Install Pre-built VSIX

1. In VS Code, open Command Palette (`Cmd+Shift+P` / `Ctrl+Shift+P`)
2. Run `Extensions: Install from VSIX...`
3. Select the `.vsix` file

### Run in Development Mode

```bash
cd jjj/vscode
code .
# Press F5 to launch Extension Development Host
```

## Getting Started

1. **Open a Jujutsu repository** in VS Code
2. **Click the JJJ icon** in the Activity Bar (left side)
3. **Initialize jjj** if not already done:
   - Run `JJJ: Initialize Repository` from the Command Palette
   - Or run `jjj init` in the terminal

The sidebar will show your project tree once initialized.

## Features

### Unified Project Tree

The sidebar contains a single **Project** tree view. The hierarchy is:

```
Milestone (solved/total)  · @lead
  Problem [status] priority  · @assignee
    Solution [status]  · @assignee
      Critique [severity]
Backlog
  Problem ...
```

Milestones are shown with a `(solved/total)` progress indicator. The **Backlog** node collects problems not assigned to any milestone.

**Filter mode** (default: Open only): In "Open" mode, solved/withdrawn/approved items are hidden. Problems with active children auto-expand. Toggle with the filter button in the view toolbar or `Cmd+Shift+O` / `Ctrl+Shift+O`.

**Tab navigation**: While the tree is focused, `Tab` / `Shift+Tab` cycles through open actionable items.

### Drag-and-Drop Milestone Management

Drag problems from one milestone to another (or to Backlog) directly in the tree. The extension calls `jjj milestone add-problem` and `jjj milestone remove-problem` automatically.

### Context Menu Actions

Right-click any item in the tree for relevant actions:

| Item | Available actions |
|------|-------------------|
| Milestone | New Problem, New Problem in Milestone |
| Problem | New Solution, Solve, Dissolve, Reopen, Assign to Me, Move to Milestone |
| Solution (proposed) | Submit for Review, New Critique, Resume, Withdraw, Assign to Me, Switch to Change |
| Solution (submitted) | LGTM (Sign Off), Approve, New Critique, Resume, Withdraw, Assign to Me, Switch to Change |
| Critique (open/valid) | Address, Dismiss, Validate |

### Inline Critique Display

Critiques with file and line location (imported from PR review threads via `jjj github import`) appear as comment threads anchored to the relevant line in the editor. Thread actions (Address, Dismiss, Validate) are available inline without switching to the tree.

Use **JJJ: Add Critique Here** from the editor right-click menu to raise a critique pinned to the current cursor position.

### Command Palette

Access all jjj commands via `Cmd+Shift+P` / `Ctrl+Shift+P`:

**Problems**
- `JJJ: New Problem` — create a new problem
- `JJJ: Solve Problem` — mark a problem solved
- `JJJ: Dissolve Problem` — mark a problem dissolved
- `JJJ: Reopen Problem` — reopen a solved or dissolved problem

**Solutions**
- `JJJ: New Solution` — create a solution for a problem
- `JJJ: Submit Solution` — submit a proposed solution for review
- `JJJ: Approve Solution` — approve a submitted solution
- `JJJ: Withdraw Solution` — withdraw a proposed or submitted solution
- `JJJ: LGTM (Sign Off)` — sign off on a submitted solution
- `JJJ: Resume Solution` — resume working on a solution

**Critiques**
- `JJJ: New Critique` — raise a critique on a solution
- `JJJ: Add Critique Here` — raise a critique anchored to the current editor line
- `JJJ: Address Critique` — mark a critique as addressed
- `JJJ: Dismiss Critique` — dismiss a critique
- `JJJ: Validate Critique` — confirm a critique as a real flaw

**Milestones**
- `JJJ: New Milestone` — create a milestone

**Navigation**
- `JJJ: Switch to Change` — run `jj edit` to the change ID of the selected solution
- `JJJ: Next Open Item` — focus next open item in tree (`Tab`)
- `JJJ: Previous Open Item` — focus previous open item in tree (`Shift+Tab`)
- `JJJ: Toggle Filter` — toggle open/all filter (`Cmd+Shift+O`)

**GitHub**
- `JJJ: Sync GitHub` — pull PR reviews and issue state
- `JJJ: Import GitHub Issue` — import a GitHub issue as a problem
- `JJJ: GitHub Sync Status` — show sync status in a preview pane

**View**
- `JJJ: Refresh All` — reload all data from the jjj store

## Configuration

Configure the extension in VS Code Settings (`Cmd+,` / `Ctrl+,`):

### `jjj.path`

**Type**: `string`
**Default**: `"jjj"`

Path to the jjj executable. Set this if jjj is not in your system PATH.

```json
{
  "jjj.path": "/Users/yourname/.cargo/bin/jjj"
}
```

### `jjj.filterMode`

**Type**: `"open"` | `"all"`
**Default**: `"open"`

Controls the default tree filter. `"open"` hides completed/solved/approved items; `"all"` shows everything.

```json
{
  "jjj.filterMode": "all"
}
```

## Troubleshooting

### Extension Not Activating

**Symptom**: jjj sidebar doesn't appear

**Solutions**:
1. Ensure VS Code 1.85.0 or later is installed
2. Check that the extension is enabled in Extensions view
3. Reload window: `Developer: Reload Window`

### "jjj command not found"

**Symptom**: Extension shows errors when trying to execute commands

**Solutions**:
1. Verify jjj is installed:
   ```bash
   jjj --version
   ```

2. If jjj is not in PATH, set `jjj.path` in settings:
   ```json
   {
     "jjj.path": "/full/path/to/jjj"
   }
   ```

3. Find jjj location:
   ```bash
   which jjj  # macOS/Linux
   where jjj  # Windows
   ```

### Views Not Showing Data

**Symptom**: Sidebar tree is empty

**Solutions**:
1. Ensure the repository is initialized:
   - Run `JJJ: Initialize Repository` from Command Palette
   - Or run `jjj init` in terminal

2. Check that you are in a Jujutsu repository:
   ```bash
   jj status
   ```

3. Refresh the view:
   - Click the refresh button in the tree toolbar
   - Or run `JJJ: Refresh All` from Command Palette

### "Repository not initialized"

**Symptom**: Welcome view persists after running init

**Solutions**:
1. Reload window after initialization:
   - Command Palette → `Developer: Reload Window`

2. Verify initialization worked:
   ```bash
   jjj status --json
   ```

## Development & Contributing

### Project Structure

```
vscode/
├── package.json               # Extension manifest and registered commands
├── src/
│   ├── extension.ts           # Main entry point
│   ├── cli.ts                 # jjj CLI wrapper
│   ├── commands.ts            # All command handlers
│   ├── cache.ts               # In-memory data cache
│   └── views/
│       └── projectTreeProvider.ts  # Unified tree with drag-and-drop
└── media/                     # Icons
```

### Development Setup

1. **Install dependencies**:
   ```bash
   cd jjj/vscode
   npm install
   ```

2. **Open in VS Code**:
   ```bash
   code .
   ```

3. **Run extension**:
   - Press `F5` to launch Extension Development Host

4. **Run tests**:
   ```bash
   npm test
   ```

5. **Package**:
   ```bash
   npm run package
   ```

## Resources

- **Main Documentation**: [jjj Docs](https://doug.github.io/jjj/)
- **CLI Reference**: [CLI Commands](../reference/cli-workflow.md)
- **GitHub Repository**: [github.com/doug/jjj](https://github.com/doug/jjj)
- **Report Issues**: [GitHub Issues](https://github.com/doug/jjj/issues)
