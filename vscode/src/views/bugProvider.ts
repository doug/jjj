import * as vscode from 'vscode';
import { JJJ, Bug } from '../jjj';

export class BugProvider implements vscode.TreeDataProvider<BugItem> {
  private _onDidChangeTreeData: vscode.EventEmitter<BugItem | undefined | null | void> =
    new vscode.EventEmitter<BugItem | undefined | null | void>();
  readonly onDidChangeTreeData: vscode.Event<BugItem | undefined | null | void> =
    this._onDidChangeTreeData.event;

  constructor(private jjj: JJJ) {}

  refresh(): void {
    this._onDidChangeTreeData.fire();
  }

  getTreeItem(element: BugItem): vscode.TreeItem {
    return element;
  }

  async getChildren(element?: BugItem): Promise<BugItem[]> {
    if (element) {
      return [];
    } else {
      try {
        const bugs = await this.jjj.listBugs({ open: true });
        return bugs.map((bug) => new BugItem(bug));
      } catch (error) {
        vscode.window.showErrorMessage(`Failed to load bugs: ${error}`);
        return [];
      }
    }
  }
}

export class BugItem extends vscode.TreeItem {
  constructor(public readonly bug: Bug) {
    super(bug.title, vscode.TreeItemCollapsibleState.None);
    this.tooltip = this._getTooltip();
    this.description = this._getDescription();

    this.command = {
      command: 'jjj.openBug',
      title: 'Open Bug',
      arguments: [bug],
    };

    this.iconPath = this._getIcon();
    this.contextValue = 'bug';
  }

  private _getTooltip(): string {
    let tooltip = `${this.bug.id}: ${this.bug.title}`;

    if (this.bug.severity) {
      tooltip += `\nSeverity: ${this.bug.severity}`;
    }

    if (this.bug.status) {
      tooltip += `\nStatus: ${this.bug.status}`;
    }

    if (this.bug.reporter) {
      tooltip += `\nReporter: ${this.bug.reporter}`;
    }

    if (this.bug.assignee) {
      tooltip += `\nAssignee: ${this.bug.assignee}`;
    }

    if (this.bug.feature_id) {
      tooltip += `\nFeature: ${this.bug.feature_id}`;
    }

    if (this.bug.repro_steps) {
      tooltip += `\n\nRepro Steps:\n${this.bug.repro_steps}`;
    }

    return tooltip;
  }

  private _getDescription(): string {
    const parts = [this.bug.id];

    parts.push(this._getSeverityEmoji());
    parts.push(this.bug.status);

    if (this.bug.assignee) {
      parts.push(`@${this.bug.assignee}`);
    }

    if (this.bug.feature_id) {
      parts.push(`F: ${this.bug.feature_id}`);
    }

    return parts.join(' • ');
  }

  private _getSeverityEmoji(): string {
    switch (this.bug.severity) {
      case 'critical':
        return '🔴 Critical';
      case 'high':
        return '🟠 High';
      case 'medium':
        return '🟡 Medium';
      case 'low':
        return '🟢 Low';
      default:
        return this.bug.severity || '';
    }
  }

  private _getIcon(): vscode.ThemeIcon {
    switch (this.bug.status) {
      case 'fixed':
        return new vscode.ThemeIcon('check', new vscode.ThemeColor('testing.iconPassed'));
      case 'closed':
        return new vscode.ThemeIcon('circle-slash', new vscode.ThemeColor('testing.iconSkipped'));
      case 'inprogress':
        return new vscode.ThemeIcon('tools', new vscode.ThemeColor('charts.blue'));
      case 'confirmed':
        return new vscode.ThemeIcon('bug', new vscode.ThemeColor('charts.orange'));
      case 'new':
        return new vscode.ThemeIcon('circle-filled', new vscode.ThemeColor('testing.iconErrored'));
      case 'wontfix':
      case 'duplicate':
        return new vscode.ThemeIcon('circle-outline', new vscode.ThemeColor('testing.iconSkipped'));
      default:
        return new vscode.ThemeIcon('bug');
    }
  }
}
