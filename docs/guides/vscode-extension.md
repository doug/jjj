# VSCode Extension

The jjj VSCode extension provides IDE integration for managing tasks, features, milestones, bugs, and code reviews directly within Visual Studio Code.

> **Note**
>
> The VSCode extension is in early development (v0.0.1). Basic views and infrastructure are implemented. Advanced features are planned for future releases.

## Overview

The extension integrates jjj's project management capabilities into your editor:

**Currently Implemented:**
- Sidebar views for Tasks, Features, Milestones, Bugs, and Reviews
- Command palette integration for jjj commands
- Welcome view for initializing repositories
- Refresh functionality for all views
- Basic command structure

**Planned Features:**
- Interactive Kanban board webview
- Drag-and-drop task management
- Planning and roadmap visualizations
- Dashboard with project overview
- Inline code review with comments
- Status bar integration

## Installation

### Prerequisites

- **Visual Studio Code** 1.70.0 or later
- **jjj CLI** installed and accessible in your PATH
- A **jjj-initialized repository**

### Development Installation

Currently, the extension is not published to the VSCode Marketplace. To use it:

#### Method 1: Build from Source

```bash
# Clone the repository
git clone https://github.com/doug/jjj.git
cd jjj/vscode

# Install dependencies
npm install

# Compile the extension
npm run compile

# Package the extension
npm run package
# This creates jjj-vscode-extension-0.0.1.vsix
```

#### Method 2: Install Pre-built VSIX

If a `.vsix` file is available:

1. In VSCode, open Command Palette (`Cmd+Shift+P` / `Ctrl+Shift+P`)
2. Run `Extensions: Install from VSIX...`
3. Select the `jjj-vscode-extension-0.0.1.vsix` file

#### Method 3: Run in Development Mode

```bash
cd jjj/vscode
code .
# Press F5 to launch Extension Development Host
```

## Getting Started

### Initialize a Repository

1. **Open a Jujutsu repository** in VSCode
2. **Open the jjj sidebar**:
   - Click the beaker icon in the Activity Bar (left side)
3. **Initialize jjj**:
   - Click "Initialize Repository" in the Welcome view
   - Or run `JJJ: Initialize Repository` from Command Palette

### First Steps

Once initialized, the sidebar shows:
- **Milestones** - Release targets and sprints
- **Features** - User-facing functionality
- **Tasks** - Work items for features
- **Bugs** - Issues to fix
- **Reviews** - Code review requests

## Features

### Sidebar Views

The jjj sidebar provides tree views for all project work items.

#### Milestones View

Shows all milestones in your project with target dates and status.

**Actions:**
- Click to open milestone details
- Right-click for context menu
- Refresh button to sync with latest data
- Create new milestone (`+` button)

#### Features View

Displays features grouped by status or milestone.

**Actions:**
- Expand to see linked tasks
- Click to open feature details
- Create new feature (`+` button)
- Refresh to update

#### Tasks View

Lists tasks with grouping options (by column, feature, or assignee).

**Actions:**
- Click task to view details
- Toggle grouping with group-by button
- Refresh tasks
- Create new task (`+` button)

**Grouping Options:**
- By column (TODO, In Progress, Done, etc.)
- By feature
- By assignee

#### Bugs View

Shows bugs organized by severity or status.

**Actions:**
- View bug details
- Link bugs to features
- Create new bug (`+` button)
- Refresh view

#### Reviews View

Displays pending code reviews.

**Actions:**
- View review details
- See review status
- Refresh reviews

### Command Palette

Access jjj commands via Command Palette (`Cmd+Shift+P` / `Ctrl+Shift+P`):

**Available Commands:**
- `JJJ: Initialize Repository` - Set up jjj in current repo
- `JJJ: Open Task` - View task details
- `JJJ: Open Feature` - View feature details
- `JJJ: Open Milestone` - View milestone details
- `JJJ: Open Bug` - View bug details
- `JJJ: Open Review` - View review details
- `JJJ: Create Task` - Create a new task
- `JJJ: Create Feature` - Create a new feature
- `JJJ: Create Milestone` - Create a new milestone
- `JJJ: Create Bug` - Report a new bug
- `JJJ: Open Dashboard` - Open dashboard view (planned)
- `JJJ: Open Kanban Board` - Open Kanban board (planned)
- `JJJ: Open Planning View` - Open planning view (planned)
- `JJJ: Refresh All Views` - Refresh all sidebar views
- `JJJ: Refresh Tasks` - Refresh tasks view
- `JJJ: Refresh Features` - Refresh features view
- `JJJ: Refresh Milestones` - Refresh milestones view
- `JJJ: Refresh Bugs` - Refresh bugs view
- `JJJ: Refresh Reviews` - Refresh reviews view
- `JJJ: Toggle Task Grouping` - Change task grouping mode

### Webview Panels (Planned)

The following webview panels are planned for future releases:

#### Kanban Board
Interactive board showing tasks organized by column with drag-and-drop support.

#### Planning View
Hierarchical view of milestones, features, and tasks with progress tracking.

#### Dashboard
Overview panel showing project status, pending reviews, and task summaries.

## Configuration

Configure the extension in VSCode Settings (`Cmd+,` / `Ctrl+,`):

