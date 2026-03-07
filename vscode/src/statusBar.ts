import * as vscode from "vscode";
import { DataCache } from "./cache";

export class NextActionStatusBar implements vscode.Disposable {
  private item: vscode.StatusBarItem;
  private disposables: vscode.Disposable[] = [];

  constructor(cache: DataCache) {
    this.item = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Right, 50);
    this.item.command = "jjj.nextOpenItem";
    this.disposables.push(this.item);
    this.disposables.push(cache.onDidChange(() => this.update(cache)));
    this.update(cache);
  }

  private update(cache: DataCache): void {
    const summary = cache.getStatus()?.summary;
    if (!summary) {
      this.item.hide();
      return;
    }

    const { open_problems, review_solutions, open_critiques } = summary;
    const segments: string[] = [];
    const tooltipParts: string[] = [];

    if (open_problems > 0) {
      segments.push(`$(issues) ${open_problems}`);
      tooltipParts.push(`${open_problems} open problem${open_problems === 1 ? "" : "s"}`);
    }
    if (review_solutions > 0) {
      segments.push(`$(beaker) ${review_solutions}`);
      tooltipParts.push(`${review_solutions} solution${review_solutions === 1 ? "" : "s"} in review`);
    }
    if (open_critiques > 0) {
      segments.push(`$(warning) ${open_critiques}`);
      tooltipParts.push(`${open_critiques} open critique${open_critiques === 1 ? "" : "s"}`);
    }

    if (segments.length === 0) {
      this.item.hide();
      return;
    }

    this.item.text = segments.join("  ");
    this.item.tooltip = tooltipParts.join(" · ");
    this.item.show();
  }

  dispose(): void {
    this.disposables.forEach(d => d.dispose());
  }
}

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
    const icon = sol.status === "submitted" ? "$(beaker)" : "$(lightbulb)";
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
