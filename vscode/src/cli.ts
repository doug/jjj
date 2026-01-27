import * as cp from "child_process";
import * as vscode from "vscode";

// --- Data Interfaces ---

export interface Problem {
  id: string;
  title: string;
  parent_id: string | null;
  status: "open" | "in_progress" | "solved" | "dissolved";
  solution_ids: string[];
  child_ids: string[];
  milestone_id: string | null;
  assignee: string | null;
  created_at: string;
  updated_at: string;
  description: string;
  context: string;
  priority: "low" | "medium" | "high" | "critical";
  dissolved_reason: string | null;
}

export interface Solution {
  id: string;
  title: string;
  problem_id: string;
  status: "proposed" | "testing" | "accepted" | "refuted";
  critique_ids: string[];
  change_ids: string[];
  assignee: string | null;
  reviewers: string[];
  sign_offs: Array<{ reviewer: string; at: string; comment?: string }>;
  force_accepted: boolean;
  created_at: string;
  updated_at: string;
  approach: string;
  tradeoffs: string;
  supersedes: string | null;
}

export interface Reply {
  id: string;
  author: string;
  body: string;
  created_at: string;
}

export interface Critique {
  id: string;
  title: string;
  solution_id: string;
  status: "open" | "addressed" | "valid" | "dismissed";
  severity: "low" | "medium" | "high" | "critical";
  author: string | null;
  created_at: string;
  updated_at: string;
  argument: string;
  evidence: string;
  file_path: string | null;
  line_start: number | null;
  line_end: number | null;
  code_context: string[];
  replies: Reply[];
}

export interface Milestone {
  id: string;
  title: string;
  target_date: string | null;
  status: "planning" | "active" | "completed" | "cancelled";
  problem_ids: string[];
  assignee: string | null;
  created_at: string;
  updated_at: string;
  goals: string;
  success_criteria: string;
}

export interface NextItem {
  category: "blocked" | "ready" | "review" | "waiting" | "todo";
  entity_type: "solution" | "problem";
  entity_id: string;
  title: string;
  summary: string;
  suggested_command: string;
  details: { id: string; text: string; severity?: string }[];
  priority: string;
  priority_sort: number;
}

export interface NextResult {
  items: NextItem[];
  total_count: number;
  user: string;
}

export interface DashboardData {
  my_problems: Problem[];
  my_solutions: Solution[];
  open_critiques: Critique[];
}

// --- CLI Client ---

export class JjjCli {
  private get bin(): string {
    return vscode.workspace.getConfiguration("jjj").get<string>("path") || "jjj";
  }

  private get cwd(): string {
    return vscode.workspace.workspaceFolders?.[0]?.uri.fsPath || ".";
  }

  private exec(args: string[]): Promise<string> {
    return new Promise((resolve, reject) => {
      cp.execFile(this.bin, args, { cwd: this.cwd, maxBuffer: 1024 * 1024 }, (err, stdout, stderr) => {
        if (err) {
          reject(new Error(stderr || err.message));
        } else {
          resolve(stdout.trim());
        }
      });
    });
  }

  private async execJson<T>(args: string[]): Promise<T> {
    const output = await this.exec([...args, "--json"]);
    return JSON.parse(output) as T;
  }

  // --- Queries ---

  async next(all = false): Promise<NextResult> {
    const args = ["next", "--json"];
    if (all) { args.push("--all"); }
    const output = await this.exec(args);
    return JSON.parse(output) as NextResult;
  }

  async listProblems(): Promise<Problem[]> {
    return this.execJson<Problem[]>(["problem", "list"]);
  }

  async showProblem(id: string): Promise<Problem> {
    return this.execJson<Problem>(["problem", "show", id]);
  }

  async listSolutions(): Promise<Solution[]> {
    return this.execJson<Solution[]>(["solution", "list"]);
  }

  async showSolution(id: string): Promise<Solution> {
    return this.execJson<Solution>(["solution", "show", id]);
  }

  async listCritiques(): Promise<Critique[]> {
    return this.execJson<Critique[]>(["critique", "list"]);
  }

  async showCritique(id: string): Promise<Critique> {
    return this.execJson<Critique>(["critique", "show", id]);
  }

  async listMilestones(): Promise<Milestone[]> {
    return this.execJson<Milestone[]>(["milestone", "list"]);
  }

  async showMilestone(id: string): Promise<Milestone> {
    return this.execJson<Milestone>(["milestone", "show", id]);
  }

  async dashboard(): Promise<DashboardData> {
    return this.execJson<DashboardData>(["dashboard"]);
  }

  // --- Mutations ---

  async newProblem(title: string): Promise<string> {
    return this.exec(["problem", "new", title]);
  }

  async newSolution(title: string, problemId: string): Promise<string> {
    return this.exec(["solution", "new", title, "--problem", problemId]);
  }

  async newCritique(solutionId: string, title: string, severity: string, file?: string, line?: number): Promise<string> {
    const args = ["critique", "new", solutionId, title, "--severity", severity];
    if (file) { args.push("--file", file); }
    if (line !== undefined) { args.push("--line", String(line)); }
    return this.exec(args);
  }

  async replyCritique(critiqueId: string, body: string): Promise<string> {
    return this.exec(["critique", "reply", critiqueId, body]);
  }

  async addressCritique(critiqueId: string): Promise<string> {
    return this.exec(["critique", "address", critiqueId]);
  }

  async dismissCritique(critiqueId: string): Promise<string> {
    return this.exec(["critique", "dismiss", critiqueId]);
  }

  async acceptSolution(solutionId: string, force = false): Promise<string> {
    const args = ["solution", "accept", solutionId];
    if (force) { args.push("--force"); }
    return this.exec(args);
  }

  async refuteSolution(solutionId: string): Promise<string> {
    return this.exec(["solution", "refute", solutionId]);
  }

  async requestReview(solutionId: string, reviewers: string[]): Promise<string> {
    return this.exec(["solution", "review", solutionId, ...reviewers]);
  }

  async lgtm(solutionId: string, comment?: string): Promise<string> {
    const args = ["solution", "lgtm", solutionId];
    if (comment) { args.push("--comment", comment); }
    return this.exec(args);
  }

  async solveProblem(problemId: string): Promise<string> {
    return this.exec(["problem", "solve", problemId]);
  }

  async dissolveProblem(problemId: string): Promise<string> {
    return this.exec(["problem", "dissolve", problemId]);
  }

  async newMilestone(title: string): Promise<string> {
    return this.exec(["milestone", "new", title]);
  }

  async milestoneAddProblem(milestoneId: string, problemId: string): Promise<string> {
    return this.exec(["milestone", "add-problem", milestoneId, problemId]);
  }

  async milestoneRemoveProblem(milestoneId: string, problemId: string): Promise<string> {
    return this.exec(["milestone", "remove-problem", milestoneId, problemId]);
  }

  async startWorking(titleOrId: string, problemId?: string): Promise<string> {
    const args = ["start", titleOrId];
    if (problemId) { args.push("--problem", problemId); }
    return this.exec(args);
  }

  async assignProblem(problemId: string, assignee: string): Promise<string> {
    return this.exec(["problem", "assign", problemId, "--to", assignee]);
  }

  async assignSolution(solutionId: string, assignee: string): Promise<string> {
    return this.exec(["solution", "assign", solutionId, "--to", assignee]);
  }
}
