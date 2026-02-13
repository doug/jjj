import * as vscode from "vscode";
import { DataCache } from "../cache";
import { JjjCli, Problem, Solution, Critique, Milestone } from "../cli";

type TreeNode = MilestoneNode | ProblemNode | SolutionNode | CritiqueNode;

function isOpenProblem(p: Problem): boolean {
  return p.status !== "solved" && p.status !== "dissolved";
}

function isOpenSolution(s: Solution): boolean {
  return s.status !== "accepted" && s.status !== "refuted";
}

function isOpenCritique(c: Critique): boolean {
  return c.status === "open";
}

class MilestoneNode extends vscode.TreeItem {
  constructor(public readonly milestone: Milestone | null, problemCount: number, solvedCount: number) {
    const label = milestone ? milestone.title : "Backlog";
    super(label, vscode.TreeItemCollapsibleState.Expanded);
    this.contextValue = milestone ? "milestone" : "backlog";
    this.description = milestone
      ? `(${solvedCount}/${problemCount} solved)`
      : `(${problemCount} problems)`;
    this.iconPath = milestone
      ? new vscode.ThemeIcon("milestone")
      : new vscode.ThemeIcon("inbox");
  }
}

class ProblemNode extends vscode.TreeItem {
  constructor(public readonly problem: Problem) {
    super(problem.title, vscode.TreeItemCollapsibleState.Collapsed);
    this.contextValue = "problem";
    this.description = `${problem.id} [${problem.status}]${problem.priority !== "medium" ? ` ${problem.priority}` : ""}`;
    this.iconPath = problem.status === "solved"
      ? new vscode.ThemeIcon("check", new vscode.ThemeColor("testing.iconPassed"))
      : problem.status === "in_progress"
        ? new vscode.ThemeIcon("debug-start", new vscode.ThemeColor("editorInfo.foreground"))
        : new vscode.ThemeIcon("circle-outline");
    this.command = {
      command: "jjj.openEntity",
      title: "Open",
      arguments: ["problem", problem.id],
    };
  }
}

class SolutionNode extends vscode.TreeItem {
  constructor(public readonly solution: Solution, critiqueCount: number) {
    super(solution.title, critiqueCount > 0
      ? vscode.TreeItemCollapsibleState.Collapsed
      : vscode.TreeItemCollapsibleState.None);
    this.contextValue = "solution";
    const critDesc = critiqueCount > 0 ? ` — ${critiqueCount} critiques` : "";
    this.description = `${solution.id} [${solution.status}]${critDesc}`;
    this.iconPath = solution.status === "accepted"
      ? new vscode.ThemeIcon("check", new vscode.ThemeColor("testing.iconPassed"))
      : solution.status === "refuted"
        ? new vscode.ThemeIcon("x", new vscode.ThemeColor("errorForeground"))
        : solution.status === "testing"
          ? new vscode.ThemeIcon("beaker")
          : new vscode.ThemeIcon("lightbulb");
    this.command = {
      command: "jjj.openEntity",
      title: "Open",
      arguments: ["solution", solution.id],
    };
  }
}

class CritiqueNode extends vscode.TreeItem {
  constructor(public readonly critique: Critique) {
    super(critique.title, vscode.TreeItemCollapsibleState.None);
    this.contextValue = "critique";
    const location = critique.file_path
      ? ` — ${critique.file_path}:${critique.line_start}`
      : "";
    this.description = `${critique.id} [${critique.severity}]${location}`;
    this.iconPath = critique.status === "open"
      ? (critique.severity === "high" || critique.severity === "critical"
        ? new vscode.ThemeIcon("warning", new vscode.ThemeColor("errorForeground"))
        : new vscode.ThemeIcon("warning", new vscode.ThemeColor("editorWarning.foreground")))
      : new vscode.ThemeIcon("pass", new vscode.ThemeColor("testing.iconPassed"));
    this.command = {
      command: "jjj.openEntity",
      title: "Open",
      arguments: ["critique", critique.id],
    };
  }
}

const MIME_TYPE = "application/vnd.jjj.problem";

export class ProjectTreeProvider implements vscode.TreeDataProvider<TreeNode>, vscode.TreeDragAndDropController<TreeNode>, vscode.Disposable {
  dropMimeTypes = [MIME_TYPE];
  dragMimeTypes = [MIME_TYPE];

  private _onDidChangeTreeData = new vscode.EventEmitter<TreeNode | undefined>();
  readonly onDidChangeTreeData = this._onDidChangeTreeData.event;
  private cacheSubscription: vscode.Disposable;

  private _filterMode: "all" | "open" = "open";

  get filterMode(): "all" | "open" {
    return this._filterMode;
  }

  toggleFilter(): void {
    this._filterMode = this._filterMode === "all" ? "open" : "all";
    this._onDidChangeTreeData.fire(undefined);
  }

  constructor(private cache: DataCache, private cli: JjjCli) {
    this.cacheSubscription = cache.onDidChange(() => this._onDidChangeTreeData.fire(undefined));
  }

  dispose(): void {
    this._onDidChangeTreeData.dispose();
    this.cacheSubscription.dispose();
  }

  getTreeItem(element: TreeNode): vscode.TreeItem {
    return element;
  }

