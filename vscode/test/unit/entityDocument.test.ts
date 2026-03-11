import * as assert from "assert";
import * as sinon from "sinon";
import * as vscode from "vscode";
import { EntityDocumentProvider } from "../../src/documents/entityDocumentProvider";
import { DataCache } from "../../src/cache";
import { Problem, Solution, Critique, Milestone, JjjCli } from "../../src/cli";

function makeProblem(overrides: Partial<Problem> = {}): Problem {
  return {
    id: "p1", title: "Search is slow", parent_id: null, status: "open",
    solution_ids: ["s1"], child_ids: [], milestone_id: "m1",
    assignee: "doug", created_at: "", updated_at: "",
    description: "Search queries take 3+ seconds", context: "",
    priority: "medium", dissolved_reason: null, tags: [],
    ...overrides,
  };
}

function makeSolution(overrides: Partial<Solution> = {}): Solution {
  return {
    id: "s1", title: "Add search index", problem_id: "p1", status: "submitted",
    critique_ids: ["c1"], change_ids: ["kxq2p"],
    assignee: "doug",
    force_approved: false, created_at: "", updated_at: "",
    approach: "Add a B-tree index", tradeoffs: "Extra storage",
    supersedes: null, tags: [],
    ...overrides,
  };
}

function makeCritique(overrides: Partial<Critique> = {}): Critique {
  return {
    id: "c1", title: "SQL injection risk", solution_id: "s1", status: "open",
    severity: "high", author: "alice", reviewer: "alice", created_at: "", updated_at: "",
    argument: "The query concatenates user input", evidence: "See line 42",
    file_path: "src/db.rs", line_start: 42, line_end: null,
    code_context: [], replies: [
      { id: "R-1", author: "bob", body: "Good catch", created_at: "2025-01-25T10:30:00Z" },
    ],
    ...overrides,
  };
}

function makeMilestone(overrides: Partial<Milestone> = {}): Milestone {
  return {
    id: "m1", title: "v0.2 Release", target_date: "2025-03-01", status: "active",
    problem_ids: ["p1"], assignee: "doug", created_at: "", updated_at: "",
    goals: "Ship search improvements", success_criteria: "Sub-second queries",
    ...overrides,
  };
}

