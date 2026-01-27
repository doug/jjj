# VS Code Extension Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Rebuild the JJJ VS Code extension around the Popperian model (Problems, Solutions, Critiques) with 2 sidebar views, virtual documents, inline critique threads, and drag-and-drop milestone planning.

**Architecture:** Clean rebuild in `vscode-jjj/` (new directory, separate from old `vscode/`). TypeScript extension communicating with `jjj` CLI via `--json` flags. Tree views for Next Actions and Project Tree, virtual documents for entity details, gutter decorations and inline webviews for code critiques.

**Tech Stack:** TypeScript, VS Code Extension API, `child_process` for CLI, no frameworks

**Prerequisite:** The `jjj next --json` command does not exist yet. Task 1 implements it in Rust before starting the extension.

---

## Task 1: Implement `jjj next --json` CLI Command

The VS Code extension's primary view depends on `jjj next --json`. This must be built first in Rust.

**Files:**
- Create: `src/commands/next.rs`
- Modify: `src/commands/mod.rs`
- Modify: `src/cli.rs`

**Step 1: Add NextAction enum and CLI args to cli.rs**

Add to the `Commands` enum:

```rust
    /// Show next actions (what should I work on?)
    Next {
        /// Show all items (no limit)
        #[arg(long)]
        all: bool,

        /// Only my authored work
        #[arg(long)]
        mine: bool,

        /// Show top N items (default: 5)
        #[arg(long)]
        limit: Option<usize>,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
```

**Step 2: Create src/commands/next.rs**

```rust
use crate::error::Result;
use crate::jj::JjClient;
use crate::models::{CritiqueStatus, Problem, Solution, Critique};
use crate::storage::MetadataStore;

pub fn execute(all: bool, mine: bool, limit: Option<usize>, json: bool) -> Result<()> {
    let jj_client = JjClient::new()?;
    let store = MetadataStore::new(jj_client)?;

    let user = store.jj_client.user_identity().unwrap_or_default();
    let problems = store.list_problems()?;
    let solutions = store.list_solutions()?;
    let critiques = store.list_critiques()?;

    let mut items: Vec<serde_json::Value> = Vec::new();
    let effective_limit = if all { usize::MAX } else { limit.unwrap_or(5) };

    // 1. BLOCKED: Solutions with open critiques
    for solution in solutions.iter().filter(|s| s.is_active()) {
        let open_critiques: Vec<&Critique> = critiques
            .iter()
            .filter(|c| c.solution_id == solution.id && c.status == CritiqueStatus::Open)
            .collect();

        if !open_critiques.is_empty() {
            let top_critique = open_critiques.iter()
                .max_by_key(|c| c.severity.clone())
                .unwrap();

            items.push(serde_json::json!({
                "category": "blocked",
                "entity_type": "solution",
                "entity_id": solution.id,
                "title": solution.title,
                "summary": format!("{} open critique(s)", open_critiques.len()),
                "suggested_command": format!("jjj critique show {}", top_critique.id),
                "details": open_critiques.iter().map(|c| serde_json::json!({
                    "id": c.id,
                    "text": c.title,
                    "severity": format!("{}", c.severity),
                })).collect::<Vec<_>>(),
            }));
        }
    }

    // 2. READY: Solutions with all critiques resolved + review satisfied
    for solution in solutions.iter().filter(|s| s.is_active()) {
        let has_open = critiques.iter()
            .any(|c| c.solution_id == solution.id && c.status == CritiqueStatus::Open);

        if !has_open && !solution.critique_ids.is_empty() {
            items.push(serde_json::json!({
                "category": "ready",
                "entity_type": "solution",
                "entity_id": solution.id,
                "title": solution.title,
                "summary": "All critiques resolved",
                "suggested_command": format!("jjj solution accept {}", solution.id),
                "details": [],
            }));
        }
    }

    // 3. REVIEW: Solutions where user is a requested reviewer but hasn't LGTM'd
    if !mine {
        for solution in solutions.iter().filter(|s| s.is_active()) {
            if solution.requested_reviewers.iter().any(|r| user.contains(r))
                && !solution.reviewed_by.iter().any(|r| user.contains(r))
            {
                items.push(serde_json::json!({
                    "category": "review",
                    "entity_type": "solution",
                    "entity_id": solution.id,
                    "title": solution.title,
                    "summary": format!("Review requested by {}", solution.assignee.as_deref().unwrap_or("unknown")),
                    "suggested_command": format!("jjj solution show {}", solution.id),
                    "details": [],
                }));
            }
        }
    }

    // 4. WAITING: User's solutions awaiting review
    for solution in solutions.iter().filter(|s| s.is_active()) {
        let is_mine = solution.assignee.as_ref().map(|a| user.contains(a)).unwrap_or(false);
        if is_mine && !solution.requested_reviewers.is_empty()
            && !solution.has_lgtm_from_requested_reviewer()
        {
            items.push(serde_json::json!({
                "category": "waiting",
                "entity_type": "solution",
                "entity_id": solution.id,
                "title": solution.title,
                "summary": format!("Awaiting review from {}", solution.requested_reviewers.join(", ")),
                "suggested_command": "",
                "details": [],
            }));
        }
    }

    // 5. TODO: Open problems with no active solutions
    for problem in problems.iter().filter(|p| p.is_open()) {
        let has_active_solution = solutions.iter()
            .any(|s| s.problem_id == problem.id && s.is_active());

        if !has_active_solution {
            items.push(serde_json::json!({
                "category": "todo",
                "entity_type": "problem",
                "entity_id": problem.id,
                "title": problem.title,
                "summary": "No solutions proposed",
                "suggested_command": format!("jjj start \"solution title\" --problem {}", problem.id),
                "details": [],
            }));
        }
    }

    // Apply limit
    items.truncate(effective_limit);

    if json {
        let output = serde_json::json!({
            "items": items,
            "total_count": items.len(),
            "user": user,
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        if items.is_empty() {
            println!("No pending actions. All caught up!");
            return Ok(());
        }

        println!("Next actions:\n");
        for (i, item) in items.iter().enumerate() {
            let category = item["category"].as_str().unwrap_or("").to_uppercase();
            let entity_id = item["entity_id"].as_str().unwrap_or("");
            let title = item["title"].as_str().unwrap_or("");
            let summary = item["summary"].as_str().unwrap_or("");

            println!("{}. [{}] {}: {} — {}", i + 1, category, entity_id, title, summary);

            if let Some(details) = item["details"].as_array() {
                for detail in details {
                    let id = detail["id"].as_str().unwrap_or("");
                    let text = detail["text"].as_str().unwrap_or("");
                    let severity = detail["severity"].as_str().unwrap_or("");
                    println!("   {}: {} [{}]", id, text, severity);
                }
            }

            if let Some(cmd) = item["suggested_command"].as_str() {
                if !cmd.is_empty() {
                    println!("   -> {}", cmd);
                }
            }
            println!();
        }

        let total = items.len();
        if !all && total >= effective_limit {
            println!("Showing {} items. Use --all to see everything.", effective_limit);
        }
    }

    Ok(())
}
```

