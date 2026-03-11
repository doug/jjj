import * as assert from "assert";
import * as sinon from "sinon";
import { DataTransfer, DataTransferItem } from "vscode";
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
      { id: "s2", title: "Accepted", status: "approved", change_ids: [] },
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

// --- Drag and drop helpers ---

function createTestProviderForDragDrop() {
  const mockCli = {
    milestoneRemoveProblem: sinon.stub().resolves("ok"),
    milestoneAddProblem: sinon.stub().resolves("ok"),
  };
  const mockCache = {
    onDidChange: () => ({ dispose: () => {} }),
    getMilestones: () => [{ id: "m1", title: "Sprint 1" }, { id: "m2", title: "Sprint 2" }],
    getProblemsForMilestone: (id: string) => id === "m1"
      ? [{ id: "p1", title: "Problem 1", status: "open", milestone_id: "m1", priority: "medium" }]
      : [],
    getBacklogProblems: () => [{ id: "p2", title: "Backlog Problem", status: "open", milestone_id: null, priority: "medium" }],
    getSolutionsForProblem: () => [],
    getCritiquesForSolution: () => [],
    getProblem: (id: string) => {
      if (id === "p1") { return { id: "p1", title: "Problem 1", status: "open", milestone_id: "m1" }; }
      if (id === "p2") { return { id: "p2", title: "Backlog", status: "open", milestone_id: null }; }
      return undefined;
    },
    refresh: sinon.stub().resolves(),
  };
  return { provider: new ProjectTreeProvider(mockCache as any, mockCli as any), mockCli, mockCache };
}

