import * as vscode from 'vscode';
import { JJJ, Milestone } from '../jjj';

export class MilestoneProvider implements vscode.TreeDataProvider<MilestoneItem> {
  private _onDidChangeTreeData: vscode.EventEmitter<MilestoneItem | undefined | null | void> =
    new vscode.EventEmitter<MilestoneItem | undefined | null | void>();
  readonly onDidChangeTreeData: vscode.Event<MilestoneItem | undefined | null | void> =
    this._onDidChangeTreeData.event;

  constructor(private jjj: JJJ) {}

  refresh(): void {
    this._onDidChangeTreeData.fire();
  }

  getTreeItem(element: MilestoneItem): vscode.TreeItem {
    return element;
  }

  async getChildren(element?: MilestoneItem): Promise<MilestoneItem[]> {
    if (element) {
      return [];
    } else {
      try {
        const milestones = await this.jjj.listMilestones();
        return milestones.map((milestone) => new MilestoneItem(milestone));
      } catch (error) {
        vscode.window.showErrorMessage(`Failed to load milestones: ${error}`);
        return [];
      }
    }
  }
}

export class MilestoneItem extends vscode.TreeItem {
  constructor(public readonly milestone: Milestone) {
    super(milestone.title, vscode.TreeItemCollapsibleState.None);
    this.tooltip = this._getTooltip();
    this.description = this._getDescription();

    this.command = {
      command: 'jjj.openMilestone',
      title: 'Open Milestone',
      arguments: [milestone],
    };

    this.iconPath = this._getIcon();
    this.contextValue = 'milestone';
  }

  private _getTooltip(): string {
    let tooltip = `${this.milestone.id}: ${this.milestone.title}`;

    if (this.milestone.description) {
      tooltip += `\n${this.milestone.description}`;
    }

    if (this.milestone.target_date) {
      tooltip += `\nTarget: ${this.milestone.target_date}`;
    }

    const featureCount = this.milestone.feature_ids?.length || 0;
    const bugCount = this.milestone.bug_ids?.length || 0;
    tooltip += `\n${featureCount} features, ${bugCount} bugs`;

    return tooltip;
  }

  private _getDescription(): string {
    const parts = [this.milestone.id];

    parts.push(this.milestone.status);

    if (this.milestone.target_date) {
      const targetDate = new Date(this.milestone.target_date);
      const now = new Date();
      const daysUntil = Math.ceil((targetDate.getTime() - now.getTime()) / (1000 * 60 * 60 * 24));

      if (daysUntil < 0) {
        parts.push(`⚠️ ${Math.abs(daysUntil)}d overdue`);
      } else if (daysUntil < 7) {
        parts.push(`⏰ ${daysUntil}d`);
      } else {
        parts.push(`📅 ${this.milestone.target_date}`);
      }
    }

    const featureCount = this.milestone.feature_ids?.length || 0;
    if (featureCount > 0) {
      parts.push(`${featureCount}F`);
    }

    return parts.join(' • ');
  }

  private _getIcon(): vscode.ThemeIcon {
    switch (this.milestone.status) {
      case 'Released':
        return new vscode.ThemeIcon('package', new vscode.ThemeColor('testing.iconPassed'));
      case 'Active':
        return new vscode.ThemeIcon('rocket', new vscode.ThemeColor('charts.blue'));
      case 'Planning':
        return new vscode.ThemeIcon('calendar', new vscode.ThemeColor('charts.yellow'));
      case 'Cancelled':
        return new vscode.ThemeIcon('circle-slash', new vscode.ThemeColor('testing.iconFailed'));
      default:
        return new vscode.ThemeIcon('milestone');
    }
  }
}