**Step 3: Wire up in commands/mod.rs**

Add `pub mod next;` and add the match arm:

```rust
Commands::Next { all, mine, limit, json } => next::execute(all, mine, limit, json),
```

**Step 4: Build and test**

Run: `cargo build -p jjj && cargo test -p jjj`
Expected: Compiles and all tests pass

**Step 5: Commit**

```bash
git add src/commands/next.rs src/commands/mod.rs src/cli.rs
git commit -m "feat: add jjj next command for prioritized action list"
```

---

## Task 2: Scaffold Extension Project

**Files:**
- Create: `vscode-jjj/package.json`
- Create: `vscode-jjj/tsconfig.json`
- Create: `vscode-jjj/.vscodeignore`
- Create: `vscode-jjj/src/extension.ts`

**Step 1: Create package.json**

```json
{
  "name": "jjj-vscode",
  "displayName": "JJJ — Popperian Project Management",
  "description": "Problems, Solutions, Critiques — managed visually in VS Code",
  "version": "0.1.0",
  "publisher": "jjj",
  "license": "Apache-2.0",
  "engines": {
    "vscode": "^1.85.0"
  },
  "categories": ["Other"],
  "main": "./out/extension.js",
  "activationEvents": [],
  "contributes": {
    "configuration": {
      "title": "JJJ",
      "properties": {
        "jjj.path": {
          "type": "string",
          "default": "jjj",
          "description": "Path to the jjj executable"
        }
      }
    },
    "commands": [
      { "command": "jjj.refreshAll", "title": "JJJ: Refresh All", "icon": "$(refresh)" },
      { "command": "jjj.newProblem", "title": "JJJ: New Problem", "icon": "$(plus)" },
      { "command": "jjj.newSolution", "title": "JJJ: New Solution", "icon": "$(plus)" },
      { "command": "jjj.newCritique", "title": "JJJ: New Critique" },
      { "command": "jjj.newMilestone", "title": "JJJ: New Milestone", "icon": "$(plus)" },
      { "command": "jjj.acceptSolution", "title": "JJJ: Accept Solution" },
      { "command": "jjj.refuteSolution", "title": "JJJ: Refute Solution" },
      { "command": "jjj.requestReview", "title": "JJJ: Request Review" },
      { "command": "jjj.lgtm", "title": "JJJ: LGTM" },
      { "command": "jjj.solveProblem", "title": "JJJ: Solve Problem" },
      { "command": "jjj.dissolveProblem", "title": "JJJ: Dissolve Problem" },
      { "command": "jjj.addressCritique", "title": "JJJ: Address Critique" },
      { "command": "jjj.dismissCritique", "title": "JJJ: Dismiss Critique" },
      { "command": "jjj.addCritiqueHere", "title": "JJJ: Add Critique Here" },
      { "command": "jjj.startWorking", "title": "JJJ: Start Working" },
      { "command": "jjj.openEntity", "title": "JJJ: Open Entity" }
    ],
    "viewsContainers": {
      "activitybar": [
        {
          "id": "jjj-sidebar",
          "title": "JJJ",
          "icon": "$(beaker)"
        }
      ]
    },
    "views": {
      "jjj-sidebar": [
        { "id": "jjj-next-actions", "name": "Next Actions" },
        { "id": "jjj-project-tree", "name": "Project Tree" }
      ]
    },
    "menus": {
      "view/title": [
        { "command": "jjj.refreshAll", "when": "view == jjj-next-actions", "group": "navigation" },
        { "command": "jjj.newProblem", "when": "view == jjj-project-tree", "group": "navigation" },
        { "command": "jjj.newMilestone", "when": "view == jjj-project-tree", "group": "navigation" }
      ],
      "view/item/context": [
        { "command": "jjj.newSolution", "when": "viewItem == problem", "group": "1_actions@1" },
        { "command": "jjj.solveProblem", "when": "viewItem == problem", "group": "1_actions@2" },
        { "command": "jjj.dissolveProblem", "when": "viewItem == problem", "group": "1_actions@3" },
        { "command": "jjj.acceptSolution", "when": "viewItem == solution", "group": "1_actions@1" },
        { "command": "jjj.refuteSolution", "when": "viewItem == solution", "group": "1_actions@2" },
        { "command": "jjj.requestReview", "when": "viewItem == solution", "group": "1_actions@3" },
        { "command": "jjj.lgtm", "when": "viewItem == solution", "group": "1_actions@4" },
        { "command": "jjj.newCritique", "when": "viewItem == solution", "group": "1_actions@5" },
        { "command": "jjj.startWorking", "when": "viewItem == solution", "group": "1_actions@6" },
        { "command": "jjj.addressCritique", "when": "viewItem == critique", "group": "1_actions@1" },
        { "command": "jjj.dismissCritique", "when": "viewItem == critique", "group": "1_actions@2" },
        { "command": "jjj.newProblem", "when": "viewItem == milestone || viewItem == backlog", "group": "1_actions@1" }
      ],
      "editor/context": [
        { "command": "jjj.addCritiqueHere", "group": "jjj@1" }
      ]
    }
  },
  "scripts": {
    "compile": "tsc -p ./",
    "watch": "tsc -watch -p ./",
    "lint": "eslint src --ext ts",
    "package": "vsce package",
    "install-ext": "vsce package && code --install-extension *.vsix"
  },
  "devDependencies": {
    "@types/node": "^20.0.0",
    "@types/vscode": "^1.85.0",
    "@typescript-eslint/eslint-plugin": "^7.0.0",
    "@typescript-eslint/parser": "^7.0.0",
    "eslint": "^8.56.0",
    "typescript": "^5.3.0",
    "@vscode/vsce": "^3.0.0"
  }
}
```