### jjj.path

**Type**: `string`
**Default**: `"jjj"`

Path to the jjj executable. Set this if jjj is not in your system PATH.

```json
{
  "jjj.path": "/usr/local/bin/jjj"
}
```

**Example**: If you installed jjj in a custom location:
```json
{
  "jjj.path": "/Users/yourname/.cargo/bin/jjj"
}
```

## Development Status

### Implemented (v0.0.1)

- ✅ Extension scaffolding and activation
- ✅ Sidebar container with activity bar icon
- ✅ Welcome view for uninitialized repositories
- ✅ Tree view providers for:
  - Milestones
  - Features
  - Tasks
  - Bugs
  - Reviews
- ✅ Command palette integration
- ✅ Refresh commands for all views
- ✅ Toggle task grouping
- ✅ Create commands for all work item types
- ✅ JJJ CLI wrapper (`jjj.ts`)
- ✅ Custom document provider for jjj files
- ✅ Basic testing infrastructure

### Planned (Future Releases)

- ⏳ **v0.2.0** - Webview Panels
  - Interactive Kanban board
  - Planning view with hierarchy
  - Dashboard panel

- ⏳ **v0.3.0** - Code Review Integration
  - Inline comment widgets
  - Gutter decorations
  - Diff view integration
  - Approve/request changes actions

- ⏳ **v0.4.0** - Advanced Features
  - Status bar integration
  - Context menus with actions
  - Keyboard shortcuts
  - CodeLens for task IDs
  - Hover tooltips
  - Quick picks for operations

- ⏳ **v1.0.0** - Production Release
  - Full feature set complete
  - Published to VSCode Marketplace
  - Performance optimizations
  - Comprehensive documentation

## Troubleshooting

### Extension Not Activating

**Symptom**: jjj sidebar doesn't appear

**Solutions**:
1. Ensure VSCode 1.70.0 or later is installed
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
   which jjj  # On macOS/Linux
   where jjj  # On Windows
   ```

### Views Not Showing Data

**Symptom**: Sidebar views are empty

**Solutions**:
1. Ensure repository is initialized:
   - Run `JJJ: Initialize Repository` from Command Palette
   - Or run `jjj init` in terminal

2. Check that you're in a Jujutsu repository:
   ```bash
   jj status
   ```

3. Try refreshing views:
   - Run `JJJ: Refresh All Views`
   - Or click refresh button in each view

### "Repository not initialized"

**Symptom**: Welcome view persists after running init

**Solutions**:
1. Reload window after initialization:
   - Command Palette → `Developer: Reload Window`

2. Verify initialization worked:
   ```bash
   jjj board --json
   ```

3. Re-open the workspace folder

## Development & Contributing

The VSCode extension is part of the jjj project and welcomes contributions!

### Project Structure

```
vscode/
├── package.json          # Extension manifest
├── src/
│   ├── extension.ts      # Main entry point
│   ├── jjj.ts           # CLI wrapper
│   ├── views/           # Tree view providers
│   │   ├── taskProvider.ts
│   │   ├── featureProvider.ts
│   │   ├── milestoneProvider.ts
│   │   ├── bugProvider.ts
│   │   ├── kanbanPanel.ts
│   │   ├── dashboardPanel.ts
│   │   └── planningPanel.ts
│   ├── editors/         # Custom editors
│   │   └── jjjDocumentProvider.ts
│   └── test/            # Test suites
└── media/               # Icons and styles
```

### Development Setup

1. **Clone the repository**:
   ```bash
   git clone https://github.com/doug/jjj.git
   cd jjj/vscode
   ```

2. **Install dependencies**:
   ```bash
   npm install
   ```

3. **Open in VSCode**:
   ```bash
   code .
   ```

4. **Run extension**:
   - Press `F5` to launch Extension Development Host
   - Test your changes in the new window

5. **Run tests**:
   ```bash
   npm test
   ```

### Making Changes

1. Modify code in `src/`
2. Compile: `npm run compile`
3. Test in Extension Development Host (F5)
4. Run tests: `npm test`
5. Submit a pull request

### Testing

The extension includes:
- Unit tests for providers
- Integration tests with mock jjj CLI
- Test fixtures with mock data

Run tests:
```bash
npm test                    # All tests
npm run test:unit          # Unit tests only
npm run test:integration   # Integration tests
```

## Resources

- **Main Documentation**: [jjj Docs](https://doug.github.io/jjj/)
- **CLI Reference**: [CLI Commands](../reference/cli.md)
- **GitHub Repository**: [github.com/doug/jjj](https://github.com/doug/jjj)
- **Report Issues**: [GitHub Issues](https://github.com/doug/jjj/issues)
- **Discussions**: [GitHub Discussions](https://github.com/doug/jjj/discussions)

## Feedback

We'd love your feedback on the extension!

- 🐛 **Report bugs**: [GitHub Issues](https://github.com/doug/jjj/issues)
- 💡 **Request features**: [GitHub Discussions](https://github.com/doug/jjj/discussions)
- ⭐ **Star the project**: [GitHub](https://github.com/doug/jjj)
- 📖 **Read the docs**: [Documentation](https://doug.github.io/jjj/)