describe("Drag and drop", () => {
  afterEach(() => {
    sinon.restore();
  });

  it("handleDrag extracts problem IDs from ProblemNodes and sets them on DataTransfer", () => {
    const { provider } = createTestProviderForDragDrop();
    provider.toggleFilter(); // show all
    const milestones = provider.getChildren(undefined);
    const problems = provider.getChildren(milestones[0]); // m1 problems
    const dataTransfer = new DataTransfer();

    provider.handleDrag(problems, dataTransfer);

    const item = dataTransfer.get("application/vnd.jjj.problem");
    assert.ok(item, "DataTransfer should have the problem MIME type set");
    assert.deepStrictEqual(item!.value, ["p1"]);
  });

  it("handleDrag ignores non-ProblemNode sources (SolutionNode, CritiqueNode)", () => {
    const { provider } = createTestProviderForDragDrop();
    // Create a provider that has solutions to drag
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
        { id: "c1", title: "Critique", status: "open", severity: "high" },
      ],
      getProblem: () => undefined,
    } as any;
    const p = new ProjectTreeProvider(mockCache, {} as any);
    const milestones = p.getChildren(undefined);
    const problems = p.getChildren(milestones[0]);
    const solutions = p.getChildren(problems[0]);
    const critiques = p.getChildren(solutions[0]);
    const dataTransfer = new DataTransfer();

    // Drag solutions and critiques (not problems)
    provider.handleDrag([...solutions, ...critiques], dataTransfer);

    const item = dataTransfer.get("application/vnd.jjj.problem");
    assert.strictEqual(item, undefined, "DataTransfer should not be set for non-ProblemNode sources");
  });

  it("handleDrop on MilestoneNode: removes from old milestone, adds to new", async () => {
    const { provider, mockCli, mockCache } = createTestProviderForDragDrop();
    provider.toggleFilter(); // show all
    const milestones = provider.getChildren(undefined);
    const m2Node = milestones[1]; // Sprint 2

    const dataTransfer = new DataTransfer();
    dataTransfer.set("application/vnd.jjj.problem", new DataTransferItem(["p1"]));

    await provider.handleDrop(m2Node, dataTransfer);

    assert.ok(mockCli.milestoneRemoveProblem.calledOnce, "Should remove from old milestone");
    assert.deepStrictEqual(mockCli.milestoneRemoveProblem.firstCall.args, ["m1", "p1"]);
    assert.ok(mockCli.milestoneAddProblem.calledOnce, "Should add to new milestone");
    assert.deepStrictEqual(mockCli.milestoneAddProblem.firstCall.args, ["m2", "p1"]);
  });

  it("handleDrop on Backlog (MilestoneNode with null milestone): removes from old milestone, does not add", async () => {
    const { provider, mockCli, mockCache } = createTestProviderForDragDrop();
    provider.toggleFilter(); // show all
    const milestones = provider.getChildren(undefined);
    // Backlog is the last node (null milestone)
    const backlogNode = milestones[milestones.length - 1];
    assert.strictEqual((backlogNode as any).contextValue, "backlog");

    const dataTransfer = new DataTransfer();
    dataTransfer.set("application/vnd.jjj.problem", new DataTransferItem(["p1"]));

    await provider.handleDrop(backlogNode, dataTransfer);

    assert.ok(mockCli.milestoneRemoveProblem.calledOnce, "Should remove from old milestone");
    assert.deepStrictEqual(mockCli.milestoneRemoveProblem.firstCall.args, ["m1", "p1"]);
    assert.ok(mockCli.milestoneAddProblem.notCalled, "Should NOT add to any milestone for backlog drop");
  });

  it("handleDrop skips if problem is already in target milestone", async () => {
    const { provider, mockCli, mockCache } = createTestProviderForDragDrop();
    provider.toggleFilter(); // show all
    const milestones = provider.getChildren(undefined);
    const m1Node = milestones[0]; // Sprint 1 (p1 is already in m1)

    const dataTransfer = new DataTransfer();
    dataTransfer.set("application/vnd.jjj.problem", new DataTransferItem(["p1"]));

    await provider.handleDrop(m1Node, dataTransfer);

    assert.ok(mockCli.milestoneRemoveProblem.notCalled, "Should not remove (already in target)");
    assert.ok(mockCli.milestoneAddProblem.notCalled, "Should not add (already in target)");
  });

  it("handleDrop ignores drop on non-MilestoneNode", async () => {
    const { provider, mockCli, mockCache } = createTestProviderForDragDrop();
    provider.toggleFilter(); // show all
    const milestones = provider.getChildren(undefined);
    const problems = provider.getChildren(milestones[0]);
    const problemNode = problems[0]; // ProblemNode, not MilestoneNode

    const dataTransfer = new DataTransfer();
    dataTransfer.set("application/vnd.jjj.problem", new DataTransferItem(["p1"]));

    await provider.handleDrop(problemNode, dataTransfer);

    assert.ok(mockCli.milestoneRemoveProblem.notCalled, "Should not call remove on non-MilestoneNode target");
    assert.ok(mockCli.milestoneAddProblem.notCalled, "Should not call add on non-MilestoneNode target");
  });

  it("handleDrop calls cache.refresh() after successful move", async () => {
    const { provider, mockCli, mockCache } = createTestProviderForDragDrop();
    provider.toggleFilter(); // show all
    const milestones = provider.getChildren(undefined);
    const m2Node = milestones[1]; // Sprint 2

    const dataTransfer = new DataTransfer();
    dataTransfer.set("application/vnd.jjj.problem", new DataTransferItem(["p1"]));

    await provider.handleDrop(m2Node, dataTransfer);

    assert.ok(mockCache.refresh.calledOnce, "Should call cache.refresh() after drop");
  });
});

// --- Tree structure helpers ---

function createTestProviderForTreeStructure() {
  const mockCache = {
    onDidChange: () => ({ dispose: () => {} }),
    getMilestones: () => [
      { id: "m1", title: "Sprint 1", assignee: "Alice <alice@test.com>" },
    ],
    getProblemsForMilestone: () => [
      { id: "p1", title: "Open Problem", status: "open", priority: "medium" },
      { id: "p2", title: "In Progress", status: "in_progress", priority: "high", assignee: "Bob <bob@test.com>", tags: ["auth", "urgent"] },
      { id: "p3", title: "Solved Problem", status: "solved", priority: "medium" },
    ],
    getBacklogProblems: () => [
      { id: "p4", title: "Backlog Item", status: "open", priority: "low" },
    ],
    getSolutionsForProblem: (id: string) => {
      if (id === "p1") {
        return [
          { id: "s1", title: "Proposed Sol", status: "proposed", change_ids: [], tags: ["wip"] },
          { id: "s2", title: "Submitted Sol", status: "submitted", change_ids: ["abc123"] },
          { id: "s3", title: "Approved Sol", status: "approved", change_ids: [] },
          { id: "s4", title: "Withdrawn Sol", status: "withdrawn", change_ids: [] },
        ];
      }
      return [];
    },
    getCritiquesForSolution: (id: string) => {
      if (id === "s1") {
        return [
          { id: "c1", title: "Open Critique", status: "open", severity: "high", file_path: "src/main.rs", line_start: 42 },
          { id: "c2", title: "Addressed Critique", status: "addressed", severity: "low", file_path: null, line_start: null },
          { id: "c3", title: "Valid Critique", status: "valid", severity: "critical", file_path: "src/lib.rs", line_start: 10 },
          { id: "c4", title: "Dismissed Critique", status: "dismissed", severity: "medium", file_path: null, line_start: null },
        ];
      }
      return [];
    },
    getProblem: () => undefined,
  } as any;
  return new ProjectTreeProvider(mockCache, {} as any);
}