**Step 2: Create tsconfig.json**

```json
{
  "compilerOptions": {
    "module": "commonjs",
    "target": "ES2022",
    "outDir": "out",
    "lib": ["ES2022"],
    "sourceMap": true,
    "rootDir": "src",
    "strict": true,
    "esModuleInterop": true,
    "resolveJsonModule": true,
    "skipLibCheck": true
  },
  "include": ["src/**/*.ts"],
  "exclude": ["node_modules", ".vscode-test"]
}
```

**Step 3: Create .vscodeignore**

```
.vscode/**
src/**
node_modules/**
.gitignore
tsconfig.json
*.ts
```

**Step 4: Create src/extension.ts (minimal stub)**

```typescript
import * as vscode from "vscode";

export function activate(context: vscode.ExtensionContext) {
  console.log("JJJ extension activated");
}

export function deactivate() {}
```

**Step 5: Install dependencies and compile**

Run: `cd vscode-jjj && npm install && npm run compile`
Expected: Compiles with no errors

**Step 6: Commit**

```bash
git add vscode-jjj/
git commit -m "feat(vscode): scaffold new extension project"
```

---

## Task 3: CLI Wrapper

**Files:**
- Create: `vscode-jjj/src/cli.ts`

**Step 1: Create the CLI wrapper with typed interfaces**

```typescript
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
  tags: string[];
  assignee: string | null;
  created_at: string;
  updated_at: string;
  description: string;
  context: string;
}

export interface Solution {
  id: string;
  title: string;
  problem_id: string;
  status: "proposed" | "testing" | "accepted" | "refuted";
  critique_ids: string[];
  change_ids: string[];
  tags: string[];
  assignee: string | null;
  requested_reviewers: string[];
  reviewed_by: string[];
  requires_review: boolean | null;
  created_at: string;
  updated_at: string;
  approach: string;
  tradeoffs: string;
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
  tags: string[];
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
  private bin: string;
  private cwd: string;

  constructor() {
    this.bin =
      vscode.workspace.getConfiguration("jjj").get<string>("path") || "jjj";
    this.cwd = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath || ".";
  }

  private exec(args: string[]): Promise<string> {
    return new Promise((resolve, reject) => {
      const cmd = [this.bin, ...args].join(" ");
      cp.exec(cmd, { cwd: this.cwd, maxBuffer: 1024 * 1024 }, (err, stdout, stderr) => {
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

  async tagList(): Promise<string[]> {
    return this.execJson<string[]>(["tag", "list"]);
  }

  // --- Mutations ---

  async newProblem(title: string): Promise<string> {
    return this.exec(["problem", "new", JSON.stringify(title)]);
  }

  async newSolution(title: string, problemId: string): Promise<string> {
    return this.exec(["solution", "new", JSON.stringify(title), "--problem", problemId]);
  }

  async newCritique(solutionId: string, title: string, severity: string, file?: string, line?: number): Promise<string> {
    const args = ["critique", "new", solutionId, JSON.stringify(title), "--severity", severity];
    if (file) { args.push("--file", file); }
    if (line !== undefined) { args.push("--line", String(line)); }
    return this.exec(args);
  }

  async replyCritique(critiqueId: string, body: string): Promise<string> {
    return this.exec(["critique", "reply", critiqueId, JSON.stringify(body)]);
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

  async lgtm(solutionId: string): Promise<string> {
    return this.exec(["solution", "lgtm", solutionId]);
  }

  async solveProblem(problemId: string): Promise<string> {
    return this.exec(["problem", "solve", problemId]);
  }

  async dissolveProblem(problemId: string): Promise<string> {
    return this.exec(["problem", "dissolve", problemId]);
  }

  async newMilestone(title: string): Promise<string> {
    return this.exec(["milestone", "new", JSON.stringify(title)]);
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
    return this.exec(["problem", "assign", problemId, assignee]);
  }

  async assignSolution(solutionId: string, assignee: string): Promise<string> {
    return this.exec(["solution", "assign", solutionId, assignee]);
  }
}
```

**Step 2: Compile**

Run: `cd vscode-jjj && npm run compile`
Expected: No errors

**Step 3: Commit**

```bash
git add vscode-jjj/src/cli.ts
git commit -m "feat(vscode): add CLI wrapper with typed interfaces"
```

---

## Task 4: Data Cache

**Files:**
- Create: `vscode-jjj/src/cache.ts`

**Step 1: Create a caching layer that batches CLI calls**

