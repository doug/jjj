import * as assert from "assert";
import * as sinon from "sinon";
import { DataCache } from "../../src/cache";
import { JjjCli, Problem, Solution, Critique, Milestone } from "../../src/cli";

function makeProblem(overrides: Partial<Problem> = {}): Problem {
  return {
    id: "P-1", title: "Test problem", parent_id: null, status: "open",
    solution_ids: [], child_ids: [], milestone_id: null, tags: [],
    assignee: null, created_at: "", updated_at: "", description: "", context: "",
    priority: "medium", dissolved_reason: null,
    ...overrides,
  };
}

function makeSolution(overrides: Partial<Solution> = {}): Solution {
  return {
    id: "S-1", title: "Test solution", problem_id: "P-1", status: "proposed",
    critique_ids: [], change_ids: [], tags: [], assignee: null,
    requested_reviewers: [], reviewed_by: [], requires_review: null,
    created_at: "", updated_at: "", approach: "", tradeoffs: "",
    supersedes: null,
    ...overrides,
  };
}

function makeCritique(overrides: Partial<Critique> = {}): Critique {
  return {
    id: "CQ-1", title: "Test critique", solution_id: "S-1", status: "open",
    severity: "medium", author: null, created_at: "", updated_at: "",
    argument: "", evidence: "", file_path: null, line_start: null,
    line_end: null, code_context: [], replies: [],
    ...overrides,
  };
}

function makeMilestone(overrides: Partial<Milestone> = {}): Milestone {
  return {
    id: "M-1", title: "Test milestone", target_date: null, status: "active",
    problem_ids: [], tags: [], assignee: null, created_at: "", updated_at: "",
    goals: "", success_criteria: "",
    ...overrides,
  };
}

describe("DataCache", () => {
  let cache: DataCache;
  let cli: sinon.SinonStubbedInstance<JjjCli>;

  beforeEach(async () => {
    cli = sinon.createStubInstance(JjjCli);
    cli.listProblems.resolves([
      makeProblem({ id: "P-1", milestone_id: "M-1" }),
      makeProblem({ id: "P-2", milestone_id: "M-1" }),
      makeProblem({ id: "P-3", milestone_id: null }),
      makeProblem({ id: "P-4", milestone_id: null }),
    ]);
    cli.listSolutions.resolves([
      makeSolution({ id: "S-1", problem_id: "P-1" }),
      makeSolution({ id: "S-2", problem_id: "P-1" }),
      makeSolution({ id: "S-3", problem_id: "P-2" }),
    ]);
    cli.listCritiques.resolves([
      makeCritique({ id: "CQ-1", solution_id: "S-1", file_path: "src/db.rs", line_start: 42 }),
      makeCritique({ id: "CQ-2", solution_id: "S-1", file_path: null, line_start: null }),
      makeCritique({ id: "CQ-3", solution_id: "S-3" }),
    ]);
    cli.listMilestones.resolves([makeMilestone({ id: "M-1" })]);
    cli.next.resolves({ items: [], total_count: 0, user: "test" });

    cache = new DataCache(cli as unknown as JjjCli);
    await cache.refresh();
  });

  afterEach(() => {
    cache.dispose();
    sinon.restore();
  });

  describe("getters return readonly arrays", () => {
    it("getProblems returns all problems", () => {
      assert.strictEqual(cache.getProblems().length, 4);
    });

    it("getSolutions returns all solutions", () => {
      assert.strictEqual(cache.getSolutions().length, 3);
    });

    it("getCritiques returns all critiques", () => {
      assert.strictEqual(cache.getCritiques().length, 3);
    });

    it("getMilestones returns all milestones", () => {
      assert.strictEqual(cache.getMilestones().length, 1);
    });
  });

  describe("helper queries", () => {
    it("getProblemsForMilestone returns filtered problems", () => {
      const result = cache.getProblemsForMilestone("M-1");
      assert.strictEqual(result.length, 2);
      assert.deepStrictEqual(result.map(p => p.id), ["P-1", "P-2"]);
    });

    it("getBacklogProblems returns problems without milestone", () => {
      const result = cache.getBacklogProblems();
      assert.strictEqual(result.length, 2);
      assert.deepStrictEqual(result.map(p => p.id), ["P-3", "P-4"]);
    });

    it("getSolutionsForProblem returns filtered solutions", () => {
      const result = cache.getSolutionsForProblem("P-1");
      assert.strictEqual(result.length, 2);
      assert.deepStrictEqual(result.map(s => s.id), ["S-1", "S-2"]);
    });

    it("getCritiquesForSolution returns filtered critiques", () => {
      const result = cache.getCritiquesForSolution("S-1");
      assert.strictEqual(result.length, 2);
    });

    it("getCritiquesWithLocations returns only critiques with file_path and line_start", () => {
      const result = cache.getCritiquesWithLocations();
      assert.strictEqual(result.length, 1);
      assert.strictEqual(result[0].id, "CQ-1");
    });

    it("getProblem returns by id", () => {
      assert.strictEqual(cache.getProblem("P-2")?.id, "P-2");
    });

    it("getProblem returns undefined for unknown id", () => {
      assert.strictEqual(cache.getProblem("P-99"), undefined);
    });

    it("getSolution returns by id", () => {
      assert.strictEqual(cache.getSolution("S-3")?.id, "S-3");
    });

    it("getCritique returns by id", () => {
      assert.strictEqual(cache.getCritique("CQ-2")?.id, "CQ-2");
    });

    it("getMilestone returns by id", () => {
      assert.strictEqual(cache.getMilestone("M-1")?.id, "M-1");
    });
  });

  describe("refresh", () => {
    it("fires onDidChange after refresh", async () => {
      let fired = false;
      cache.onDidChange(() => { fired = true; });
      await cache.refresh();
      assert.strictEqual(fired, true);
    });

    it("skips refresh when already loading", async () => {
      // Call refresh twice concurrently
      const p1 = cache.refresh();
      const p2 = cache.refresh();
      await Promise.all([p1, p2]);
      // Should only have called CLI methods twice total (once in beforeEach, once for p1; p2 skipped)
      assert.strictEqual(cli.listProblems.callCount, 2);
    });

    it("falls back to previous data on error", async () => {
      cli.listProblems.rejects(new Error("network error"));
      await cache.refresh();
      // Should still have the data from beforeEach
      assert.strictEqual(cache.getProblems().length, 4);
    });
  });
});