describe("Tree structure", () => {
  it("root returns MilestoneNodes and a BacklogNode", () => {
    const provider = createTestProviderForTreeStructure();
    provider.toggleFilter(); // show all
    const roots = provider.getChildren(undefined);
    assert.strictEqual(roots.length, 2);
    assert.strictEqual((roots[0] as any).contextValue, "milestone");
    assert.strictEqual((roots[1] as any).contextValue, "backlog");
  });

  it("MilestoneNode shows milestone icon, BacklogNode shows inbox icon", () => {
    const provider = createTestProviderForTreeStructure();
    provider.toggleFilter(); // show all
    const roots = provider.getChildren(undefined);
    assert.strictEqual((roots[0] as any).iconPath.id, "milestone");
    assert.strictEqual((roots[1] as any).iconPath.id, "inbox");
  });

  it("MilestoneNode shows progress description", () => {
    const provider = createTestProviderForTreeStructure();
    provider.toggleFilter(); // show all
    const roots = provider.getChildren(undefined);
    const milestoneNode = roots[0] as any;
    // 3 problems total, 1 solved => "1/3 . 33%"
    assert.ok(milestoneNode.description.includes("1/3"), `Expected progress in description, got: ${milestoneNode.description}`);
    assert.ok(milestoneNode.description.includes("33%"), `Expected percentage in description, got: ${milestoneNode.description}`);
  });

  it("BacklogNode shows problem count in description", () => {
    const provider = createTestProviderForTreeStructure();
    provider.toggleFilter(); // show all
    const roots = provider.getChildren(undefined);
    const backlogNode = roots[1] as any;
    assert.ok(backlogNode.description.includes("1 problems"), `Expected problem count, got: ${backlogNode.description}`);
  });

  it("ProblemNode has correct contextValue 'problem'", () => {
    const provider = createTestProviderForTreeStructure();
    provider.toggleFilter(); // show all
    const roots = provider.getChildren(undefined);
    const problems = provider.getChildren(roots[0]);
    assert.strictEqual((problems[0] as any).contextValue, "problem");
    assert.strictEqual((problems[1] as any).contextValue, "problem");
    assert.strictEqual((problems[2] as any).contextValue, "problem");
  });

  it("ProblemNode shows status icon: solved=check, in_progress=debug-start, open=circle-outline", () => {
    const provider = createTestProviderForTreeStructure();
    provider.toggleFilter(); // show all
    const roots = provider.getChildren(undefined);
    const problems = provider.getChildren(roots[0]);
    // p1 is open
    assert.strictEqual((problems[0] as any).iconPath.id, "circle-outline");
    // p2 is in_progress
    assert.strictEqual((problems[1] as any).iconPath.id, "debug-start");
    // p3 is solved
    assert.strictEqual((problems[2] as any).iconPath.id, "check");
  });

  it("ProblemNode description includes id, status, priority (when non-medium), assignee, tags", () => {
    const provider = createTestProviderForTreeStructure();
    provider.toggleFilter(); // show all
    const roots = provider.getChildren(undefined);
    const problems = provider.getChildren(roots[0]);

    // p1: open, medium priority, no assignee, no tags
    const desc1 = (problems[0] as any).description as string;
    assert.ok(desc1.includes("p1"), `Expected id in description: ${desc1}`);
    assert.ok(desc1.includes("[open]"), `Expected status in description: ${desc1}`);
    // medium priority is excluded
    assert.ok(!desc1.includes("medium"), `Medium priority should not appear: ${desc1}`);

    // p2: in_progress, high priority, has assignee and tags
    const desc2 = (problems[1] as any).description as string;
    assert.ok(desc2.includes("p2"), `Expected id in description: ${desc2}`);
    assert.ok(desc2.includes("[in_progress]"), `Expected status in description: ${desc2}`);
    assert.ok(desc2.includes("high"), `Expected priority in description: ${desc2}`);
    assert.ok(desc2.includes("@Bob"), `Expected assignee in description: ${desc2}`);
    assert.ok(desc2.includes("#auth"), `Expected tag in description: ${desc2}`);
    assert.ok(desc2.includes("#urgent"), `Expected tag in description: ${desc2}`);
  });

  it("SolutionNode contextValue matches solution-${status}", () => {
    const provider = createTestProviderForTreeStructure();
    provider.toggleFilter(); // show all
    const roots = provider.getChildren(undefined);
    const problems = provider.getChildren(roots[0]);
    const solutions = provider.getChildren(problems[0]); // p1 has 4 solutions

    assert.strictEqual((solutions[0] as any).contextValue, "solution-proposed");
    assert.strictEqual((solutions[1] as any).contextValue, "solution-submitted");
    assert.strictEqual((solutions[2] as any).contextValue, "solution-approved");
    assert.strictEqual((solutions[3] as any).contextValue, "solution-withdrawn");
  });

  it("SolutionNode shows correct icon per status", () => {
    const provider = createTestProviderForTreeStructure();
    provider.toggleFilter(); // show all
    const roots = provider.getChildren(undefined);
    const problems = provider.getChildren(roots[0]);
    const solutions = provider.getChildren(problems[0]);

    assert.strictEqual((solutions[0] as any).iconPath.id, "lightbulb"); // proposed
    assert.strictEqual((solutions[1] as any).iconPath.id, "beaker"); // submitted
    assert.strictEqual((solutions[2] as any).iconPath.id, "check"); // approved
    assert.strictEqual((solutions[3] as any).iconPath.id, "x"); // withdrawn
  });

  it("CritiqueNode contextValue matches critique-${status}", () => {
    const provider = createTestProviderForTreeStructure();
    provider.toggleFilter(); // show all
    const roots = provider.getChildren(undefined);
    const problems = provider.getChildren(roots[0]);
    const solutions = provider.getChildren(problems[0]);
    const critiques = provider.getChildren(solutions[0]); // s1 has 4 critiques

    assert.strictEqual((critiques[0] as any).contextValue, "critique-open");
    assert.strictEqual((critiques[1] as any).contextValue, "critique-addressed");
    assert.strictEqual((critiques[2] as any).contextValue, "critique-valid");
    assert.strictEqual((critiques[3] as any).contextValue, "critique-dismissed");
  });

  it("CritiqueNode description includes severity and location", () => {
    const provider = createTestProviderForTreeStructure();
    provider.toggleFilter(); // show all
    const roots = provider.getChildren(undefined);
    const problems = provider.getChildren(roots[0]);
    const solutions = provider.getChildren(problems[0]);
    const critiques = provider.getChildren(solutions[0]);

    // c1: severity=high, file_path=src/main.rs:42
    const desc1 = (critiques[0] as any).description as string;
    assert.ok(desc1.includes("[high]"), `Expected severity in description: ${desc1}`);
    assert.ok(desc1.includes("src/main.rs:42"), `Expected file location in description: ${desc1}`);

    // c2: severity=low, no file_path
    const desc2 = (critiques[1] as any).description as string;
    assert.ok(desc2.includes("[low]"), `Expected severity in description: ${desc2}`);
    assert.ok(!desc2.includes("---"), `Should not have location when no file_path: ${desc2}`);

    // c3: severity=critical, file_path=src/lib.rs:10
    const desc3 = (critiques[2] as any).description as string;
    assert.ok(desc3.includes("[critical]"), `Expected severity in description: ${desc3}`);
    assert.ok(desc3.includes("src/lib.rs:10"), `Expected file location in description: ${desc3}`);
  });
});

