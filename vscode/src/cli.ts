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
  github_issue?: number | null;
  tags: string[];
}

export interface Solution {
  id: string;
  title: string;
  problem_id: string;
  status: "proposed" | "submitted" | "approved" | "withdrawn";
  critique_ids: string[];
  change_ids: string[];
  assignee: string | null;
  force_approved: boolean;
  created_at: string;
  updated_at: string;
  approach: string;
  tradeoffs: string;
  supersedes: string | null;
  github_pr?: number | null;
  github_branch?: string | null;
  tags: string[];
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
  reviewer?: string;
  created_at: string;
  updated_at: string;
  argument: string;
  evidence: string;
  file_path: string | null;
  line_start: number | null;
  line_end: number | null;
  code_context: string[];
  context_before?: string[];
  context_after?: string[];
  replies: Reply[];
  github_review_id?: number | null;
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

export interface StatusItem {
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

export interface StatusResult {
  active_solution: { id: string; title: string; problem_id: string; status: string } | null;
  items: StatusItem[];
  total_count: number;
  user: string;
  summary: { open_problems: number; review_solutions: number; open_critiques: number };
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

  async version(): Promise<string> {
    return this.exec(["--version"]);
  }

  // --- Queries ---

  async status(all = false): Promise<StatusResult> {
    const args = ["status", "--json"];
    if (all) { args.push("--all"); }
    const output = await this.exec(args);
    return JSON.parse(output) as StatusResult;
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

  // --- Mutations ---

  async newProblem(title: string): Promise<string> {
    return this.exec(["problem", "new", title]);
  }

  async newSolution(title: string, problemId: string, reviewers?: string[]): Promise<string> {
    const args = ["solution", "new", title, "--problem", problemId];
    for (const reviewer of reviewers ?? []) {
      args.push("--reviewer", reviewer);
    }
    return this.exec(args);
  }

  async newCritique(
    solutionId: string,
    title: string,
    severity: string,
    filePath?: string,
    line?: number,
    reviewer?: string
  ): Promise<string> {
    const args = ["critique", "new", solutionId, title, "--severity", severity];
    if (filePath) {
      args.push("--file", filePath);
      if (line) {args.push("--line", String(line));}
    }
    if (reviewer) {
      args.push("--reviewer", reviewer);
    }
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

  async validateCritique(critiqueId: string): Promise<string> {
    return this.exec(["critique", "validate", critiqueId]);
  }

  async submitSolution(solutionId: string): Promise<string> {
    return this.exec(["solution", "submit", solutionId]);
  }

  async approveSolution(solutionId: string, force = false): Promise<string> {
    const args = ["solution", "approve", solutionId];
    if (force) { args.push("--force"); }
    return this.exec(args);
  }

  async withdrawSolution(solutionId: string): Promise<string> {
    return this.exec(["solution", "withdraw", solutionId]);
  }

  async lgtmSolution(solutionId: string): Promise<string> {
    return this.exec(["solution", "lgtm", solutionId]);
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

  async resumeSolution(solutionId: string): Promise<string> {
    return this.exec(["solution", "resume", solutionId]);
  }

  async assignProblem(problemId: string, assignee: string): Promise<string> {
    return this.exec(["problem", "assign", problemId, "--to", assignee]);
  }

  async assignSolution(solutionId: string, assignee: string): Promise<string> {
    return this.exec(["solution", "assign", solutionId, "--to", assignee]);
  }

  async reopenProblem(problemId: string): Promise<string> {
    return this.exec(["problem", "reopen", problemId]);
  }

  async editProblem(problemId: string, title: string): Promise<string> {
    return this.exec(["problem", "edit", problemId, "--title", title]);
  }

  async editSolution(solutionId: string, title: string): Promise<string> {
    return this.exec(["solution", "edit", solutionId, "--title", title]);
  }

  async editProblemTags(id: string, tags: string[]): Promise<string> {
    return this.exec(["problem", "edit", id, "--set-tags", tags.join(",") || ""]);
  }

  async editSolutionTags(id: string, tags: string[]): Promise<string> {
    return this.exec(["solution", "edit", id, "--set-tags", tags.join(",") || ""]);
  }

  async listTags(): Promise<{tag: string; count: number}[]> {
    return this.execJson<{tag: string; count: number}[]>(["tags"]);
  }

  // --- GitHub Sync ---

  async syncGithub(): Promise<string> {
    return this.exec(["github"]);
  }

  async syncGithubImport(issue: string): Promise<string> {
    return this.exec(["github", "import", issue]);
  }

  async syncGithubStatus(): Promise<string> {
    return this.exec(["github", "status"]);
  }
}