```typescript
import * as vscode from "vscode";
import { JjjCli, Problem, Solution, Critique, Milestone, NextResult } from "./cli";

export class DataCache {
  private cli: JjjCli;
  private _onDidChange = new vscode.EventEmitter<void>();
  readonly onDidChange = this._onDidChange.event;

  private problems: Problem[] = [];
  private solutions: Solution[] = [];
  private critiques: Critique[] = [];
  private milestones: Milestone[] = [];
  private nextResult: NextResult | null = null;
  private loading = false;

  constructor(cli: JjjCli) {
    this.cli = cli;
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
        this.cli.next(true).catch(() => this.nextResult),
      ]);
      this.problems = problems;
      this.solutions = solutions;
      this.critiques = critiques;
      this.milestones = milestones;
      this.nextResult = next;
      this._onDidChange.fire();
    } finally {
      this.loading = false;
    }
  }

  getProblems(): Problem[] { return this.problems; }
  getSolutions(): Solution[] { return this.solutions; }
  getCritiques(): Critique[] { return this.critiques; }
  getMilestones(): Milestone[] { return this.milestones; }
  getNext(): NextResult | null { return this.nextResult; }

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
```

**Step 2: Compile**

Run: `cd vscode-jjj && npm run compile`
Expected: No errors

**Step 3: Commit**

```bash
git add vscode-jjj/src/cache.ts
git commit -m "feat(vscode): add data cache with batch refresh"
```

---

## Task 5: Next Actions Tree View

**Files:**
- Create: `vscode-jjj/src/views/nextActionsProvider.ts`
- Modify: `vscode-jjj/src/extension.ts`

**Step 1: Create the tree data provider**

```typescript
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
```

**Step 2: Wire into extension.ts**

```typescript
import * as vscode from "vscode";
import { JjjCli } from "./cli";
import { DataCache } from "./cache";
import { NextActionsProvider } from "./views/nextActionsProvider";

export function activate(context: vscode.ExtensionContext) {
  const cli = new JjjCli();
  const cache = new DataCache(cli);

  // Views
  const nextActions = new NextActionsProvider(cache);
  vscode.window.registerTreeDataProvider("jjj-next-actions", nextActions);

  // Commands
  context.subscriptions.push(
    vscode.commands.registerCommand("jjj.refreshAll", () => cache.refresh()),
  );

  // Auto-refresh
  const interval = setInterval(() => cache.refresh(), 30000);
  context.subscriptions.push({ dispose: () => clearInterval(interval) });

  // Refresh on file save
  context.subscriptions.push(
    vscode.workspace.onDidSaveTextDocument(() => cache.refresh()),
  );

  // Initial load
  cache.refresh();
}

export function deactivate() {}
```

**Step 3: Compile**

Run: `cd vscode-jjj && npm run compile`
Expected: No errors

**Step 4: Commit**

```bash
git add vscode-jjj/src/views/nextActionsProvider.ts vscode-jjj/src/extension.ts
git commit -m "feat(vscode): add Next Actions tree view"
```

---

## Task 6: Project Tree View with Drag-and-Drop

**Files:**
- Create: `vscode-jjj/src/views/projectTreeProvider.ts`
- Modify: `vscode-jjj/src/extension.ts`

**Step 1: Create the project tree provider**

```typescript
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
    this.description = `${problem.id} [${problem.status}]`;
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

export class ProjectTreeProvider implements vscode.TreeDataProvider<TreeNode>, vscode.TreeDragAndDropController<TreeNode> {
  dropMimeTypes = [MIME_TYPE];
  dragMimeTypes = [MIME_TYPE];

  private _onDidChangeTreeData = new vscode.EventEmitter<TreeNode | undefined>();
  readonly onDidChangeTreeData = this._onDidChangeTreeData.event;

  constructor(private cache: DataCache, private cli: JjjCli) {
    cache.onDidChange(() => this._onDidChangeTreeData.fire(undefined));
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
    if (!item || !target) { return; }

    const problemIds: string[] = item.value;
    const targetMilestoneId = target instanceof MilestoneNode
      ? target.milestone?.id ?? null
      : null;

    for (const problemId of problemIds) {
      const problem = this.cache.getProblem(problemId);
      if (!problem) { continue; }

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
```

**Step 2: Wire into extension.ts**

Add to the activate function:

```typescript
import { ProjectTreeProvider } from "./views/projectTreeProvider";

// In activate():
const projectTree = new ProjectTreeProvider(cache, cli);
const treeView = vscode.window.createTreeView("jjj-project-tree", {
  treeDataProvider: projectTree,
  dragAndDropController: projectTree,
  canSelectMany: true,
});
context.subscriptions.push(treeView);
```

**Step 3: Compile**

Run: `cd vscode-jjj && npm run compile`
Expected: No errors

**Step 4: Commit**

```bash
git add vscode-jjj/src/views/projectTreeProvider.ts vscode-jjj/src/extension.ts
git commit -m "feat(vscode): add Project Tree with drag-and-drop milestone planning"
```

---

## Task 7: Virtual Document Provider

**Files:**
- Create: `vscode-jjj/src/documents/entityDocumentProvider.ts`
- Modify: `vscode-jjj/src/extension.ts`

**Step 1: Create the virtual document content provider**

