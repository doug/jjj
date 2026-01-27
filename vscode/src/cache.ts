import * as vscode from "vscode";
import { JjjCli, Problem, Solution, Critique, Milestone, StatusResult } from "./cli";

export class DataCache implements vscode.Disposable {
  private cli: JjjCli;
  private _onDidChange = new vscode.EventEmitter<void>();
  readonly onDidChange = this._onDidChange.event;

  private problems: Problem[] = [];
  private solutions: Solution[] = [];
  private critiques: Critique[] = [];
  private milestones: Milestone[] = [];
  private statusResult: StatusResult | null = null;
  private loading = false;

  constructor(cli: JjjCli) {
    this.cli = cli;
  }

  dispose(): void {
    this._onDidChange.dispose();
  }

  async refresh(): Promise<void> {
    if (this.loading) { return; }
    this.loading = true;
    try {
      const [problems, solutions, critiques, milestones, next] = await Promise.all([
        this.cli.listProblems().catch(() => this.problems),
        this.cli.listSolutions().catch(() => this.solutions),
        this.cli.listCritiques().catch(() => this.critiques),
        this.cli.listMilestones().catch(() => this.milestones),
        this.cli.status(true).catch(() => this.statusResult),
      ]);
      this.problems = problems;
      this.solutions = solutions;
      this.critiques = critiques;
      this.milestones = milestones;
      this.statusResult = next;
      this._onDidChange.fire();
    } finally {
      this.loading = false;
    }
  }

  getProblems(): readonly Problem[] { return this.problems; }
  getSolutions(): readonly Solution[] { return this.solutions; }
  getCritiques(): readonly Critique[] { return this.critiques; }
  getMilestones(): readonly Milestone[] { return this.milestones; }
  getStatus(): StatusResult | null { return this.statusResult; }

  getProblemsForMilestone(milestoneId: string): Problem[] {
    return this.problems.filter(p => p.milestone_id === milestoneId);
  }

  getBacklogProblems(): Problem[] {
    return this.problems.filter(p => !p.milestone_id);
  }

  getSolutionsForProblem(problemId: string): Solution[] {
    return this.solutions.filter(s => s.problem_id === problemId);
  }

  getCritiquesForSolution(solutionId: string): Critique[] {
    return this.critiques.filter(c => c.solution_id === solutionId);
  }

  getCritiquesWithLocations(): Critique[] {
    return this.critiques.filter(c => c.file_path && c.line_start);
  }

  getProblem(id: string): Problem | undefined {
    return this.problems.find(p => p.id === id);
  }

  getSolution(id: string): Solution | undefined {
    return this.solutions.find(s => s.id === id);
  }

  getCritique(id: string): Critique | undefined {
    return this.critiques.find(c => c.id === id);
  }

  getMilestone(id: string): Milestone | undefined {
    return this.milestones.find(m => m.id === id);
  }
}
