import * as assert from "assert";
import * as sinon from "sinon";
import * as vscode from "vscode";
import { EntityDocumentProvider } from "../../src/documents/entityDocumentProvider";
import { DataCache } from "../../src/cache";
import { Problem, Solution, Critique, Milestone, JjjCli } from "../../src/cli";

function makeProblem(overrides: Partial<Problem> = {}): Problem {
  return {
    id: "P-1", title: "Search is slow", parent_id: null, status: "open",
    solution_ids: ["S-1"], child_ids: [], milestone_id: "M-1",
    assignee: "doug", created_at: "", updated_at: "",
    description: "Search queries take 3+ seconds", context: "",
    priority: "medium", dissolved_reason: null,
    ...overrides,
  };
}

function makeSolution(overrides: Partial<Solution> = {}): Solution {
  return {
    id: "S-1", title: "Add search index", problem_id: "P-1", status: "testing",
    critique_ids: ["CQ-1"], change_ids: ["kxq2p"],
    assignee: "doug", reviewers: ["alice"],
    sign_offs: [{ reviewer: "alice", at: "2026-01-27T15:30:00Z", comment: "looks good" }],
    force_accepted: false, created_at: "", updated_at: "",
    approach: "Add a B-tree index", tradeoffs: "Extra storage",
    supersedes: null,
    ...overrides,
  };
}

