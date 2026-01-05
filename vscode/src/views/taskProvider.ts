import * as vscode from 'vscode';
import { JJJ, Task } from '../jjj';

export class TaskProvider implements vscode.TreeDataProvider<TaskItem> {
    private _onDidChangeTreeData: vscode.EventEmitter<TaskItem | undefined | null | void> = new vscode.EventEmitter<TaskItem | undefined | null | void>();
    readonly onDidChangeTreeData: vscode.Event<TaskItem | undefined | null | void> = this._onDidChangeTreeData.event;

    constructor(private jjj: JJJ) { }

    refresh(): void {
        this._onDidChangeTreeData.fire();
    }

    getTreeItem(element: TaskItem): vscode.TreeItem {
        return element;
    }

    async getChildren(element?: TaskItem): Promise<TaskItem[]> {
        if (element) {
            return [];
        } else {
            try {
                const tasks = await this.jjj.listTasks();
                return tasks.map(task => new TaskItem(task));
            } catch (error) {
                vscode.window.showErrorMessage(`Failed to load tasks: ${error}`);
                return [];
            }
        }
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
            arguments: [task]
        };

        this.iconPath = new vscode.ThemeIcon('checklist');
    }
}
