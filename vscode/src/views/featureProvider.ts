import * as vscode from 'vscode';
import { JJJ, Feature } from '../jjj';

export class FeatureProvider implements vscode.TreeDataProvider<FeatureItem> {
  private _onDidChangeTreeData: vscode.EventEmitter<FeatureItem | undefined | null | void> =
    new vscode.EventEmitter<FeatureItem | undefined | null | void>();
  readonly onDidChangeTreeData: vscode.Event<FeatureItem | undefined | null | void> =
    this._onDidChangeTreeData.event;

  constructor(private jjj: JJJ) {}

  refresh(): void {
    this._onDidChangeTreeData.fire();
  }

  getTreeItem(element: FeatureItem): vscode.TreeItem {
    return element;
  }

  async getChildren(element?: FeatureItem): Promise<FeatureItem[]> {
    if (element) {
      return [];
    } else {
      try {
        const features = await this.jjj.listFeatures();
        return features.map((feature) => new FeatureItem(feature));
      } catch (error) {
        vscode.window.showErrorMessage(`Failed to load features: ${error}`);
        return [];
      }
    }
  }
}

export class FeatureItem extends vscode.TreeItem {
  constructor(public readonly feature: Feature) {
    super(feature.title, vscode.TreeItemCollapsibleState.None);
    this.tooltip = `${feature.id}: ${feature.title}`;
    this.description = this._getDescription();

    this.command = {
      command: 'jjj.openFeature',
      title: 'Open Feature',
      arguments: [feature],
    };

    this.iconPath = this._getIcon();
    this.contextValue = 'feature';
  }

  private _getDescription(): string {
    const parts = [this.feature.id];

    if (this.feature.milestone_id) {
      parts.push(`M: ${this.feature.milestone_id}`);
    }

    parts.push(this.feature.status);

    if (this.feature.priority) {
      parts.push(this._getPriorityEmoji());
    }

    const taskCount = this.feature.task_ids?.length || 0;
    if (taskCount > 0) {
      parts.push(`${taskCount} tasks`);
    }

    return parts.join(' • ');
  }

  private _getPriorityEmoji(): string {
    switch (this.feature.priority) {
      case 'Critical':
        return '🔴';
      case 'High':
        return '🟠';
      case 'Medium':
        return '🟡';
      case 'Low':
        return '🟢';
      default:
        return '';
    }
  }

  private _getIcon(): vscode.ThemeIcon {
    switch (this.feature.status) {
      case 'Done':
        return new vscode.ThemeIcon('check', new vscode.ThemeColor('testing.iconPassed'));
      case 'InProgress':
        return new vscode.ThemeIcon('sync', new vscode.ThemeColor('charts.blue'));
      case 'Review':
        return new vscode.ThemeIcon('eye', new vscode.ThemeColor('charts.purple'));
      case 'Blocked':
        return new vscode.ThemeIcon('error', new vscode.ThemeColor('testing.iconFailed'));
      case 'Backlog':
      default:
        return new vscode.ThemeIcon('circle-outline');
    }
  }
}