describe("getAllOpenItems", () => {
  it("returns items in depth-first order: problems, then their solutions, then critiques", () => {
    const mockCache = {
      onDidChange: () => ({ dispose: () => {} }),
      getMilestones: () => [{ id: "m1", title: "Sprint" }],
      getProblemsForMilestone: () => [
        { id: "p1", title: "Problem 1", status: "open", priority: "medium" },
      ],
      getBacklogProblems: () => [],
      getSolutionsForProblem: (id: string) => id === "p1"
        ? [{ id: "s1", title: "Solution 1", status: "proposed", change_ids: [] }]
        : [],
      getCritiquesForSolution: (id: string) => id === "s1"
        ? [{ id: "c1", title: "Critique 1", status: "open", severity: "high" }]
        : [],
      getProblem: () => undefined,
    } as any;
    const provider = new ProjectTreeProvider(mockCache, {} as any);
    const items = provider.getAllOpenItems();

    assert.strictEqual(items.length, 3);
    assert.strictEqual((items[0] as any).problem?.id, "p1");
    assert.strictEqual((items[1] as any).solution?.id, "s1");
    assert.strictEqual((items[2] as any).critique?.id, "c1");
  });

  it("includes backlog problems", () => {
    const mockCache = {
      onDidChange: () => ({ dispose: () => {} }),
      getMilestones: () => [],
      getProblemsForMilestone: () => [],
      getBacklogProblems: () => [
        { id: "p1", title: "Backlog Problem", status: "open", priority: "medium" },
      ],
      getSolutionsForProblem: () => [],
      getCritiquesForSolution: () => [],
      getProblem: () => undefined,
    } as any;
    const provider = new ProjectTreeProvider(mockCache, {} as any);
    const items = provider.getAllOpenItems();

    assert.strictEqual(items.length, 1);
    assert.strictEqual((items[0] as any).problem?.id, "p1");
  });

  it("skips solved problems, approved solutions, addressed critiques", () => {
    const mockCache = {
      onDidChange: () => ({ dispose: () => {} }),
      getMilestones: () => [{ id: "m1", title: "Sprint" }],
      getProblemsForMilestone: () => [
        { id: "p1", title: "Open", status: "open", priority: "medium" },
        { id: "p2", title: "Solved", status: "solved", priority: "medium" },
        { id: "p3", title: "Dissolved", status: "dissolved", priority: "medium" },
      ],
      getBacklogProblems: () => [],
      getSolutionsForProblem: (id: string) => {
        if (id === "p1") {
          return [
            { id: "s1", title: "Proposed", status: "proposed", change_ids: [] },
            { id: "s2", title: "Approved", status: "approved", change_ids: [] },
            { id: "s3", title: "Withdrawn", status: "withdrawn", change_ids: [] },
          ];
        }
        // Even solved problems have their solutions iterated
        if (id === "p2") {
          return [
            { id: "s4", title: "Also Proposed", status: "proposed", change_ids: [] },
          ];
        }
        return [];
      },
      getCritiquesForSolution: (id: string) => {
        if (id === "s1") {
          return [
            { id: "c1", title: "Open", status: "open", severity: "high" },
            { id: "c2", title: "Addressed", status: "addressed", severity: "low" },
            { id: "c3", title: "Dismissed", status: "dismissed", severity: "medium" },
            { id: "c4", title: "Valid", status: "valid", severity: "critical" },
          ];
        }
        return [];
      },
      getProblem: () => undefined,
    } as any;
    const provider = new ProjectTreeProvider(mockCache, {} as any);
    const items = provider.getAllOpenItems();

    // Should include: p1 (open), s1 (proposed), c1 (open), c4 (valid), s4 (proposed from p2 - because getAllOpenItems iterates all problems' solutions)
    const ids = items.map((item: any) => item.problem?.id || item.solution?.id || item.critique?.id);
    assert.ok(ids.includes("p1"), "Should include open problem p1");
    assert.ok(!ids.includes("p2"), "Should skip solved problem p2");
    assert.ok(!ids.includes("p3"), "Should skip dissolved problem p3");
    assert.ok(ids.includes("s1"), "Should include proposed solution s1");
    assert.ok(!ids.includes("s2"), "Should skip approved solution s2");
    assert.ok(!ids.includes("s3"), "Should skip withdrawn solution s3");
    assert.ok(ids.includes("c1"), "Should include open critique c1");
    assert.ok(!ids.includes("c2"), "Should skip addressed critique c2");
    assert.ok(!ids.includes("c3"), "Should skip dismissed critique c3");
    assert.ok(ids.includes("c4"), "Should include valid critique c4");
    // s4 is proposed under p2 - getAllOpenItems still iterates solutions under solved problems
    assert.ok(ids.includes("s4"), "Should include proposed solution s4 even under solved problem");
  });
});
