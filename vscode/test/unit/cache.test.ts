import * as assert from "assert";
import * as sinon from "sinon";
import { DataCache } from "../../src/cache";
import { JjjCli, Problem, Solution, Critique, Milestone } from "../../src/cli";

function makeProblem(overrides: Partial<Problem> = {}): Problem {
  return {
    id: "p1", title: "Test problem", parent_id: null, status: "open",
    solution_ids: [], child_ids: [], milestone_id: null,
    assignee: null, created_at: "", updated_at: "", description: "", context: "",
    priority: "medium", dissolved_reason: null,
    ...overrides,
  };
}

function makeSolution(overrides: Partial<Solution> = {}): Solution {
  return {
    id: "s1", title: "Test solution", problem_id: "p1", status: "proposed",
    critique_ids: [], change_ids: [], assignee: null,
    reviewers: [], sign_offs: [], force_accepted: false,
    created_at: "", updated_at: "", approach: "", tradeoffs: "",
    supersedes: null,
    ...overrides,
  };
}

function makeCritique(overrides: Partial<Critique> = {}): Critique {
  return {
    id: "c1", title: "Test critique", solution_id: "s1", status: "open",
    severity: "medium", author: null, created_at: "", updated_at: "",
    argument: "", evidence: "", file_path: null, line_start: null,
    line_end: null, code_context: [], replies: [],
    ...overrides,
  };
}

function makeMilestone(overrides: Partial<Milestone> = {}): Milestone {
  return {
    id: "m1", title: "Test milestone", target_date: null, status: "active",
    problem_ids: [], assignee: null, created_at: "", updated_at: "",
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
      makeProblem({ id: "p1", milestone_id: "m1" }),
      makeProblem({ id: "p2", milestone_id: "m1" }),
      makeProblem({ id: "p3", milestone_id: null }),
      makeProblem({ id: "p4", milestone_id: null }),
    ]);
    cli.listSolutions.resolves([
      makeSolution({ id: "s1", problem_id: "p1" }),
      makeSolution({ id: "s2", problem_id: "p1" }),
      makeSolution({ id: "s3", problem_id: "p2" }),
    ]);
    cli.listCritiques.resolves([
      makeCritique({ id: "c1", solution_id: "s1", file_path: "src/db.rs", line_start: 42 }),
      makeCritique({ id: "c2", solution_id: "s1", file_path: null, line_start: null }),
      makeCritique({ id: "c3", solution_id: "s3" }),
    ]);
    cli.listMilestones.resolves([makeMilestone({ id: "m1" })]);
    cli.status.resolves({ active_solution: null, items: [], total_count: 0, user: "test", summary: { open_problems: 0, testing_solutions: 0, open_critiques: 0 } });

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
      const result = cache.getProblemsForMilestone("m1");
      assert.strictEqual(result.length, 2);
      assert.deepStrictEqual(result.map(p => p.id), ["p1", "p2"]);
    });

    it("getBacklogProblems returns problems without milestone", () => {
      const result = cache.getBacklogProblems();
      assert.strictEqual(result.length, 2);
      assert.deepStrictEqual(result.map(p => p.id), ["p3", "p4"]);
    });

    it("getSolutionsForProblem returns filtered solutions", () => {
      const result = cache.getSolutionsForProblem("p1");
      assert.strictEqual(result.length, 2);
      assert.deepStrictEqual(result.map(s => s.id), ["s1", "s2"]);
    });

    it("getCritiquesForSolution returns filtered critiques", () => {
      const result = cache.getCritiquesForSolution("s1");
      assert.strictEqual(result.length, 2);
    });

    it("getCritiquesWithLocations returns only critiques with file_path and line_start", () => {
      const result = cache.getCritiquesWithLocations();
      assert.strictEqual(result.length, 1);
      assert.strictEqual(result[0].id, "c1");
    });

    it("getProblem returns by id", () => {
      assert.strictEqual(cache.getProblem("p2")?.id, "p2");
    });

    it("getProblem returns undefined for unknown id", () => {
      assert.strictEqual(cache.getProblem("p99"), undefined);
    });

    it("getSolution returns by id", () => {
      assert.strictEqual(cache.getSolution("s3")?.id, "s3");
    });

    it("getCritique returns by id", () => {
      assert.strictEqual(cache.getCritique("c2")?.id, "c2");
    });

    it("getMilestone returns by id", () => {
      assert.strictEqual(cache.getMilestone("m1")?.id, "m1");
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