```typescript
import * as vscode from "vscode";
import { DataCache } from "../cache";

export class EntityDocumentProvider implements vscode.TextDocumentContentProvider {
  private _onDidChange = new vscode.EventEmitter<vscode.Uri>();
  readonly onDidChange = this._onDidChange.event;

  constructor(private cache: DataCache) {
    cache.onDidChange(() => {
      // Refresh all open entity documents
      vscode.workspace.textDocuments.forEach(doc => {
        if (doc.uri.scheme === "jjj") {
          this._onDidChange.fire(doc.uri);
        }
      });
    });
  }

  provideTextDocumentContent(uri: vscode.Uri): string {
    const [type, id] = uri.path.split("/").filter(Boolean);

    switch (type) {
      case "problem": return this.renderProblem(id);
      case "solution": return this.renderSolution(id);
      case "critique": return this.renderCritique(id);
      case "milestone": return this.renderMilestone(id);
      default: return `Unknown entity type: ${type}`;
    }
  }

  private renderProblem(id: string): string {
    const p = this.cache.getProblem(id);
    if (!p) { return `Problem ${id} not found`; }

    const solutions = this.cache.getSolutionsForProblem(id);
    const solutionList = solutions.length > 0
      ? solutions.map(s => {
        const critiques = this.cache.getCritiquesForSolution(s.id);
        const openCount = critiques.filter(c => c.status === "open").length;
        return `  ${s.id}: ${s.title} [${s.status}] — ${openCount} open critiques`;
      }).join("\n")
      : "  (none)";

    return [
      `${p.title}`,
      "━".repeat(60),
      `Status: ${p.status}  │  Milestone: ${p.milestone_id || "Backlog"}  │  Assignee: ${p.assignee || "unassigned"}`,
      `Tags: ${p.tags.length > 0 ? p.tags.join(", ") : "none"}`,
      "",
      "## Description",
      "",
      p.description || "(no description)",
      "",
      p.context ? `## Context\n\n${p.context}\n` : "",
      `## Solutions (${solutions.length})`,
      solutionList,
    ].filter(Boolean).join("\n");
  }

  private renderSolution(id: string): string {
    const s = this.cache.getSolution(id);
    if (!s) { return `Solution ${id} not found`; }

    const critiques = this.cache.getCritiquesForSolution(id);
    const openCritiques = critiques.filter(c => c.status === "open");
    const critiqueList = critiques.length > 0
      ? critiques.map(c => {
        const loc = c.file_path ? ` — ${c.file_path}:${c.line_start}` : "";
        return `  ${c.id}: ${c.title} [${c.severity}] (${c.status})${loc}`;
      }).join("\n")
      : "  (none)";

    const reviewers = s.requested_reviewers.length > 0
      ? s.requested_reviewers.map(r => {
        const lgtm = s.reviewed_by.includes(r) ? " (LGTM)" : " (requested)";
        return `@${r}${lgtm}`;
      }).join(", ")
      : "none";

    return [
      `${s.title}`,
      "━".repeat(60),
      `Status: ${s.status}  │  Problem: ${s.problem_id}  │  Assignee: ${s.assignee || "unassigned"}`,
      `Reviewers: ${reviewers}`,
      `Tags: ${s.tags.length > 0 ? s.tags.join(", ") : "none"}`,
      "",
      "## Approach",
      "",
      s.approach || "(no approach described)",
      "",
      s.tradeoffs ? `## Tradeoffs\n\n${s.tradeoffs}\n` : "",
      `## Critiques (${openCritiques.length} open)`,
      critiqueList,
      "",
      s.change_ids.length > 0 ? `## Changes\n${s.change_ids.map(c => `  ${c}`).join("\n")}` : "",
    ].filter(Boolean).join("\n");
  }

  private renderCritique(id: string): string {
    const c = this.cache.getCritique(id);
    if (!c) { return `Critique ${id} not found`; }

    const location = c.file_path ? `${c.file_path}:${c.line_start}` : "none";
    const replies = c.replies.length > 0
      ? c.replies.map(r =>
        `@${r.author} — ${new Date(r.created_at).toLocaleString()}\n${r.body}`
      ).join("\n\n")
      : "(no replies)";

    return [
      `${c.title}`,
      "━".repeat(60),
      `Status: ${c.status}  │  Solution: ${c.solution_id}  │  Severity: ${c.severity}`,
      `Location: ${location}  │  Author: ${c.author || "unknown"}`,
      "",
      "## Argument",
      "",
      c.argument || "(no argument)",
      "",
      c.evidence ? `## Evidence\n\n${c.evidence}\n` : "",
      `## Discussion (${c.replies.length} replies)`,
      "",
      replies,
    ].filter(Boolean).join("\n");
  }

  private renderMilestone(id: string): string {
    const m = this.cache.getMilestone(id);
    if (!m) { return `Milestone ${id} not found`; }

    const problems = this.cache.getProblemsForMilestone(id);
    const solved = problems.filter(p => p.status === "solved").length;
    const problemList = problems.length > 0
      ? problems.map(p => `  ${p.id}: ${p.title} [${p.status}]`).join("\n")
      : "  (none)";

    return [
      `${m.title}`,
      "━".repeat(60),
      `Status: ${m.status}  │  Target: ${m.target_date ? new Date(m.target_date).toLocaleDateString() : "none"}`,
      `Progress: ${solved}/${problems.length} solved  │  Assignee: ${m.assignee || "unassigned"}`,
      `Tags: ${m.tags.length > 0 ? m.tags.join(", ") : "none"}`,
      "",
      "## Goals",
      "",
      m.goals || "(no goals)",
      "",
      m.success_criteria ? `## Success Criteria\n\n${m.success_criteria}\n` : "",
      `## Problems (${problems.length})`,
      problemList,
    ].filter(Boolean).join("\n");
  }
}
```

**Step 2: Wire into extension.ts and add openEntity command**

Add to activate():

```typescript
import { EntityDocumentProvider } from "./documents/entityDocumentProvider";

// In activate():
const docProvider = new EntityDocumentProvider(cache);
context.subscriptions.push(
  vscode.workspace.registerTextDocumentContentProvider("jjj", docProvider),
);

context.subscriptions.push(
  vscode.commands.registerCommand("jjj.openEntity", async (type: string, id: string) => {
    const uri = vscode.Uri.parse(`jjj:///${type}/${id}`);
    const doc = await vscode.workspace.openTextDocument(uri);
    await vscode.window.showTextDocument(doc, { preview: false });
  }),
);
```

**Step 3: Compile**

Run: `cd vscode-jjj && npm run compile`
Expected: No errors

**Step 4: Commit**

```bash
git add vscode-jjj/src/documents/entityDocumentProvider.ts vscode-jjj/src/extension.ts
git commit -m "feat(vscode): add virtual document provider for entity details"
```

---

## Task 8: Status Bar

**Files:**
- Create: `vscode-jjj/src/statusBar.ts`
- Modify: `vscode-jjj/src/extension.ts`

**Step 1: Create status bar manager**

```typescript
import * as vscode from "vscode";
import { DataCache } from "./cache";

