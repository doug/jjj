import * as assert from "assert";
import { ProjectTreeProvider } from "../../src/views/projectTreeProvider";

describe("ProjectTreeProvider", () => {
  describe("filter mode", () => {
    it("defaults to open mode", () => {
      const provider = createTestProvider();
      assert.strictEqual(provider.filterMode, "open");
    });

    it("toggleFilter switches between modes", () => {
      const provider = createTestProvider();
      assert.strictEqual(provider.filterMode, "open");
      provider.toggleFilter();
      assert.strictEqual(provider.filterMode, "all");
      provider.toggleFilter();
      assert.strictEqual(provider.filterMode, "open");
    });

    it("fires onDidChangeTreeData when filter toggles", () => {
      const provider = createTestProvider();
      let fired = false;
      provider.onDidChangeTreeData(() => { fired = true; });
      provider.toggleFilter();
      assert.strictEqual(fired, true);
    });
  });
});

describe("filtering", () => {
  it("shows all problems in all mode", () => {
    const provider = createTestProviderWithData();
    provider.toggleFilter(); // switch to "all"
    const milestones = provider.getChildren(undefined);
    const problems = provider.getChildren(milestones[0]);
    assert.strictEqual(problems.length, 2); // open + solved
  });

  it("hides solved problems in open mode", () => {
    const provider = createTestProviderWithData();
    // default is "open"
    const milestones = provider.getChildren(undefined);
    const problems = provider.getChildren(milestones[0]);
    assert.strictEqual(problems.length, 1); // only open
    assert.strictEqual((problems[0] as any).problem.status, "open");
  });

  it("hides milestones with no open items in open mode", () => {
    const provider = createTestProviderWithEmptyMilestone();
    const milestones = provider.getChildren(undefined);
    // Should only show backlog, not the empty milestone
    assert.strictEqual(milestones.length, 1);
  });

  it("hides accepted solutions in open mode", () => {
    const provider = createTestProviderWithSolutions();
    const milestones = provider.getChildren(undefined);
    const problems = provider.getChildren(milestones[0]);
    const solutions = provider.getChildren(problems[0]);
    assert.strictEqual(solutions.length, 1); // only proposed, not accepted
  });

  it("hides addressed critiques in open mode", () => {
    const provider = createTestProviderWithCritiques();
    const milestones = provider.getChildren(undefined);
    const problems = provider.getChildren(milestones[0]);
    const solutions = provider.getChildren(problems[0]);
    const critiques = provider.getChildren(solutions[0]);
    assert.strictEqual(critiques.length, 1); // only open, not addressed
  });
});

function createTestProvider(): ProjectTreeProvider {
  const mockCache = {
    onDidChange: () => ({ dispose: () => {} }),
    getMilestones: () => [],
    getProblemsForMilestone: () => [],
    getBacklogProblems: () => [],
    getSolutionsForProblem: () => [],
    getCritiquesForSolution: () => [],
    getProblem: () => undefined,
  } as any;
  const mockCli = {} as any;
  return new ProjectTreeProvider(mockCache, mockCli);
}

function createTestProviderWithData(): ProjectTreeProvider {
  const mockCache = {
    onDidChange: () => ({ dispose: () => {} }),
    getMilestones: () => [{ id: "m1", title: "Sprint 1" }],
    getProblemsForMilestone: () => [
      { id: "p1", title: "Open Problem", status: "open" },
      { id: "p2", title: "Solved Problem", status: "solved" },
    ],
    getBacklogProblems: () => [],
    getSolutionsForProblem: () => [],
    getCritiquesForSolution: () => [],
    getProblem: () => undefined,
  } as any;
  return new ProjectTreeProvider(mockCache, {} as any);
}

function createTestProviderWithEmptyMilestone(): ProjectTreeProvider {
  const mockCache = {
    onDidChange: () => ({ dispose: () => {} }),
    getMilestones: () => [{ id: "m1", title: "Empty Sprint" }],
    getProblemsForMilestone: () => [
      { id: "p1", title: "Solved", status: "solved" },
    ],
    getBacklogProblems: () => [],
    getSolutionsForProblem: () => [],
    getCritiquesForSolution: () => [],
    getProblem: () => undefined,
  } as any;
  return new ProjectTreeProvider(mockCache, {} as any);
}

function createTestProviderWithSolutions(): ProjectTreeProvider {
  const mockCache = {
    onDidChange: () => ({ dispose: () => {} }),
    getMilestones: () => [{ id: "m1", title: "Sprint" }],
    getProblemsForMilestone: () => [
      { id: "p1", title: "Problem", status: "open", priority: "medium" },
    ],
    getBacklogProblems: () => [],
    getSolutionsForProblem: () => [
      { id: "s1", title: "Proposed", status: "proposed" },
      { id: "s2", title: "Accepted", status: "accepted" },
    ],
    getCritiquesForSolution: () => [],
    getProblem: () => undefined,
  } as any;
  return new ProjectTreeProvider(mockCache, {} as any);
}

function createTestProviderWithCritiques(): ProjectTreeProvider {
  const mockCache = {
    onDidChange: () => ({ dispose: () => {} }),
    getMilestones: () => [{ id: "m1", title: "Sprint" }],
    getProblemsForMilestone: () => [
      { id: "p1", title: "Problem", status: "open", priority: "medium" },
    ],
    getBacklogProblems: () => [],
    getSolutionsForProblem: () => [
      { id: "s1", title: "Solution", status: "proposed" },
    ],
    getCritiquesForSolution: () => [
      { id: "c1", title: "Open Critique", status: "open", severity: "high" },
      { id: "c2", title: "Addressed", status: "addressed", severity: "low" },
    ],
    getProblem: () => undefined,
  } as any;
  return new ProjectTreeProvider(mockCache, {} as any);
}
