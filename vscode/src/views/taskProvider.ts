import * as vscode from 'vscode';
import { JJJ, Task } from '../jjj';

export class TaskProvider implements vscode.TreeDataProvider<TaskItem | TaskGroup> {
  private _onDidChangeTreeData: vscode.EventEmitter<
    TaskItem | TaskGroup | undefined | null | void
  > = new vscode.EventEmitter<TaskItem | TaskGroup | undefined | null | void>();
  readonly onDidChangeTreeData: vscode.Event<TaskItem | TaskGroup | undefined | null | void> =
    this._onDidChangeTreeData.event;

  private groupByColumn = true;

  constructor(private jjj: JJJ) {}

  refresh(): void {
    this._onDidChangeTreeData.fire();
  }

  toggleGrouping(): void {
    this.groupByColumn = !this.groupByColumn;
    this.refresh();
  }

  getTreeItem(element: TaskItem | TaskGroup): vscode.TreeItem {
    return element;
  }

  async getChildren(element?: TaskItem | TaskGroup): Promise<(TaskItem | TaskGroup)[]> {
    if (element instanceof TaskGroup) {
      // Return tasks in this group
      return element.tasks.map((task) => new TaskItem(task));
    } else if (element instanceof TaskItem) {
      // Tasks have no children
      return [];
    } else {
      // Root level
      try {
        const tasks = await this.jjj.listTasks();

        if (this.groupByColumn) {
          // Group tasks by column
          const columns = new Map<string, Task[]>();
          for (const task of tasks) {
            const column = task.column || 'Uncategorized';
            if (!columns.has(column)) {
              columns.set(column, []);
            }
            columns.get(column)!.push(task);
          }

          // Convert to TaskGroups
          return Array.from(columns.entries()).map(
            ([column, tasks]) => new TaskGroup(column, tasks),
          );
        } else {
          // Flat list
          return tasks.map((task) => new TaskItem(task));
        }
      } catch (error) {
        vscode.window.showErrorMessage(`Failed to load tasks: ${error}`);
        return [];
      }
    }
  }
}

export class TaskGroup extends vscode.TreeItem {
  constructor(
    public readonly column: string,
    public readonly tasks: Task[],
  ) {
    super(column, vscode.TreeItemCollapsibleState.Expanded);
    this.description = `${tasks.length} tasks`;
    this.iconPath = new vscode.ThemeIcon('folder');
    this.contextValue = 'taskGroup';
  }
}

export class TaskItem extends vscode.TreeItem {
  constructor(public readonly task: Task) {
    super(task.title, vscode.TreeItemCollapsibleState.None);
    this.tooltip = `${task.id}: ${task.title}`;
    this.description = `${task.id} [${task.column}]`;

    this.command = {
      command: 'jjj.openTask',
      title: 'Open Task',
      arguments: [task],
    };

    this.iconPath = new vscode.ThemeIcon('checklist');
  }
}