  getChildren(element?: TreeNode): TreeNode[] {
    const filterOpen = this._filterMode === "open";

    if (!element) {
      // Root: milestones + backlog
      const milestones = this.cache.getMilestones();
      const nodes: TreeNode[] = [];

      for (const m of milestones) {
        const problems = this.cache.getProblemsForMilestone(m.id);
        const openProblems = problems.filter(isOpenProblem);
        const solved = problems.filter(p => p.status === "solved").length;

        // In open mode, skip milestones with no open problems
        if (filterOpen && openProblems.length === 0) {
          continue;
        }

        nodes.push(new MilestoneNode(m, problems.length, solved));
      }

      const backlog = this.cache.getBacklogProblems();
      // Always show backlog (even if empty in open mode)
      nodes.push(new MilestoneNode(null, backlog.length, 0));

      return nodes;
    }

    if (element instanceof MilestoneNode) {
      let problems = element.milestone
        ? this.cache.getProblemsForMilestone(element.milestone.id)
        : this.cache.getBacklogProblems();

      if (filterOpen) {
        problems = problems.filter(isOpenProblem);
      }

      return problems.map(p => new ProblemNode(p));
    }

    if (element instanceof ProblemNode) {
      let solutions = this.cache.getSolutionsForProblem(element.problem.id);

      if (filterOpen) {
        solutions = solutions.filter(isOpenSolution);
      }

      return solutions.map(s => {
        const critiques = this.cache.getCritiquesForSolution(s.id);
        return new SolutionNode(s, critiques.filter(c => c.status === "open").length);
      });
    }

    if (element instanceof SolutionNode) {
      let critiques = this.cache.getCritiquesForSolution(element.solution.id);

      if (filterOpen) {
        critiques = critiques.filter(isOpenCritique);
      }

      return critiques.map(c => new CritiqueNode(c));
    }

    return [];
  }

  // --- Tab Navigation ---

  /**
   * Get all open (actionable) items in depth-first order.
   * Milestones are skipped as they're containers, not actionable.
   */
  getAllOpenItems(): TreeNode[] {
    const items: TreeNode[] = [];
    const milestones = this.cache.getMilestones();

    for (const m of milestones) {
      const problems = this.cache.getProblemsForMilestone(m.id);
      for (const p of problems) {
        if (isOpenProblem(p)) {
          items.push(new ProblemNode(p));
        }
        const solutions = this.cache.getSolutionsForProblem(p.id);
        for (const s of solutions) {
          const critiques = this.cache.getCritiquesForSolution(s.id);
          if (isOpenSolution(s)) {
            items.push(new SolutionNode(s, critiques.filter(c => c.status === "open").length));
          }
          for (const c of critiques) {
            if (isOpenCritique(c)) {
              items.push(new CritiqueNode(c));
            }
          }
        }
      }
    }

    // Backlog
    const backlog = this.cache.getBacklogProblems();
    for (const p of backlog) {
      if (isOpenProblem(p)) {
        items.push(new ProblemNode(p));
      }
      const solutions = this.cache.getSolutionsForProblem(p.id);
      for (const s of solutions) {
        const critiques = this.cache.getCritiquesForSolution(s.id);
        if (isOpenSolution(s)) {
          items.push(new SolutionNode(s, critiques.filter(c => c.status === "open").length));
        }
        for (const c of critiques) {
          if (isOpenCritique(c)) {
            items.push(new CritiqueNode(c));
          }
        }
      }
    }

    return items;
  }

  private getItemId(node: TreeNode | undefined): string | undefined {
    if (!node) return undefined;
    if (node instanceof ProblemNode) return `p:${node.problem.id}`;
    if (node instanceof SolutionNode) return `s:${node.solution.id}`;
    if (node instanceof CritiqueNode) return `c:${node.critique.id}`;
    return undefined;
  }

  getNextOpenItem(current: TreeNode | undefined): TreeNode | undefined {
    const items = this.getAllOpenItems();
    if (items.length === 0) return undefined;

    if (!current) return items[0];

    const currentId = this.getItemId(current);
    const currentIndex = items.findIndex(item => this.getItemId(item) === currentId);

    if (currentIndex === -1) return items[0];

    const nextIndex = (currentIndex + 1) % items.length;
    return items[nextIndex];
  }

  getPrevOpenItem(current: TreeNode | undefined): TreeNode | undefined {
    const items = this.getAllOpenItems();
    if (items.length === 0) return undefined;

    if (!current) return items[items.length - 1];

    const currentId = this.getItemId(current);
    const currentIndex = items.findIndex(item => this.getItemId(item) === currentId);

    if (currentIndex === -1) return items[items.length - 1];

    const prevIndex = (currentIndex - 1 + items.length) % items.length;
    return items[prevIndex];
  }

  // --- Drag and Drop ---

  handleDrag(source: readonly TreeNode[], dataTransfer: vscode.DataTransfer): void {
    const problems = source.filter((n): n is ProblemNode => n instanceof ProblemNode);
    if (problems.length > 0) {
      dataTransfer.set(MIME_TYPE, new vscode.DataTransferItem(
        problems.map(p => p.problem.id),
      ));
    }
  }

  async handleDrop(target: TreeNode | undefined, dataTransfer: vscode.DataTransfer): Promise<void> {
    const item = dataTransfer.get(MIME_TYPE);
    if (!item) { return; }

    // Only accept drops onto milestones (including Backlog)
    if (!(target instanceof MilestoneNode)) { return; }

    const problemIds: string[] = item.value;
    const targetMilestoneId = target.milestone?.id ?? null;

    for (const problemId of problemIds) {
      const problem = this.cache.getProblem(problemId);
      if (!problem) { continue; }

      // Skip if already in the target milestone
      if (problem.milestone_id === targetMilestoneId) { continue; }

      // Remove from old milestone
      if (problem.milestone_id) {
        await this.cli.milestoneRemoveProblem(problem.milestone_id, problemId);
      }

      // Add to new milestone (unless dropping on Backlog)
      if (targetMilestoneId) {
        await this.cli.milestoneAddProblem(targetMilestoneId, problemId);
      }
    }

    await this.cache.refresh();
  }
}
