import * as vscode from "vscode";
import { DataCache } from "../cache";
import { NextItem } from "../cli";

const CATEGORY_ICONS: Record<string, vscode.ThemeIcon> = {
  blocked: new vscode.ThemeIcon("error", new vscode.ThemeColor("errorForeground")),
  ready: new vscode.ThemeIcon("check", new vscode.ThemeColor("testing.iconPassed")),
  review: new vscode.ThemeIcon("eye", new vscode.ThemeColor("editorInfo.foreground")),
  waiting: new vscode.ThemeIcon("clock"),
  todo: new vscode.ThemeIcon("circle-outline"),
};

export class NextActionsProvider implements vscode.TreeDataProvider<NextActionItem> {
  private _onDidChangeTreeData = new vscode.EventEmitter<NextActionItem | undefined>();
  readonly onDidChangeTreeData = this._onDidChangeTreeData.event;

  constructor(private cache: DataCache) {
    cache.onDidChange(() => this._onDidChangeTreeData.fire(undefined));
  }

  getTreeItem(element: NextActionItem): vscode.TreeItem {
    return element;
  }

  getChildren(element?: NextActionItem): NextActionItem[] {
    if (element) {
      // Detail children for blocked items
      return element.nextItem.details.map(d => {
        const item = new NextActionItem(
          `${d.id}: ${d.text}`,
          { ...element.nextItem, entity_id: d.id, title: d.text, details: [] },
          vscode.TreeItemCollapsibleState.None,
        );
        if (d.severity) {
          item.description = `[${d.severity}]`;
        }
        item.iconPath = new vscode.ThemeIcon("comment");
        return item;
      });
    }

    const next = this.cache.getNext();
    if (!next || next.items.length === 0) {
      return [new NextActionItem("All caught up!", {} as NextItem, vscode.TreeItemCollapsibleState.None)];
    }

    return next.items.map(ni => {
      const label = `${ni.entity_id}: ${ni.title}`;
      const hasChildren = ni.details.length > 0;
      const state = hasChildren
        ? vscode.TreeItemCollapsibleState.Expanded
        : vscode.TreeItemCollapsibleState.None;
      const item = new NextActionItem(label, ni, state);
      item.description = ni.summary;
      item.iconPath = CATEGORY_ICONS[ni.category] || new vscode.ThemeIcon("circle-outline");
      item.tooltip = `[${ni.category.toUpperCase()}] ${ni.summary}`;
      item.contextValue = ni.entity_type;
      if (ni.suggested_command) {
        item.command = {
          command: "jjj.openEntity",
          title: "Open",
          arguments: [ni.entity_type, ni.entity_id],
        };
      }
      return item;
    });
  }
}

export class NextActionItem extends vscode.TreeItem {
  constructor(
    label: string,
    public readonly nextItem: NextItem,
    collapsibleState: vscode.TreeItemCollapsibleState,
  ) {
    super(label, collapsibleState);
  }
}