export class StatusBar {
  private item: vscode.StatusBarItem;

  constructor(private cache: DataCache) {
    this.item = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Left, 50);
    this.item.command = "jjj-next-actions.focus";
    cache.onDidChange(() => this.update());
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
    this.item.dispose();
  }
}
```

**Step 2: Wire into extension.ts**

```typescript
import { StatusBar } from "./statusBar";

// In activate():
const statusBar = new StatusBar(cache);
context.subscriptions.push(statusBar);
```

**Step 3: Compile and commit**

```bash
cd vscode-jjj && npm run compile
git add vscode-jjj/src/statusBar.ts vscode-jjj/src/extension.ts
git commit -m "feat(vscode): add status bar with blocked/ready/review counts"
```

---

## Task 9: Command Handlers

**Files:**
- Create: `vscode-jjj/src/commands.ts`
- Modify: `vscode-jjj/src/extension.ts`

**Step 1: Create command handler registrations**

```typescript
import * as vscode from "vscode";
import { JjjCli } from "./cli";
import { DataCache } from "./cache";

export function registerCommands(
  context: vscode.ExtensionContext,
  cli: JjjCli,
  cache: DataCache,
) {
  const register = (name: string, handler: () => Promise<void>) => {
    context.subscriptions.push(
      vscode.commands.registerCommand(name, async () => {
        try {
          await handler();
          await cache.refresh();
        } catch (e: any) {
          vscode.window.showErrorMessage(`JJJ: ${e.message}`);
        }
      }),
    );
  };

  // --- Problem ---

  register("jjj.newProblem", async () => {
    const title = await vscode.window.showInputBox({ prompt: "Problem title" });
    if (!title) { return; }
    const result = await cli.newProblem(title);
    vscode.window.showInformationMessage(result);
  });

  register("jjj.solveProblem", async () => {
    const problems = cache.getProblems().filter(p => p.is_open || p.status === "open" || p.status === "in_progress");
    const pick = await vscode.window.showQuickPick(
      problems.map(p => ({ label: `${p.id}: ${p.title}`, id: p.id })),
      { placeHolder: "Select problem to solve" },
    );
    if (!pick) { return; }
    const result = await cli.solveProblem(pick.id);
    vscode.window.showInformationMessage(result);
  });

  register("jjj.dissolveProblem", async () => {
    const problems = cache.getProblems().filter(p => p.status === "open" || p.status === "in_progress");
    const pick = await vscode.window.showQuickPick(
      problems.map(p => ({ label: `${p.id}: ${p.title}`, id: p.id })),
      { placeHolder: "Select problem to dissolve" },
    );
    if (!pick) { return; }
    const result = await cli.dissolveProblem(pick.id);
    vscode.window.showInformationMessage(result);
  });

  // --- Solution ---

  register("jjj.newSolution", async () => {
    const problems = cache.getProblems().filter(p => p.status === "open" || p.status === "in_progress");
    const problemPick = await vscode.window.showQuickPick(
      problems.map(p => ({ label: `${p.id}: ${p.title}`, id: p.id })),
      { placeHolder: "Which problem does this solve?" },
    );
    if (!problemPick) { return; }
    const title = await vscode.window.showInputBox({ prompt: "Solution title" });
    if (!title) { return; }
    const result = await cli.newSolution(title, problemPick.id);
    vscode.window.showInformationMessage(result);
  });

  register("jjj.acceptSolution", async () => {
    const solutions = cache.getSolutions().filter(s => s.status === "testing");
    const pick = await vscode.window.showQuickPick(
      solutions.map(s => ({ label: `${s.id}: ${s.title}`, id: s.id })),
      { placeHolder: "Select solution to accept" },
    );
    if (!pick) { return; }
    const result = await cli.acceptSolution(pick.id);
    vscode.window.showInformationMessage(result);
  });

  register("jjj.refuteSolution", async () => {
    const solutions = cache.getSolutions().filter(s => s.status === "testing" || s.status === "proposed");
    const pick = await vscode.window.showQuickPick(
      solutions.map(s => ({ label: `${s.id}: ${s.title}`, id: s.id })),
      { placeHolder: "Select solution to refute" },
    );
    if (!pick) { return; }
    const result = await cli.refuteSolution(pick.id);
    vscode.window.showInformationMessage(result);
  });

  register("jjj.requestReview", async () => {
    const solutions = cache.getSolutions().filter(s => s.is_active || s.status === "testing" || s.status === "proposed");
    const solutionPick = await vscode.window.showQuickPick(
      solutions.map(s => ({ label: `${s.id}: ${s.title}`, id: s.id })),
      { placeHolder: "Select solution for review" },
    );
    if (!solutionPick) { return; }

    const reviewer = await vscode.window.showInputBox({
      prompt: "Reviewer name (e.g., @alice)",
      placeHolder: "@alice",
    });
    if (!reviewer) { return; }
    const result = await cli.requestReview(solutionPick.id, [reviewer]);
    vscode.window.showInformationMessage(result);
  });

  register("jjj.lgtm", async () => {
    const solutions = cache.getSolutions().filter(s => s.status === "testing" || s.status === "proposed");
    const pick = await vscode.window.showQuickPick(
      solutions.map(s => ({ label: `${s.id}: ${s.title}`, id: s.id })),
      { placeHolder: "Select solution to LGTM" },
    );
    if (!pick) { return; }
    const result = await cli.lgtm(pick.id);
    vscode.window.showInformationMessage(result);
  });

  register("jjj.startWorking", async () => {
    const problems = cache.getProblems().filter(p => p.status === "open" || p.status === "in_progress");
    const problemPick = await vscode.window.showQuickPick(
      problems.map(p => ({ label: `${p.id}: ${p.title}`, id: p.id })),
      { placeHolder: "Which problem to work on?" },
    );
    if (!problemPick) { return; }
    const title = await vscode.window.showInputBox({ prompt: "Solution title" });
    if (!title) { return; }
    const result = await cli.startWorking(title, problemPick.id);
    vscode.window.showInformationMessage(result);
  });

  // --- Critique ---

  register("jjj.newCritique", async () => {
    const solutions = cache.getSolutions().filter(s => s.status === "testing" || s.status === "proposed");
    const solutionPick = await vscode.window.showQuickPick(
      solutions.map(s => ({ label: `${s.id}: ${s.title}`, id: s.id })),
      { placeHolder: "Which solution to critique?" },
    );
    if (!solutionPick) { return; }
    const title = await vscode.window.showInputBox({ prompt: "Critique title" });
    if (!title) { return; }
    const severity = await vscode.window.showQuickPick(
      ["low", "medium", "high", "critical"],
      { placeHolder: "Severity" },
    );
    if (!severity) { return; }
    const result = await cli.newCritique(solutionPick.id, title, severity);
    vscode.window.showInformationMessage(result);
  });

  register("jjj.addressCritique", async () => {
    const critiques = cache.getCritiques().filter(c => c.status === "open");
    const pick = await vscode.window.showQuickPick(
      critiques.map(c => ({ label: `${c.id}: ${c.title} [${c.severity}]`, id: c.id })),
      { placeHolder: "Select critique to address" },
    );
    if (!pick) { return; }
    const result = await cli.addressCritique(pick.id);
    vscode.window.showInformationMessage(result);
  });

  register("jjj.dismissCritique", async () => {
    const critiques = cache.getCritiques().filter(c => c.status === "open");
    const pick = await vscode.window.showQuickPick(
      critiques.map(c => ({ label: `${c.id}: ${c.title} [${c.severity}]`, id: c.id })),
      { placeHolder: "Select critique to dismiss" },
    );
    if (!pick) { return; }
    const result = await cli.dismissCritique(pick.id);
    vscode.window.showInformationMessage(result);
  });

  // --- Milestone ---

  register("jjj.newMilestone", async () => {
    const title = await vscode.window.showInputBox({ prompt: "Milestone title" });
    if (!title) { return; }
    const result = await cli.newMilestone(title);
    vscode.window.showInformationMessage(result);
  });

  // --- Editor: Add Critique Here ---

  register("jjj.addCritiqueHere", async () => {
    const editor = vscode.window.activeTextEditor;
    if (!editor) { return; }

    const line = editor.selection.active.line + 1; // 1-based
    const filePath = vscode.workspace.asRelativePath(editor.document.uri);

    const solutions = cache.getSolutions().filter(s => s.status === "testing" || s.status === "proposed");
    const solutionPick = await vscode.window.showQuickPick(
      solutions.map(s => ({ label: `${s.id}: ${s.title}`, id: s.id })),
      { placeHolder: "Which solution to critique?" },
    );
    if (!solutionPick) { return; }

    const title = await vscode.window.showInputBox({ prompt: "Critique title" });
    if (!title) { return; }

    const severity = await vscode.window.showQuickPick(
      ["low", "medium", "high", "critical"],
      { placeHolder: "Severity" },
    );
    if (!severity) { return; }

    const result = await cli.newCritique(solutionPick.id, title, severity, filePath, line);
    vscode.window.showInformationMessage(result);
  });
}
```

**Step 2: Wire into extension.ts**

```typescript
import { registerCommands } from "./commands";