function makeCritique(overrides: Partial<Critique> = {}): Critique {
  return {
    id: "CQ-1", title: "SQL injection risk", solution_id: "S-1", status: "open",
    severity: "high", author: "alice", created_at: "", updated_at: "",
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
    id: "M-1", title: "v0.2 Release", target_date: "2025-03-01", status: "active",
    problem_ids: ["P-1"], assignee: "doug", created_at: "", updated_at: "",
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
    cli.next.resolves({ items: [], total_count: 0, user: "test" });

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
      const uri = vscode.Uri.parse("jjj:///problem/P-1.md");
      const content = provider.provideTextDocumentContent(uri);
      assert.ok(content.includes("Search is slow"));
    });

    it("works without .md suffix", () => {
      const uri = vscode.Uri.parse("jjj:///problem/P-1");
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
      const uri = vscode.Uri.parse("jjj:///problem/P-1.md");
      const content = provider.provideTextDocumentContent(uri);
      assert.ok(content.includes("Search is slow"));
    });

    it("includes status and milestone", () => {
      const uri = vscode.Uri.parse("jjj:///problem/P-1.md");
      const content = provider.provideTextDocumentContent(uri);
      assert.ok(content.includes("Status: open"));
      assert.ok(content.includes("Milestone: M-1"));
    });

    it("includes description", () => {
      const uri = vscode.Uri.parse("jjj:///problem/P-1.md");
      const content = provider.provideTextDocumentContent(uri);
      assert.ok(content.includes("Search queries take 3+ seconds"));
    });

    it("lists solutions", () => {
      const uri = vscode.Uri.parse("jjj:///problem/P-1.md");
      const content = provider.provideTextDocumentContent(uri);
      assert.ok(content.includes("S-1: Add search index"));
      assert.ok(content.includes("[testing]"));
    });

    it("includes priority", () => {
      const uri = vscode.Uri.parse("jjj:///problem/P-1.md");
      const content = provider.provideTextDocumentContent(uri);
      assert.ok(content.includes("Priority: medium"));
    });

    it("shows dissolved reason when dissolved", async () => {
      const dissolvedProblem = makeProblem({
        id: "P-2", status: "dissolved",
        dissolved_reason: "The data was correct; our test was wrong",
      });
      cli.listProblems.resolves([makeProblem(), dissolvedProblem]);
      await cache.refresh();

      const uri = vscode.Uri.parse("jjj:///problem/P-2.md");
      const content = provider.provideTextDocumentContent(uri);
      assert.ok(content.includes("Dissolved Reason"));
      assert.ok(content.includes("our test was wrong"));
    });

    it("shows not found for unknown id", () => {
      const uri = vscode.Uri.parse("jjj:///problem/P-99.md");
      const content = provider.provideTextDocumentContent(uri);
      assert.ok(content.includes("not found"));
    });
  });

  describe("solution rendering", () => {
    it("includes title and status", () => {
      const uri = vscode.Uri.parse("jjj:///solution/S-1.md");
      const content = provider.provideTextDocumentContent(uri);
      assert.ok(content.includes("Add search index"));
      assert.ok(content.includes("Status: testing"));
    });

    it("includes reviewer sign-off status", () => {
      const uri = vscode.Uri.parse("jjj:///solution/S-1.md");
      const content = provider.provideTextDocumentContent(uri);
      assert.ok(content.includes("@alice: signed off"));
      assert.ok(content.includes("looks good"));
    });

    it("lists critiques", () => {
      const uri = vscode.Uri.parse("jjj:///solution/S-1.md");
      const content = provider.provideTextDocumentContent(uri);
      assert.ok(content.includes("CQ-1: SQL injection risk [high]"));
    });

    it("includes change ids", () => {
      const uri = vscode.Uri.parse("jjj:///solution/S-1.md");
      const content = provider.provideTextDocumentContent(uri);
      assert.ok(content.includes("kxq2p"));
    });

    it("shows supersedes when set", async () => {
      const solution2 = makeSolution({
        id: "S-2", title: "Use connection pool", supersedes: "S-1",
      });
      cli.listSolutions.resolves([makeSolution(), solution2]);
      await cache.refresh();

      const uri = vscode.Uri.parse("jjj:///solution/S-2.md");
      const content = provider.provideTextDocumentContent(uri);
      assert.ok(content.includes("Supersedes: S-1"));
    });

    it("shows pending reviewers", async () => {
      const solution2 = makeSolution({
        id: "S-2", title: "Pending review", reviewers: ["bob"],
        sign_offs: [], force_accepted: false,
      });
      cli.listSolutions.resolves([makeSolution(), solution2]);
      await cache.refresh();

      const uri = vscode.Uri.parse("jjj:///solution/S-2.md");
      const content = provider.provideTextDocumentContent(uri);
      assert.ok(content.includes("@bob: pending"));
    });

    it("shows non-assigned endorsements", async () => {
      const solution3 = makeSolution({
        id: "S-3", title: "Endorsed", reviewers: ["alice"],
        sign_offs: [
          { reviewer: "alice", at: "2026-01-27T15:30:00Z", comment: undefined },
          { reviewer: "charlie", at: "2026-01-27T16:00:00Z", comment: "nice work" },
        ],
        force_accepted: false,
      });
      cli.listSolutions.resolves([makeSolution(), solution3]);
      await cache.refresh();

      const uri = vscode.Uri.parse("jjj:///solution/S-3.md");
      const content = provider.provideTextDocumentContent(uri);
      assert.ok(content.includes("Also endorsed by"));
      assert.ok(content.includes("charlie"));
    });
  });

  describe("critique rendering", () => {
    it("includes severity and location", () => {
      const uri = vscode.Uri.parse("jjj:///critique/CQ-1.md");
      const content = provider.provideTextDocumentContent(uri);
      assert.ok(content.includes("Severity: high"));
      assert.ok(content.includes("src/db.rs:42"));
    });

    it("includes argument", () => {
      const uri = vscode.Uri.parse("jjj:///critique/CQ-1.md");
      const content = provider.provideTextDocumentContent(uri);
      assert.ok(content.includes("The query concatenates user input"));
    });

    it("includes replies", () => {
      const uri = vscode.Uri.parse("jjj:///critique/CQ-1.md");
      const content = provider.provideTextDocumentContent(uri);
      assert.ok(content.includes("@bob"));
      assert.ok(content.includes("Good catch"));
    });
  });

  describe("milestone rendering", () => {
    it("includes title and progress", () => {
      const uri = vscode.Uri.parse("jjj:///milestone/M-1.md");
      const content = provider.provideTextDocumentContent(uri);
      assert.ok(content.includes("v0.2 Release"));
      assert.ok(content.includes("0/1 solved"));
    });

    it("includes goals", () => {
      const uri = vscode.Uri.parse("jjj:///milestone/M-1.md");
      const content = provider.provideTextDocumentContent(uri);
      assert.ok(content.includes("Ship search improvements"));
    });

    it("lists problems", () => {
      const uri = vscode.Uri.parse("jjj:///milestone/M-1.md");
      const content = provider.provideTextDocumentContent(uri);
      assert.ok(content.includes("P-1: Search is slow"));
    });
  });
});
