# VSCode Extension for jjj - Implementation Plan

## Overview

Create a VSCode extension that provides a UI for jjj (Jujutsu Project Manager) with task management, board views, and code review integration.

## Current State

- 📁 Directory exists: `vscode/`
- ❌ No implementation yet (placeholder only)
- ✅ jjj CLI provides JSON output for all commands
- ✅ All data models support serialization

## Architecture

```
vscode-jjj/
├── package.json          # Extension manifest
├── tsconfig.json         # TypeScript configuration
├── src/
│   ├── extension.ts      # Main entry point
│   ├── jjjClient.ts      # CLI wrapper
│   ├── types.ts          # TypeScript types matching Rust models
│   ├── views/
│   │   ├── taskTreeView.ts       # Sidebar: Tasks
│   │   ├── featureTreeView.ts    # Sidebar: Features
│   │   ├── milestoneTreeView.ts  # Sidebar: Milestones
│   │   ├── bugTreeView.ts        # Sidebar: Bugs
│   │   └── reviewTreeView.ts     # Sidebar: Reviews
│   ├── panels/
│   │   ├── boardPanel.ts         # Webview: Kanban board
│   │   ├── roadmapPanel.ts       # Webview: Milestone roadmap
│   │   └── triagePanel.ts        # Webview: Bug triage
│   ├── commands/
│   │   ├── task.ts              # Task commands
│   │   ├── feature.ts           # Feature commands
│   │   ├── milestone.ts         # Milestone commands
│   │   └── bug.ts               # Bug commands
│   └── utils/
│       ├── statusBar.ts         # Status bar integration
│       └── quickPick.ts         # Quick actions
└── media/
    ├── styles.css               # Webview styles
    └── icons/                   # Custom icons

```

## Features by Priority

### Phase 1: MVP (Week 1-2)

#### 1.1 Basic Infrastructure
- [x] Extension scaffolding
- [ ] jjj CLI wrapper (execute commands, parse JSON)
- [ ] Type definitions for all jjj models
- [ ] Error handling and logging
- [ ] Configuration (jjj binary path, auto-refresh, etc.)

#### 1.2 Task Tree View
- [ ] Sidebar panel showing tasks
- [ ] Group by: Column, Feature, Tag, Assignee
- [ ] Click to view task details
- [ ] Context menu: Move, Edit, Delete
- [ ] Icons for task status
- [ ] Refresh button

#### 1.3 Basic Commands
- [ ] `jjj: Create Task` - Quick pick for feature, enter title
- [ ] `jjj: Move Task` - Select task, select column
- [ ] `jjj: Attach Current Change` - Attach @ to selected task
- [ ] `jjj: Show Board` - Open board webview

#### 1.4 Status Bar
- [ ] Show current task (if change is attached)
- [ ] Click to change task status
- [ ] Indicator for pending reviews

### Phase 2: Board & Hierarchy (Week 3-4)

#### 2.1 Kanban Board Webview
- [ ] Column-based layout
- [ ] Drag-and-drop to move tasks
- [ ] Filter by feature, tag, assignee
- [ ] Search tasks
- [ ] Click task to see details panel
- [ ] Inline task creation

#### 2.2 Feature Tree View
- [ ] Sidebar panel showing features
- [ ] Group by: Milestone, Status, Priority
- [ ] Expandable to show tasks
- [ ] Progress bars
- [ ] Context menu: New Task, Move, etc.

#### 2.3 Milestone View
- [ ] Sidebar panel showing milestones
- [ ] Expandable to show features
- [ ] Target date indicators (overdue warnings)
- [ ] Progress tracking
- [ ] Roadmap webview

### Phase 3: Code Review Integration (Week 5-6)

#### 3.1 Review Tree View
- [ ] Show pending reviews
- [ ] Show reviews you requested
- [ ] Show reviews needing your approval
- [ ] Click to view diff
- [ ] Inline comments in editor

#### 3.2 Code Review Features
- [ ] Gutter decorations for comments
- [ ] Inline comment widgets
- [ ] Comment thread view
- [ ] Approve/Request Changes actions
- [ ] Review status in status bar

#### 3.3 Review Webview Panel
- [ ] Full review interface
- [ ] File tree with changes
- [ ] Side-by-side diff
- [ ] Comment composition
- [ ] Review history

### Phase 4: Advanced Features (Week 7-8)

#### 4.1 Bug Triage
- [ ] Bug list view (grouped by severity)
- [ ] Triage webview panel
- [ ] Quick actions: Link, Assign, Update Status
- [ ] Filter by status, severity, milestone

#### 4.2 Quick Actions
- [ ] Command palette integration
- [ ] Quick picks for common operations
- [ ] Keyboard shortcuts
- [ ] Context menus in explorer

#### 4.3 Enhanced Webviews
- [ ] Feature board (tasks grouped by feature)
- [ ] Milestone roadmap (timeline view)
- [ ] Dashboard (overview of everything)
- [ ] Analytics (velocity, cycle time)

#### 4.4 Editor Integration
- [ ] Codelens showing related tasks
- [ ] Hover tooltips for task/feature IDs
- [ ] Auto-link task IDs in commit messages
- [ ] Inline task creation from TODO comments