describe("EntityDocumentProvider", () => {
  let provider: EntityDocumentProvider;
  let cache: DataCache;
  let cli: sinon.SinonStubbedInstance<JjjCli>;

  beforeEach(async () => {
    cli = sinon.createStubInstance(JjjCli);
    cli.listProblems.resolves([makeProblem()]);
    cli.listSolutions.resolves([makeSolution()]);
    cli.listCritiques.resolves([makeCritique()]);
    cli.listMilestones.resolves([makeMilestone()]);
    cli.status.resolves({ active_solution: null, items: [], total_count: 0, user: "test", summary: { open_problems: 0, review_solutions: 0, open_critiques: 0 } });

    cache = new DataCache(cli as unknown as JjjCli);
    await cache.refresh();

    provider = new EntityDocumentProvider(cache);
  });

  afterEach(() => {
    provider.dispose();
    cache.dispose();
    sinon.restore();
  });

  describe("URI parsing", () => {
    it("strips .md suffix from id", () => {
      const uri = vscode.Uri.parse("jjj:///problem/p1.md");
      const content = provider.provideTextDocumentContent(uri);
      assert.ok(content.includes("Search is slow"));
    });

    it("works without .md suffix", () => {
      const uri = vscode.Uri.parse("jjj:///problem/p1");
      const content = provider.provideTextDocumentContent(uri);
      assert.ok(content.includes("Search is slow"));
    });

    it("returns error for unknown type", () => {
      const uri = vscode.Uri.parse("jjj:///unknown/X-1.md");
      const content = provider.provideTextDocumentContent(uri);
      assert.ok(content.includes("Unknown entity type"));
    });
  });

  describe("problem rendering", () => {
    it("includes title", () => {
      const uri = vscode.Uri.parse("jjj:///problem/p1.md");
      const content = provider.provideTextDocumentContent(uri);
      assert.ok(content.includes("Search is slow"));
    });

    it("includes status and milestone", () => {
      const uri = vscode.Uri.parse("jjj:///problem/p1.md");
      const content = provider.provideTextDocumentContent(uri);
      assert.ok(content.includes("Status: open"));
      assert.ok(content.includes("Milestone: m1"));
    });

    it("includes description", () => {
      const uri = vscode.Uri.parse("jjj:///problem/p1.md");
      const content = provider.provideTextDocumentContent(uri);
      assert.ok(content.includes("Search queries take 3+ seconds"));
    });

    it("lists solutions", () => {
      const uri = vscode.Uri.parse("jjj:///problem/p1.md");
      const content = provider.provideTextDocumentContent(uri);
      assert.ok(content.includes("s1: Add search index"));
      assert.ok(content.includes("[submitted]"));
    });

    it("includes priority", () => {
      const uri = vscode.Uri.parse("jjj:///problem/p1.md");
      const content = provider.provideTextDocumentContent(uri);
      assert.ok(content.includes("Priority: medium"));
    });

    it("shows dissolved reason when dissolved", async () => {
      const dissolvedProblem = makeProblem({
        id: "p2", status: "dissolved",
        dissolved_reason: "The data was correct; our test was wrong",
      });
      cli.listProblems.resolves([makeProblem(), dissolvedProblem]);
      await cache.refresh();

      const uri = vscode.Uri.parse("jjj:///problem/p2.md");
      const content = provider.provideTextDocumentContent(uri);
      assert.ok(content.includes("Dissolved Reason"));
      assert.ok(content.includes("our test was wrong"));
    });

    it("shows not found for unknown id", () => {
      const uri = vscode.Uri.parse("jjj:///problem/p99.md");
      const content = provider.provideTextDocumentContent(uri);
      assert.ok(content.includes("not found"));
    });
  });

  describe("solution rendering", () => {
    it("includes title and status", () => {
      const uri = vscode.Uri.parse("jjj:///solution/s1.md");
      const content = provider.provideTextDocumentContent(uri);
      assert.ok(content.includes("Add search index"));
      assert.ok(content.includes("Status: submitted"));
    });

    it("lists critiques", () => {
      const uri = vscode.Uri.parse("jjj:///solution/s1.md");
      const content = provider.provideTextDocumentContent(uri);
      assert.ok(content.includes("c1: SQL injection risk [high]"));
    });

    it("includes change ids", () => {
      const uri = vscode.Uri.parse("jjj:///solution/s1.md");
      const content = provider.provideTextDocumentContent(uri);
      assert.ok(content.includes("kxq2p"));
    });

    it("shows supersedes when set", async () => {
      const solution2 = makeSolution({
        id: "s2", title: "Use connection pool", supersedes: "s1",
      });
      cli.listSolutions.resolves([makeSolution(), solution2]);
      await cache.refresh();

      const uri = vscode.Uri.parse("jjj:///solution/s2.md");
      const content = provider.provideTextDocumentContent(uri);
      assert.ok(content.includes("Supersedes: s1"));
    });

  });

  describe("critique rendering", () => {
    it("includes severity and location", () => {
      const uri = vscode.Uri.parse("jjj:///critique/c1.md");
      const content = provider.provideTextDocumentContent(uri);
      assert.ok(content.includes("Severity: high"));
      assert.ok(content.includes("src/db.rs:42"));
    });

    it("includes argument", () => {
      const uri = vscode.Uri.parse("jjj:///critique/c1.md");
      const content = provider.provideTextDocumentContent(uri);
      assert.ok(content.includes("The query concatenates user input"));
    });

    it("includes replies", () => {
      const uri = vscode.Uri.parse("jjj:///critique/c1.md");
      const content = provider.provideTextDocumentContent(uri);
      assert.ok(content.includes("@bob"));
      assert.ok(content.includes("Good catch"));
    });
  });

  describe("milestone rendering", () => {
    it("includes title and progress", () => {
      const uri = vscode.Uri.parse("jjj:///milestone/m1.md");
      const content = provider.provideTextDocumentContent(uri);
      assert.ok(content.includes("v0.2 Release"));
      assert.ok(content.includes("0/1 solved"));
    });

    it("includes goals", () => {
      const uri = vscode.Uri.parse("jjj:///milestone/m1.md");
      const content = provider.provideTextDocumentContent(uri);
      assert.ok(content.includes("Ship search improvements"));
    });

    it("lists problems", () => {
      const uri = vscode.Uri.parse("jjj:///milestone/m1.md");
      const content = provider.provideTextDocumentContent(uri);
      assert.ok(content.includes("p1: Search is slow"));
    });
  });
});