// In activate():
registerCommands(context, cli, cache);
```

**Step 3: Compile and commit**

```bash
cd vscode-jjj && npm run compile
git add vscode-jjj/src/commands.ts vscode-jjj/src/extension.ts
git commit -m "feat(vscode): add command handlers for all entity actions"
```

---

## Task 10: Critique Gutter Decorations

**Files:**
- Create: `vscode-jjj/src/editor/critiqueDecorations.ts`
- Modify: `vscode-jjj/src/extension.ts`

**Step 1: Create decoration provider for gutter icons**

```typescript
import * as vscode from "vscode";
import { DataCache } from "../cache";
import { Critique } from "../cli";

const criticalDecoration = vscode.window.createTextEditorDecorationType({
  gutterIconPath: undefined, // Will use ThemeIcon via ranges
  overviewRulerColor: "red",
  overviewRulerLane: vscode.OverviewRulerLane.Left,
  before: {
    contentText: "●",
    color: new vscode.ThemeColor("errorForeground"),
    margin: "0 4px 0 0",
  },
});

const warningDecoration = vscode.window.createTextEditorDecorationType({
  overviewRulerColor: "yellow",
  overviewRulerLane: vscode.OverviewRulerLane.Left,
  before: {
    contentText: "●",
    color: new vscode.ThemeColor("editorWarning.foreground"),
    margin: "0 4px 0 0",
  },
});

const resolvedDecoration = vscode.window.createTextEditorDecorationType({
  overviewRulerColor: "gray",
  overviewRulerLane: vscode.OverviewRulerLane.Left,
  before: {
    contentText: "○",
    color: new vscode.ThemeColor("disabledForeground"),
    margin: "0 4px 0 0",
  },
});

export class CritiqueDecorationManager {
  private disposables: vscode.Disposable[] = [];

  constructor(private cache: DataCache) {
    this.disposables.push(
      vscode.window.onDidChangeActiveTextEditor(() => this.updateDecorations()),
      cache.onDidChange(() => this.updateDecorations()),
    );
    this.updateDecorations();
  }