## Technical Details

### jjj Client Wrapper

```typescript
export class JjjClient {
  constructor(private jjjPath: string, private cwd: string) {}

  async listTasks(options?: { column?: string; tag?: string }): Promise<Task[]> {
    const args = ['task', 'list', '--json'];
    if (options?.column) args.push('--column', options.column);
    if (options?.tag) args.push('--tag', options.tag);
    
    const output = await this.execute(args);
    return JSON.parse(output);
  }

  async createTask(title: string, featureId: string, tags?: string[]): Promise<Task> {
    const args = ['task', 'new', title, '--feature', featureId];
    tags?.forEach(tag => args.push('--tag', tag));
    
    await this.execute(args);
    // Parse created task from output or re-query
  }

  private async execute(args: string[]): Promise<string> {
    return new Promise((resolve, reject) => {
      exec(`${this.jjjPath} ${args.join(' ')}`, { cwd: this.cwd }, (error, stdout, stderr) => {
        if (error) reject(new Error(stderr));
        else resolve(stdout);
      });
    });
  }
}
```

### Type Definitions

```typescript
export interface Task {
  id: string;
  title: string;
  feature_id: string;
  column: string;
  tags: string[];
  change_ids: string[];
  assignee?: string;
  created_at: string;
  updated_at: string;
}

export interface Feature {
  id: string;
  title: string;
  milestone_id?: string;
  status: FeatureStatus;
  priority: Priority;
  task_ids: string[];
  bug_ids: string[];
  assignee?: string;
  story_points?: number;
  created_at: string;
  updated_at: string;
}

// ... etc for Milestone, Bug, Review
```

### Tree View Implementation

```typescript
export class TaskTreeDataProvider implements vscode.TreeDataProvider<TaskItem> {
  private _onDidChangeTreeData = new vscode.EventEmitter<TaskItem | undefined>();
  readonly onDidChangeTreeData = this._onDidChangeTreeData.event;

  constructor(private client: JjjClient) {}

  refresh(): void {
    this._onDidChangeTreeData.fire(undefined);
  }

  async getChildren(element?: TaskItem): Promise<TaskItem[]> {
    if (!element) {
      // Root level: return columns or features
      const tasks = await this.client.listTasks();
      return this.groupTasksByColumn(tasks);
    }
    return element.children || [];
  }

  getTreeItem(element: TaskItem): vscode.TreeItem {
    return element;
  }
}
```

### Webview Panel (Board)

```typescript
export class BoardPanel {
  public static currentPanel: BoardPanel | undefined;
  private readonly _panel: vscode.WebviewPanel;

  public static createOrShow(client: JjjClient) {
    // Create or reveal panel
    const panel = vscode.window.createWebviewPanel(
      'jjjBoard',
      'jjj Kanban Board',
      vscode.ViewColumn.One,
      { enableScripts: true }
    );

    BoardPanel.currentPanel = new BoardPanel(panel, client);
  }

  private async update() {
    const data = await this.client.getBoard();
    this._panel.webview.html = this.getHtmlForWebview(data);
  }

  private getHtmlForWebview(data: any): string {
    return `<!DOCTYPE html>
    <html>
      <head>
        <style>${this.getCss()}</style>
      </head>
      <body>
        <div class="board">${this.renderColumns(data)}</div>
        <script>${this.getScript()}</script>
      </body>
    </html>`;
  }
}
```

## Development Workflow

### Setup
```bash
cd vscode
npm install
npm run compile
```

### Testing
```bash
# Open in Extension Development Host
code --extensionDevelopmentPath=./vscode

# Run tests
npm test
```

### Packaging
```bash
vsce package
# Creates vscode-jjj-0.1.0.vsix
```

## Dependencies

```json
{
  "devDependencies": {
    "@types/node": "^20.0.0",
    "@types/vscode": "^1.80.0",
    "typescript": "^5.0.0",
    "vsce": "^2.15.0"
  }
}
```

## Configuration

```json
{
  "jjj.jjjPath": {
    "type": "string",
    "default": "jjj",
    "description": "Path to jjj executable"
  },
  "jjj.autoRefresh": {
    "type": "boolean",
    "default": true,
    "description": "Auto-refresh views on file changes"
  },
  "jjj.defaultColumn": {
    "type": "string",
    "default": "TODO",
    "description": "Default column for new tasks"
  }
}
```

## Testing Strategy

1. **Unit Tests**: Test jjj client wrapper, parsers
2. **Integration Tests**: Test with real jjj commands
3. **E2E Tests**: Test UI interactions
4. **Manual Testing**: Use in real project

## Release Plan

- **v0.1.0** (MVP): Basic task management + board
- **v0.2.0**: Hierarchy (features, milestones)
- **v0.3.0**: Code review integration
- **v0.4.0**: Bug triage + advanced features
- **v1.0.0**: Production-ready, published to marketplace

## Success Metrics

- Extension load time < 500ms
- Command execution < 200ms
- Webview rendering < 100ms
- Memory usage < 50MB
- 90%+ user satisfaction

