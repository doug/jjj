import * as vscode from "vscode";
import { DataCache } from "./cache";

export class SolutionStatusBar implements vscode.Disposable {
  private item: vscode.StatusBarItem;
  private disposables: vscode.Disposable[] = [];

  constructor(cache: DataCache) {
    this.item = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Left, 100);
    this.item.command = "jjj.openActiveSolution";
    this.disposables.push(this.item);
    this.disposables.push(cache.onDidChange(() => this.update(cache)));
    this.update(cache);
  }

  private update(cache: DataCache): void {
    const status = cache.getStatus();
    if (!status?.active_solution) {
      this.item.hide();
      return;
    }
    const sol = status.active_solution;
    const icon = sol.status === "review" ? "$(beaker)" : "$(lightbulb)";
    const critiques = cache.getCritiques().filter(
      c => c.solution_id === sol.id && (c.status === "open" || c.status === "valid"),
    );
    const badge = critiques.length > 0 ? ` $(warning)${critiques.length}` : "";
    this.item.text = `${icon} ${sol.title}${badge}`;
    this.item.tooltip = `Active solution: ${sol.title} [${sol.status}]`;
    this.item.show();
  }

  dispose(): void {
    this.disposables.forEach(d => d.dispose());
  }
}
