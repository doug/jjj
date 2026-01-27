import * as vscode from "vscode";
import { DataCache } from "../cache";
import { JjjCli, Problem, Solution, Critique, Milestone } from "../cli";

type TreeNode = MilestoneNode | ProblemNode | SolutionNode | CritiqueNode;

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
    const reviewStatus = solution.reviewers.length > 0
      ? (() => {
        const signedOff = solution.sign_offs.filter(so => solution.reviewers.includes(so.reviewer)).length;
        return signedOff === solution.reviewers.length
          ? ""
          : ` ${signedOff}/${solution.reviewers.length} reviewed`;
      })()
      : "";
    this.description = `${solution.id} [${solution.status}]${critDesc}${reviewStatus}`;
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
    if (!element) {
      // Root: milestones + backlog
      const milestones = this.cache.getMilestones();
      const nodes: TreeNode[] = milestones.map(m => {
        const problems = this.cache.getProblemsForMilestone(m.id);
        const solved = problems.filter(p => p.status === "solved").length;
        return new MilestoneNode(m, problems.length, solved);
      });
      const backlog = this.cache.getBacklogProblems();
      nodes.push(new MilestoneNode(null, backlog.length, 0));
      return nodes;
    }

    if (element instanceof MilestoneNode) {
      const problems = element.milestone
        ? this.cache.getProblemsForMilestone(element.milestone.id)
        : this.cache.getBacklogProblems();
      return problems.map(p => new ProblemNode(p));
    }

    if (element instanceof ProblemNode) {
      return this.cache.getSolutionsForProblem(element.problem.id).map(s => {
        const critiques = this.cache.getCritiquesForSolution(s.id);
        return new SolutionNode(s, critiques.filter(c => c.status === "open").length);
      });
    }

    if (element instanceof SolutionNode) {
      return this.cache.getCritiquesForSolution(element.solution.id).map(c => new CritiqueNode(c));
    }

    return [];
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