  private updateDecorations() {
    const editor = vscode.window.activeTextEditor;
    if (!editor) { return; }

    const filePath = vscode.workspace.asRelativePath(editor.document.uri);
    const critiques = this.cache.getCritiquesWithLocations()
      .filter(c => c.file_path === filePath);

    const critical: vscode.DecorationOptions[] = [];
    const warning: vscode.DecorationOptions[] = [];
    const resolved: vscode.DecorationOptions[] = [];

    for (const c of critiques) {
      if (!c.line_start) { continue; }
      const line = c.line_start - 1; // 0-based
      const range = new vscode.Range(line, 0, line, 0);
      const option: vscode.DecorationOptions = {
        range,
        hoverMessage: new vscode.MarkdownString(
          `**${c.id}**: ${c.title} [${c.severity}]\n\n${c.argument}\n\n*${c.status}*`,
        ),
      };

      if (c.status !== "open") {
        resolved.push(option);
      } else if (c.severity === "high" || c.severity === "critical") {
        critical.push(option);
      } else {
        warning.push(option);
      }
    }

    editor.setDecorations(criticalDecoration, critical);
    editor.setDecorations(warningDecoration, warning);
    editor.setDecorations(resolvedDecoration, resolved);
  }

  dispose() {
    this.disposables.forEach(d => d.dispose());
    criticalDecoration.dispose();
    warningDecoration.dispose();
    resolvedDecoration.dispose();
  }
}
```

**Step 2: Wire into extension.ts**

```typescript
import { CritiqueDecorationManager } from "./editor/critiqueDecorations";

// In activate():
const decorations = new CritiqueDecorationManager(cache);
context.subscriptions.push(decorations);
```

**Step 3: Compile and commit**

```bash
cd vscode-jjj && npm run compile
git add vscode-jjj/src/editor/critiqueDecorations.ts vscode-jjj/src/extension.ts
git commit -m "feat(vscode): add gutter decorations for code-level critiques"
```

---

## Task 11: Final Extension Wiring and Build Verification

**Files:**
- Modify: `vscode-jjj/src/extension.ts` (final consolidated version)

**Step 1: Write the final extension.ts combining all components**

```typescript
import * as vscode from "vscode";
import { JjjCli } from "./cli";
import { DataCache } from "./cache";
import { NextActionsProvider } from "./views/nextActionsProvider";
import { ProjectTreeProvider } from "./views/projectTreeProvider";
import { EntityDocumentProvider } from "./documents/entityDocumentProvider";
import { StatusBar } from "./statusBar";
import { CritiqueDecorationManager } from "./editor/critiqueDecorations";
import { registerCommands } from "./commands";

export function activate(context: vscode.ExtensionContext) {
  const cli = new JjjCli();
  const cache = new DataCache(cli);

  // --- Views ---
  const nextActions = new NextActionsProvider(cache);
  vscode.window.registerTreeDataProvider("jjj-next-actions", nextActions);

  const projectTree = new ProjectTreeProvider(cache, cli);
  const treeView = vscode.window.createTreeView("jjj-project-tree", {
    treeDataProvider: projectTree,
    dragAndDropController: projectTree,
    canSelectMany: true,
  });
  context.subscriptions.push(treeView);

  // --- Virtual Documents ---
  const docProvider = new EntityDocumentProvider(cache);
  context.subscriptions.push(
    vscode.workspace.registerTextDocumentContentProvider("jjj", docProvider),
  );

  context.subscriptions.push(
    vscode.commands.registerCommand("jjj.openEntity", async (type: string, id: string) => {
      const uri = vscode.Uri.parse(`jjj:///${type}/${id}`);
      const doc = await vscode.workspace.openTextDocument(uri);
      await vscode.window.showTextDocument(doc, { preview: false });
    }),
  );

  // --- Status Bar ---
  const statusBar = new StatusBar(cache);
  context.subscriptions.push(statusBar);

  // --- Gutter Decorations ---
  const decorations = new CritiqueDecorationManager(cache);
  context.subscriptions.push(decorations);

  // --- Commands ---
  context.subscriptions.push(
    vscode.commands.registerCommand("jjj.refreshAll", () => cache.refresh()),
  );
  registerCommands(context, cli, cache);

  // --- Auto-Refresh ---
  const interval = setInterval(() => cache.refresh(), 30000);
  context.subscriptions.push({ dispose: () => clearInterval(interval) });
  context.subscriptions.push(
    vscode.workspace.onDidSaveTextDocument(() => cache.refresh()),
  );

  // --- Initial Load ---
  cache.refresh();

  console.log("JJJ extension activated");
}

export function deactivate() {}
```

**Step 2: Full build verification**

Run: `cd vscode-jjj && npm install && npm run compile`
Expected: Compiles with no errors

**Step 3: Package the extension**

Run: `cd vscode-jjj && npx vsce package --allow-missing-repository`
Expected: Produces `.vsix` file

**Step 4: Commit**

```bash
git add vscode-jjj/
git commit -m "feat(vscode): complete extension with all views, commands, and decorations"
```

---

## Summary

After completing all 11 tasks:

1. `jjj next --json` CLI command implemented in Rust
2. Extension project scaffolded with package.json and manifest
3. CLI wrapper with typed TypeScript interfaces for all entities
4. Data cache with batch refresh and change events
5. Next Actions tree view (primary sidebar)
6. Project Tree with drag-and-drop milestone planning
7. Virtual document provider for Problem/Solution/Critique/Milestone detail views
8. Status bar showing blocked/ready/review counts
9. Command handlers for all entity actions (new, accept, refute, review, etc.)
10. Gutter decorations for code-level critiques with hover details
11. Final wiring and build verification

Run final verification:
```bash
cargo build -p jjj && cargo test -p jjj
cd vscode-jjj && npm run compile
```
