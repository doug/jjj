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
    // Both the milestone (only solved problems) and the empty backlog are hidden in open mode
    assert.strictEqual(milestones.length, 0);
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
      { id: "s1", title: "Proposed", status: "proposed", change_ids: [] },
      { id: "s2", title: "Accepted", status: "accepted", change_ids: [] },
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
      { id: "s1", title: "Solution", status: "proposed", change_ids: [] },
    ],
    getCritiquesForSolution: () => [
      { id: "c1", title: "Open Critique", status: "open", severity: "high" },
      { id: "c2", title: "Addressed", status: "addressed", severity: "low" },
    ],
    getProblem: () => undefined,
  } as any;
  return new ProjectTreeProvider(mockCache, {} as any);
}

describe("Tab navigation", () => {
  it("getNextOpenItem returns first open item when no selection", () => {
    const provider = createTestProviderWithNavData();
    const next = provider.getNextOpenItem(undefined);
    assert.ok(next);
    assert.strictEqual((next as any).problem?.id, "p1");
  });

  it("getNextOpenItem skips closed items", () => {
    const provider = createTestProviderWithNavData();
    provider.toggleFilter(); // show all
    const milestones = provider.getChildren(undefined);
    const problems = provider.getChildren(milestones[0]);
    // Start from first problem (open), should go to solution (skipping solved problem)
    const next = provider.getNextOpenItem(problems[0]);
    assert.ok(next);
    // Should be the solution under p1, not the solved p2
    assert.strictEqual((next as any).solution?.id, "s1");
  });

  it("getNextOpenItem wraps around to beginning", () => {
    const provider = createTestProviderWithNavData();
    // Get to last item and call next
    const allItems = provider.getAllOpenItems();
    const last = allItems[allItems.length - 1];
    const next = provider.getNextOpenItem(last);
    // Compare by id since getAllOpenItems creates new objects
    const firstItem = allItems[0];
    assert.strictEqual((next as any).problem?.id, (firstItem as any).problem?.id);
  });

  it("getPrevOpenItem returns last open item when no selection", () => {
    const provider = createTestProviderWithNavData();
    const prev = provider.getPrevOpenItem(undefined);
    assert.ok(prev);
  });

  it("getPrevOpenItem wraps around to end", () => {
    const provider = createTestProviderWithNavData();
    const allItems = provider.getAllOpenItems();
    const first = allItems[0];
    const prev = provider.getPrevOpenItem(first);
    // Compare by id since getAllOpenItems creates new objects
    const lastItem = allItems[allItems.length - 1];
    assert.strictEqual((prev as any).critique?.id, (lastItem as any).critique?.id);
  });

  it("returns undefined when no open items", () => {
    const provider = createTestProviderNoOpenItems();
    const next = provider.getNextOpenItem(undefined);
    assert.strictEqual(next, undefined);
  });
});

function createTestProviderWithNavData(): ProjectTreeProvider {
  const mockCache = {
    onDidChange: () => ({ dispose: () => {} }),
    getMilestones: () => [{ id: "m1", title: "Sprint" }],
    getProblemsForMilestone: () => [
      { id: "p1", title: "Open", status: "open", priority: "medium" },
      { id: "p2", title: "Solved", status: "solved", priority: "medium" },
    ],
    getBacklogProblems: () => [],
    getSolutionsForProblem: (id: string) => id === "p1"
      ? [{ id: "s1", title: "Solution", status: "proposed", change_ids: [] }]
      : [],
    getCritiquesForSolution: () => [
      { id: "c1", title: "Critique", status: "open", severity: "high" },
    ],
    getProblem: () => undefined,
  } as any;
  return new ProjectTreeProvider(mockCache, {} as any);
}

function createTestProviderNoOpenItems(): ProjectTreeProvider {
  const mockCache = {
    onDidChange: () => ({ dispose: () => {} }),
    getMilestones: () => [{ id: "m1", title: "Done" }],
    getProblemsForMilestone: () => [
      { id: "p1", title: "Solved", status: "solved", priority: "medium" },
    ],
    getBacklogProblems: () => [],
    getSolutionsForProblem: () => [],
    getCritiquesForSolution: () => [],
    getProblem: () => undefined,
  } as any;
  return new ProjectTreeProvider(mockCache, {} as any);
}
