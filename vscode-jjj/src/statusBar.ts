import * as vscode from "vscode";
import { DataCache } from "./cache";

export class StatusBar {
  private item: vscode.StatusBarItem;
  private cacheSubscription: vscode.Disposable;

  constructor(private cache: DataCache) {
    this.item = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Left, 50);
    this.item.command = "jjj-next-actions.focus";
    this.cacheSubscription = cache.onDidChange(() => this.update());
    this.item.show();
  }

  private update() {
    const next = this.cache.getNext();
    if (!next || next.items.length === 0) {
      this.item.text = "$(beaker) JJJ: all clear";
      this.item.color = undefined;
      this.item.backgroundColor = undefined;
      return;
    }

    const blocked = next.items.filter(i => i.category === "blocked").length;
    const ready = next.items.filter(i => i.category === "ready").length;
    const review = next.items.filter(i => i.category === "review").length;

    const parts: string[] = [];
    if (blocked > 0) { parts.push(`${blocked} blocked`); }
    if (ready > 0) { parts.push(`${ready} ready`); }
    if (review > 0) { parts.push(`${review} to review`); }

    this.item.text = `$(beaker) JJJ: ${parts.join(", ") || "no actions"}`;

    if (blocked > 0) {
      this.item.backgroundColor = new vscode.ThemeColor("statusBarItem.errorBackground");
    } else if (review > 0) {
      this.item.backgroundColor = new vscode.ThemeColor("statusBarItem.warningBackground");
    } else {
      this.item.backgroundColor = undefined;
    }
  }

  dispose() {
    this.cacheSubscription.dispose();
    this.item.dispose();
  }
}
