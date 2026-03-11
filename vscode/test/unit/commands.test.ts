import * as assert from "assert";
import * as sinon from "sinon";
import * as vscode from "vscode";
import { registerCommands } from "../../src/commands";
import { Problem, Solution, Critique, Milestone, JjjCli, StatusResult } from "../../src/cli";

// --- Fixtures ---

function makeProblem(overrides: Partial<Problem> = {}): Problem {
  return {
    id: "p1", title: "Test problem", parent_id: null, status: "open",
    solution_ids: [], child_ids: [], milestone_id: null,
    assignee: null, created_at: "", updated_at: "", description: "", context: "",
    priority: "medium", dissolved_reason: null, tags: [],
    ...overrides,
  };
}

function makeSolution(overrides: Partial<Solution> = {}): Solution {
  return {
    id: "s1", title: "Test solution", problem_id: "p1", status: "proposed",
    critique_ids: [], change_ids: [], assignee: null,
    force_approved: false,
    created_at: "", updated_at: "", approach: "", tradeoffs: "",
    supersedes: null, tags: [],
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

function makeStatusResult(overrides: Partial<StatusResult> = {}): StatusResult {
  return {
    active_solution: null,
    items: [],
    total_count: 0,
    user: "testuser",
    summary: { open_problems: 0, review_solutions: 0, open_critiques: 0 },
    ...overrides,
  };
}

// --- Mock helpers ---

interface MockCache {
  refresh: sinon.SinonStub;
  getProblems: sinon.SinonStub;
  getSolutions: sinon.SinonStub;
  getCritiques: sinon.SinonStub;
  getMilestones: sinon.SinonStub;
  getStatus: sinon.SinonStub;
}

function makeMockCache(): MockCache {
  return {
    refresh: sinon.stub().resolves(),
    getProblems: sinon.stub().returns([]),
    getSolutions: sinon.stub().returns([]),
    getCritiques: sinon.stub().returns([]),
    getMilestones: sinon.stub().returns([]),
    getStatus: sinon.stub().returns(null),
  };
}

// --- Tests ---

describe("registerCommands", () => {
  let cli: sinon.SinonStubbedInstance<JjjCli>;
  let cache: MockCache;
  let commandHandlers: Map<string, (...args: unknown[]) => Promise<void>>;
  let registerCommandStub: sinon.SinonStub;
  let showInputBoxStub: sinon.SinonStub;
  let showQuickPickStub: sinon.SinonStub;
  let showInfoStub: sinon.SinonStub;
  let showErrorStub: sinon.SinonStub;
  let showWarningStub: sinon.SinonStub;

  beforeEach(() => {
    cli = sinon.createStubInstance(JjjCli);
    cache = makeMockCache();

    commandHandlers = new Map<string, (...args: unknown[]) => Promise<void>>();
    registerCommandStub = sinon.stub(vscode.commands, "registerCommand")
      .callsFake((name: string, handler: (...args: unknown[]) => unknown) => {
        commandHandlers.set(name, handler as (...args: unknown[]) => Promise<void>);
        return { dispose: () => {} };
      });

    showInputBoxStub = sinon.stub(vscode.window, "showInputBox");
    showQuickPickStub = sinon.stub(vscode.window, "showQuickPick");
    showInfoStub = sinon.stub(vscode.window, "showInformationMessage");
    showErrorStub = sinon.stub(vscode.window, "showErrorMessage");
    // showWarningMessage isn't in the base mock, so we assign it directly
    (vscode.window as Record<string, unknown>).showWarningMessage = sinon.stub().resolves(undefined);
    showWarningStub = vscode.window.showWarningMessage as unknown as sinon.SinonStub;

    const context = {
      subscriptions: [] as { dispose: () => void }[],
    } as unknown as vscode.ExtensionContext;

    registerCommands(context, cli as unknown as JjjCli, cache as unknown as never);
  });

  afterEach(() => {
    sinon.restore();
  });

  // Helper to invoke a registered command handler
  async function invoke(name: string, ...args: unknown[]): Promise<void> {
    const handler = commandHandlers.get(name);
    assert.ok(handler, `Command "${name}" was not registered`);
    await handler(...args);
  }

  // =========================================================================
  // register() wrapper behavior
  // =========================================================================

  describe("register() wrapper", () => {
    it("calls cache.refresh() after a successful handler", async () => {
      showInputBoxStub.resolves("My new problem");
      cli.newProblem.resolves("Created problem");

      await invoke("jjj.newProblem");

      sinon.assert.calledOnce(cache.refresh);
    });

    it("shows error message with JJJ prefix when handler throws", async () => {
      showInputBoxStub.resolves("My new problem");
      cli.newProblem.rejects(new Error("disk full"));

      await invoke("jjj.newProblem");

      sinon.assert.calledOnce(showErrorStub);
      const errorArg = showErrorStub.firstCall.args[0] as string;
      assert.ok(errorArg.startsWith("JJJ:"), `Expected error to start with "JJJ:", got: "${errorArg}"`);
      assert.ok(errorArg.includes("disk full"), `Expected error to include "disk full", got: "${errorArg}"`);
    });

    it("does NOT call cache.refresh() when handler throws", async () => {
      showInputBoxStub.resolves("My new problem");
      cli.newProblem.rejects(new Error("fail"));

      await invoke("jjj.newProblem");

      sinon.assert.notCalled(cache.refresh);
    });

    it("converts non-Error throws to string in error message", async () => {
      showInputBoxStub.resolves("My new problem");
      // Simulate a non-Error rejection by using callsFake to throw a string
      cli.newProblem.callsFake(async () => { throw "raw string error"; });

      await invoke("jjj.newProblem");

      sinon.assert.calledOnce(showErrorStub);
      const errorArg = showErrorStub.firstCall.args[0] as string;
      assert.ok(errorArg.includes("raw string error"));
    });
  });

  // =========================================================================
  // Problem commands
  // =========================================================================

  describe("jjj.newProblem", () => {
    it("creates a problem when title is provided", async () => {
      showInputBoxStub.resolves("Auth bug");
      cli.newProblem.resolves("Created p123");

      await invoke("jjj.newProblem");

      sinon.assert.calledOnceWithExactly(cli.newProblem, "Auth bug");
      sinon.assert.calledOnce(showInfoStub);
      assert.strictEqual(showInfoStub.firstCall.args[0], "Created p123");
    });

    it("does nothing when input box is cancelled", async () => {
      showInputBoxStub.resolves(undefined);

      await invoke("jjj.newProblem");

      sinon.assert.notCalled(cli.newProblem);
      sinon.assert.notCalled(showInfoStub);
    });

    it("does nothing when input box returns empty string", async () => {
      showInputBoxStub.resolves("");

      await invoke("jjj.newProblem");

      sinon.assert.notCalled(cli.newProblem);
    });
  });

  describe("jjj.solveProblem", () => {
    it("filters to open/in_progress problems and calls cli.solveProblem", async () => {
      const openProblem = makeProblem({ id: "p1", title: "Bug", status: "open" });
      const inProgressProblem = makeProblem({ id: "p2", title: "Feature", status: "in_progress" });
      const solvedProblem = makeProblem({ id: "p3", title: "Done", status: "solved" });
      cache.getProblems.returns([openProblem, inProgressProblem, solvedProblem]);

      showQuickPickStub.resolves({ label: "p1: Bug", id: "p1" });
      cli.solveProblem.resolves("Problem solved");

      await invoke("jjj.solveProblem");

      // Verify only open/in_progress problems were shown
      const items = showQuickPickStub.firstCall.args[0] as { label: string; id: string }[];
      assert.strictEqual(items.length, 2);
      assert.deepStrictEqual(items.map(i => i.id), ["p1", "p2"]);

      sinon.assert.calledOnceWithExactly(cli.solveProblem, "p1");
    });

    it("does nothing when quick pick is cancelled", async () => {
      cache.getProblems.returns([makeProblem()]);
      showQuickPickStub.resolves(undefined);

      await invoke("jjj.solveProblem");

      sinon.assert.notCalled(cli.solveProblem);
    });
  });

  describe("jjj.dissolveProblem", () => {
    it("filters to open/in_progress problems and calls cli.dissolveProblem", async () => {
      cache.getProblems.returns([
        makeProblem({ id: "p1", status: "open" }),
        makeProblem({ id: "p2", status: "dissolved" }),
      ]);
      showQuickPickStub.resolves({ label: "p1: Test problem", id: "p1" });
      cli.dissolveProblem.resolves("Dissolved");

      await invoke("jjj.dissolveProblem");

      const items = showQuickPickStub.firstCall.args[0] as { id: string }[];
      assert.strictEqual(items.length, 1);
      sinon.assert.calledOnceWithExactly(cli.dissolveProblem, "p1");
    });
  });

  // =========================================================================
  // Solution commands
  // =========================================================================

  describe("jjj.newSolution", () => {
    it("shows problem pick then title input and creates solution", async () => {
      cache.getProblems.returns([makeProblem({ id: "p1", title: "Bug", status: "open" })]);
      showQuickPickStub.resolves({ label: "p1: Bug", id: "p1" });
      showInputBoxStub.resolves("Fix the bug");
      cli.newSolution.resolves("Created s1");

      await invoke("jjj.newSolution");

      sinon.assert.calledOnceWithExactly(cli.newSolution, "Fix the bug", "p1");
      sinon.assert.calledOnce(showInfoStub);
    });

    it("does nothing when problem pick is cancelled", async () => {
      cache.getProblems.returns([makeProblem()]);
      showQuickPickStub.resolves(undefined);

      await invoke("jjj.newSolution");

      sinon.assert.notCalled(showInputBoxStub);
      sinon.assert.notCalled(cli.newSolution);
    });

    it("does nothing when title input is cancelled", async () => {
      cache.getProblems.returns([makeProblem()]);
      showQuickPickStub.resolves({ label: "p1: Test", id: "p1" });
      showInputBoxStub.resolves(undefined);

      await invoke("jjj.newSolution");

      sinon.assert.notCalled(cli.newSolution);
    });
  });

  describe("jjj.submitSolution", () => {
    it("filters to proposed solutions", async () => {
      cache.getSolutions.returns([
        makeSolution({ id: "s1", title: "Fix A", status: "proposed" }),
        makeSolution({ id: "s2", title: "Fix B", status: "submitted" }),
        makeSolution({ id: "s3", title: "Fix C", status: "proposed" }),
      ]);
      showQuickPickStub.resolves({ label: "Fix A", description: "s1".slice(0, 8), id: "s1" });
      cli.submitSolution.resolves("ok");

      await invoke("jjj.submitSolution");

      const items = showQuickPickStub.firstCall.args[0] as { id: string }[];
      assert.strictEqual(items.length, 2);
      assert.deepStrictEqual(items.map(i => i.id), ["s1", "s3"]);

      sinon.assert.calledOnceWithExactly(cli.submitSolution, "s1");
      sinon.assert.calledOnce(showInfoStub);
      assert.ok((showInfoStub.firstCall.args[0] as string).includes("submitted for review"));
    });

    it("does nothing when quick pick is cancelled", async () => {
      cache.getSolutions.returns([makeSolution()]);
      showQuickPickStub.resolves(undefined);

      await invoke("jjj.submitSolution");

      sinon.assert.notCalled(cli.submitSolution);
    });
  });

  describe("jjj.approveSolution", () => {
    it("filters to submitted solutions and calls cli.approveSolution", async () => {
      cache.getSolutions.returns([
        makeSolution({ id: "s1", status: "submitted" }),
        makeSolution({ id: "s2", status: "proposed" }),
      ]);
      showQuickPickStub.resolves({ label: "Test solution", id: "s1" });
      cli.approveSolution.resolves("ok");

      await invoke("jjj.approveSolution");

      const items = showQuickPickStub.firstCall.args[0] as { id: string }[];
      assert.strictEqual(items.length, 1);
      assert.strictEqual(items[0].id, "s1");
      sinon.assert.calledOnceWithExactly(cli.approveSolution, "s1");
    });
  });

  describe("jjj.withdrawSolution", () => {
    it("filters to submitted or proposed solutions", async () => {
      cache.getSolutions.returns([
        makeSolution({ id: "s1", status: "submitted" }),
        makeSolution({ id: "s2", status: "proposed" }),
        makeSolution({ id: "s3", status: "approved" }),
        makeSolution({ id: "s4", status: "withdrawn" }),
      ]);
      showQuickPickStub.resolves({ label: "Test solution", id: "s2" });
      cli.withdrawSolution.resolves("ok");

      await invoke("jjj.withdrawSolution");

      const items = showQuickPickStub.firstCall.args[0] as { id: string }[];
      assert.strictEqual(items.length, 2);
      assert.deepStrictEqual(items.map(i => i.id), ["s1", "s2"]);
      sinon.assert.calledOnceWithExactly(cli.withdrawSolution, "s2");
    });
  });

  describe("jjj.lgtmSolution", () => {
    it("uses active solution when available (no quick pick)", async () => {
      cache.getStatus.returns(makeStatusResult({
        active_solution: { id: "s-active", title: "Active fix", problem_id: "p1", status: "submitted" },
      }));
      cli.lgtmSolution.resolves("ok");

      await invoke("jjj.lgtmSolution");

      sinon.assert.notCalled(showQuickPickStub);
      sinon.assert.calledOnceWithExactly(cli.lgtmSolution, "s-active");
      sinon.assert.calledOnce(showInfoStub);
      assert.ok((showInfoStub.firstCall.args[0] as string).includes("Active fix"));
    });

    it("falls back to quick pick when no active solution", async () => {
      cache.getStatus.returns(makeStatusResult({ active_solution: null }));
      cache.getSolutions.returns([
        makeSolution({ id: "s1", title: "Fix", status: "submitted" }),
      ]);
      showQuickPickStub.resolves({ label: "Fix", id: "s1" });
      cli.lgtmSolution.resolves("ok");

      await invoke("jjj.lgtmSolution");

      sinon.assert.calledOnce(showQuickPickStub);
      sinon.assert.calledOnceWithExactly(cli.lgtmSolution, "s1");
    });

    it("shows info message when no submitted solutions and no active solution", async () => {
      cache.getStatus.returns(makeStatusResult({ active_solution: null }));
      cache.getSolutions.returns([]);

      await invoke("jjj.lgtmSolution");

      sinon.assert.calledOnce(showInfoStub);
      assert.ok((showInfoStub.firstCall.args[0] as string).includes("No solutions currently submitted"));
      sinon.assert.notCalled(cli.lgtmSolution);
    });

    it("does nothing when quick pick is cancelled", async () => {
      cache.getStatus.returns(makeStatusResult({ active_solution: null }));
      cache.getSolutions.returns([makeSolution({ status: "submitted" })]);
      showQuickPickStub.resolves(undefined);

      await invoke("jjj.lgtmSolution");

      sinon.assert.notCalled(cli.lgtmSolution);
    });
  });

  // =========================================================================
  // Critique commands
  // =========================================================================

  describe("jjj.newCritique", () => {
    it("picks solution, asks title and severity, then creates critique", async () => {
      cache.getSolutions.returns([
        makeSolution({ id: "s1", title: "Fix", status: "submitted" }),
      ]);
      showQuickPickStub.onFirstCall().resolves({ label: "s1: Fix", id: "s1" });
      showInputBoxStub.resolves("Logic error");
      showQuickPickStub.onSecondCall().resolves("high");
      cli.newCritique.resolves("Created critique c1");

      await invoke("jjj.newCritique");

      sinon.assert.calledOnceWithExactly(cli.newCritique, "s1", "Logic error", "high");
      sinon.assert.calledOnce(showInfoStub);
    });

    it("filters to submitted/proposed solutions only", async () => {
      cache.getSolutions.returns([
        makeSolution({ id: "s1", status: "submitted" }),
        makeSolution({ id: "s2", status: "proposed" }),
        makeSolution({ id: "s3", status: "approved" }),
      ]);
      showQuickPickStub.onFirstCall().resolves({ label: "s1: Test", id: "s1" });
      showInputBoxStub.resolves("Issue");
      showQuickPickStub.onSecondCall().resolves("low");
      cli.newCritique.resolves("ok");

      await invoke("jjj.newCritique");

      const items = showQuickPickStub.firstCall.args[0] as { id: string }[];
      assert.strictEqual(items.length, 2);
    });

    it("does nothing when solution pick is cancelled", async () => {
      cache.getSolutions.returns([makeSolution({ status: "submitted" })]);
      showQuickPickStub.resolves(undefined);

      await invoke("jjj.newCritique");

      sinon.assert.notCalled(showInputBoxStub);
      sinon.assert.notCalled(cli.newCritique);
    });

    it("does nothing when title input is cancelled", async () => {
      cache.getSolutions.returns([makeSolution({ status: "submitted" })]);
      showQuickPickStub.onFirstCall().resolves({ label: "s1: Test", id: "s1" });
      showInputBoxStub.resolves(undefined);

      await invoke("jjj.newCritique");

      sinon.assert.notCalled(cli.newCritique);
    });

    it("does nothing when severity pick is cancelled", async () => {
      cache.getSolutions.returns([makeSolution({ status: "submitted" })]);
      showQuickPickStub.onFirstCall().resolves({ label: "s1: Test", id: "s1" });
      showInputBoxStub.resolves("Bug");
      showQuickPickStub.onSecondCall().resolves(undefined);

      await invoke("jjj.newCritique");

      sinon.assert.notCalled(cli.newCritique);
    });
  });

  describe("jjj.addressCritique", () => {
    it("filters to open/valid critiques and calls cli.addressCritique", async () => {
      cache.getCritiques.returns([
        makeCritique({ id: "c1", title: "Bug", severity: "high", status: "open" }),
        makeCritique({ id: "c2", title: "Perf", severity: "low", status: "valid" }),
        makeCritique({ id: "c3", title: "Old", severity: "medium", status: "addressed" }),
        makeCritique({ id: "c4", title: "Gone", severity: "low", status: "dismissed" }),
      ]);
      showQuickPickStub.resolves({ label: "c1: Bug [high]", id: "c1" });
      cli.addressCritique.resolves("Addressed");

      await invoke("jjj.addressCritique");

      const items = showQuickPickStub.firstCall.args[0] as { id: string }[];
      assert.strictEqual(items.length, 2);
      assert.deepStrictEqual(items.map(i => i.id), ["c1", "c2"]);
      sinon.assert.calledOnceWithExactly(cli.addressCritique, "c1");
    });

    it("does nothing when quick pick is cancelled", async () => {
      cache.getCritiques.returns([makeCritique()]);
      showQuickPickStub.resolves(undefined);

      await invoke("jjj.addressCritique");

      sinon.assert.notCalled(cli.addressCritique);
    });
  });

  describe("jjj.dismissCritique", () => {
    it("filters to open/valid critiques and calls cli.dismissCritique", async () => {
      cache.getCritiques.returns([
        makeCritique({ id: "c1", status: "open" }),
        makeCritique({ id: "c2", status: "valid" }),
        makeCritique({ id: "c3", status: "dismissed" }),
      ]);
      showQuickPickStub.resolves({ label: "c1: Test critique [medium]", id: "c1" });
      cli.dismissCritique.resolves("Dismissed");

      await invoke("jjj.dismissCritique");

      const items = showQuickPickStub.firstCall.args[0] as { id: string }[];
      assert.strictEqual(items.length, 2);
      sinon.assert.calledOnceWithExactly(cli.dismissCritique, "c1");
    });
  });

  describe("jjj.validateCritique", () => {
    it("filters to open critiques only", async () => {
      cache.getCritiques.returns([
        makeCritique({ id: "c1", status: "open" }),
        makeCritique({ id: "c2", status: "valid" }),
        makeCritique({ id: "c3", status: "addressed" }),
      ]);
      showQuickPickStub.resolves({ label: "c1: Test critique [medium]", id: "c1" });
      cli.validateCritique.resolves("Validated");

      await invoke("jjj.validateCritique");

      const items = showQuickPickStub.firstCall.args[0] as { id: string }[];
      assert.strictEqual(items.length, 1);
      assert.strictEqual(items[0].id, "c1");
      sinon.assert.calledOnceWithExactly(cli.validateCritique, "c1");
    });
  });

  // =========================================================================
  // Milestone commands
  // =========================================================================

  describe("jjj.newMilestone", () => {
    it("asks for title and creates milestone", async () => {
      showInputBoxStub.resolves("v1.0 release");
      cli.newMilestone.resolves("Created milestone");

      await invoke("jjj.newMilestone");

      sinon.assert.calledOnceWithExactly(cli.newMilestone, "v1.0 release");
      sinon.assert.calledOnce(showInfoStub);
    });

    it("does nothing when title input is cancelled", async () => {
      showInputBoxStub.resolves(undefined);

      await invoke("jjj.newMilestone");

      sinon.assert.notCalled(cli.newMilestone);
    });
  });

  // =========================================================================
  // GitHub sync commands
  // =========================================================================

  describe("jjj.syncGithub", () => {
    it("calls cli.syncGithub and shows result", async () => {
      cli.syncGithub.resolves("Synced 3 issues");

      await invoke("jjj.syncGithub");

      sinon.assert.calledOnce(cli.syncGithub);
      sinon.assert.calledOnce(showInfoStub);
      assert.strictEqual(showInfoStub.firstCall.args[0], "Synced 3 issues");
    });

    it("shows default message when result is empty", async () => {
      cli.syncGithub.resolves("");

      await invoke("jjj.syncGithub");

      assert.strictEqual(showInfoStub.firstCall.args[0], "GitHub sync complete");
    });
  });

  describe("jjj.syncGithubImport", () => {
    it("asks for issue number and calls cli.syncGithubImport", async () => {
      showInputBoxStub.resolves("42");
      cli.syncGithubImport.resolves("Imported issue #42");

      await invoke("jjj.syncGithubImport");

      sinon.assert.calledOnceWithExactly(cli.syncGithubImport, "42");
      sinon.assert.calledOnce(showInfoStub);
    });

    it("does nothing when input is cancelled", async () => {
      showInputBoxStub.resolves(undefined);

      await invoke("jjj.syncGithubImport");

      sinon.assert.notCalled(cli.syncGithubImport);
    });
  });

  // =========================================================================
  // Node-aware commands (registered directly, not via register())
  // =========================================================================

  describe("jjj.assignToMe", () => {
    it("assigns a problem node to current user", async () => {
      cache.getStatus.returns(makeStatusResult({ user: "alice" }));
      cli.assignProblem.resolves("Assigned to alice");

      const node = { problem: { id: "p1", title: "Bug" } };
      await invoke("jjj.assignToMe", node);

      sinon.assert.calledOnceWithExactly(cli.assignProblem, "p1", "alice");
      sinon.assert.calledOnce(showInfoStub);
      sinon.assert.calledOnce(cache.refresh);
    });

    it("assigns a solution node to current user", async () => {
      cache.getStatus.returns(makeStatusResult({ user: "bob" }));
      cli.assignSolution.resolves("Assigned to bob");

      const node = { solution: { id: "s1", title: "Fix" } };
      await invoke("jjj.assignToMe", node);

      sinon.assert.calledOnceWithExactly(cli.assignSolution, "s1", "bob");
      sinon.assert.calledOnce(cache.refresh);
    });

    it("falls back to quick pick when no node is provided", async () => {
      cache.getStatus.returns(makeStatusResult({ user: "alice" }));
      cache.getProblems.returns([
        makeProblem({ id: "p1", title: "Bug", status: "open" }),
        makeProblem({ id: "p2", title: "Done", status: "solved" }),
      ]);
      showQuickPickStub.resolves({ label: "p1: Bug", id: "p1", type: "problem" });
      cli.assignProblem.resolves("Assigned");

      await invoke("jjj.assignToMe");

      // Only open/in_progress problems in the pick list
      const items = showQuickPickStub.firstCall.args[0] as { id: string }[];
      assert.strictEqual(items.length, 1);

      sinon.assert.calledOnceWithExactly(cli.assignProblem, "p1", "alice");
    });

    it("shows warning when user cannot be determined", async () => {
      cache.getStatus.returns(null);

      await invoke("jjj.assignToMe");

      sinon.assert.calledOnce(showWarningStub);
      sinon.assert.notCalled(cli.assignProblem);
      sinon.assert.notCalled(cli.assignSolution);
    });

    it("does nothing when quick pick is cancelled", async () => {
      cache.getStatus.returns(makeStatusResult({ user: "alice" }));
      cache.getProblems.returns([makeProblem()]);
      showQuickPickStub.resolves(undefined);

      await invoke("jjj.assignToMe");

      sinon.assert.notCalled(cli.assignProblem);
      sinon.assert.notCalled(cli.assignSolution);
    });

    it("shows error message on CLI failure", async () => {
      cache.getStatus.returns(makeStatusResult({ user: "alice" }));
      cli.assignProblem.rejects(new Error("network error"));

      const node = { problem: { id: "p1", title: "Bug" } };
      await invoke("jjj.assignToMe", node);

      sinon.assert.calledOnce(showErrorStub);
      const msg = showErrorStub.firstCall.args[0] as string;
      assert.ok(msg.startsWith("JJJ:"));
      assert.ok(msg.includes("network error"));
    });
  });

  describe("jjj.editProblem", () => {
    it("renames a problem when node is provided", async () => {
      showInputBoxStub.resolves("New title");
      cli.editProblem.resolves("ok");

      const node = { problem: { id: "p1", title: "Old title" } };
      await invoke("jjj.editProblem", node);

      sinon.assert.calledOnceWithExactly(cli.editProblem, "p1", "New title");
      sinon.assert.calledOnce(showInfoStub);
      assert.ok((showInfoStub.firstCall.args[0] as string).includes("New title"));
      sinon.assert.calledOnce(cache.refresh);
    });

    it("falls back to quick pick when no node is provided", async () => {
      cache.getProblems.returns([
        makeProblem({ id: "p1", title: "Bug" }),
        makeProblem({ id: "p2", title: "Feature" }),
      ]);
      showQuickPickStub.resolves({ label: "Bug", description: "p1".slice(0, 8), id: "p1", title: "Bug" });
      showInputBoxStub.resolves("Updated Bug");
      cli.editProblem.resolves("ok");

      await invoke("jjj.editProblem");

      sinon.assert.calledOnceWithExactly(cli.editProblem, "p1", "Updated Bug");
    });

    it("does nothing when quick pick is cancelled (no node)", async () => {
      cache.getProblems.returns([makeProblem()]);
      showQuickPickStub.resolves(undefined);

      await invoke("jjj.editProblem");

      sinon.assert.notCalled(showInputBoxStub);
      sinon.assert.notCalled(cli.editProblem);
    });

    it("does nothing when new title input is cancelled", async () => {
      showInputBoxStub.resolves(undefined);

      const node = { problem: { id: "p1", title: "Old" } };
      await invoke("jjj.editProblem", node);

      sinon.assert.notCalled(cli.editProblem);
    });

    it("pre-fills input box with current title", async () => {
      showInputBoxStub.resolves("Same title");
      cli.editProblem.resolves("ok");

      const node = { problem: { id: "p1", title: "Current title" } };
      await invoke("jjj.editProblem", node);

      const inputOptions = showInputBoxStub.firstCall.args[0] as { value: string };
      assert.strictEqual(inputOptions.value, "Current title");
    });
  });

  describe("jjj.editSolution", () => {
    it("renames a solution when node is provided", async () => {
      showInputBoxStub.resolves("Better fix");
      cli.editSolution.resolves("ok");

      const node = { solution: { id: "s1", title: "Old fix" } };
      await invoke("jjj.editSolution", node);

      sinon.assert.calledOnceWithExactly(cli.editSolution, "s1", "Better fix");
      sinon.assert.calledOnce(cache.refresh);
    });

    it("falls back to quick pick when no node is provided", async () => {
      cache.getSolutions.returns([makeSolution({ id: "s1", title: "Fix" })]);
      showQuickPickStub.resolves({ label: "Fix", description: "s1".slice(0, 8), id: "s1", title: "Fix" });
      showInputBoxStub.resolves("Updated Fix");
      cli.editSolution.resolves("ok");

      await invoke("jjj.editSolution");

      sinon.assert.calledOnceWithExactly(cli.editSolution, "s1", "Updated Fix");
    });
  });

  // =========================================================================
  // reopenProblem (via register())
  // =========================================================================

  describe("jjj.reopenProblem", () => {
    it("shows info message when no solved/dissolved problems exist", async () => {
      cache.getProblems.returns([
        makeProblem({ id: "p1", status: "open" }),
        makeProblem({ id: "p2", status: "in_progress" }),
      ]);

      await invoke("jjj.reopenProblem");

      sinon.assert.calledOnce(showInfoStub);
      assert.ok((showInfoStub.firstCall.args[0] as string).includes("No solved or dissolved"));
      sinon.assert.notCalled(cli.reopenProblem);
    });

    it("filters to solved/dissolved problems and calls cli.reopenProblem", async () => {
      cache.getProblems.returns([
        makeProblem({ id: "p1", title: "Open", status: "open" }),
        makeProblem({ id: "p2", title: "Solved", status: "solved" }),
        makeProblem({ id: "p3", title: "Dissolved", status: "dissolved" }),
      ]);
      showQuickPickStub.resolves({ label: "p2: Solved", description: "solved", id: "p2" });
      cli.reopenProblem.resolves("Reopened");

      await invoke("jjj.reopenProblem");

      const items = showQuickPickStub.firstCall.args[0] as { id: string }[];
      assert.strictEqual(items.length, 2);
      assert.deepStrictEqual(items.map(i => i.id), ["p2", "p3"]);
      sinon.assert.calledOnceWithExactly(cli.reopenProblem, "p2");
    });

    it("does nothing when quick pick is cancelled", async () => {
      cache.getProblems.returns([makeProblem({ status: "solved" })]);
      showQuickPickStub.resolves(undefined);

      await invoke("jjj.reopenProblem");

      sinon.assert.notCalled(cli.reopenProblem);
    });
  });

  // =========================================================================
  // switchToChange (node-aware)
  // =========================================================================

  describe("jjj.switchToChange", () => {
    it("uses solution from node arg when provided", async () => {
      cache.getSolutions.returns([
        makeSolution({ id: "s1", change_ids: ["abc123"] }),
      ]);
      cli.resumeSolution.resolves("Switched to s1");

      const node = { solution: { id: "s1" } };
      await invoke("jjj.switchToChange", node);

      sinon.assert.calledOnceWithExactly(cli.resumeSolution, "s1");
      sinon.assert.calledOnce(showInfoStub);
      sinon.assert.calledOnce(cache.refresh);
    });

    it("falls back to quick pick when no node is provided", async () => {
      cache.getSolutions.returns([
        makeSolution({ id: "s1", title: "Fix A", change_ids: ["abc"] }),
        makeSolution({ id: "s2", title: "Fix B", change_ids: [] }),
      ]);
      showQuickPickStub.resolves({ label: "Fix A", id: "s1" });
      cli.resumeSolution.resolves("Switched");

      await invoke("jjj.switchToChange");

      // Only solutions with changes shown
      const items = showQuickPickStub.firstCall.args[0] as { id: string }[];
      assert.strictEqual(items.length, 1);
      assert.strictEqual(items[0].id, "s1");
    });

    it("shows warning when solution has no changes", async () => {
      cache.getSolutions.returns([
        makeSolution({ id: "s1", change_ids: [] }),
      ]);

      const node = { solution: { id: "s1" } };
      await invoke("jjj.switchToChange", node);

      sinon.assert.calledOnce(showWarningStub);
      sinon.assert.notCalled(cli.resumeSolution);
    });
  });

  // =========================================================================
  // moveProblemToMilestone (node-aware)
  // =========================================================================

  describe("jjj.moveProblemToMilestone", () => {
    it("uses problem from node and moves to selected milestone", async () => {
      const problem = makeProblem({ id: "p1", milestone_id: null });
      cache.getProblems.returns([problem]);
      cache.getMilestones.returns([makeMilestone({ id: "m1", title: "Sprint 1" })]);

      showQuickPickStub.resolves({ label: "$(milestone) Sprint 1", id: "m1" });
      cli.milestoneAddProblem.resolves("ok");

      const node = { problem: { id: "p1" } };
      await invoke("jjj.moveProblemToMilestone", node);

      sinon.assert.calledOnceWithExactly(cli.milestoneAddProblem, "m1", "p1");
      sinon.assert.notCalled(cli.milestoneRemoveProblem);
      sinon.assert.calledOnce(cache.refresh);
    });

    it("removes from old milestone before adding to new one", async () => {
      const problem = makeProblem({ id: "p1", milestone_id: "m-old" });
      cache.getProblems.returns([problem]);
      cache.getMilestones.returns([makeMilestone({ id: "m-new", title: "Sprint 2" })]);

      showQuickPickStub.resolves({ label: "$(milestone) Sprint 2", id: "m-new" });
      cli.milestoneRemoveProblem.resolves("ok");
      cli.milestoneAddProblem.resolves("ok");

      const node = { problem: { id: "p1" } };
      await invoke("jjj.moveProblemToMilestone", node);

      sinon.assert.calledOnceWithExactly(cli.milestoneRemoveProblem, "m-old", "p1");
      sinon.assert.calledOnceWithExactly(cli.milestoneAddProblem, "m-new", "p1");
    });

    it("moves to backlog (removes from milestone, does not add)", async () => {
      const problem = makeProblem({ id: "p1", milestone_id: "m-old" });
      cache.getProblems.returns([problem]);
      cache.getMilestones.returns([]);

      showQuickPickStub.resolves({ label: "$(inbox) Backlog (no milestone)", id: null });
      cli.milestoneRemoveProblem.resolves("ok");

      const node = { problem: { id: "p1" } };
      await invoke("jjj.moveProblemToMilestone", node);

      sinon.assert.calledOnceWithExactly(cli.milestoneRemoveProblem, "m-old", "p1");
      sinon.assert.notCalled(cli.milestoneAddProblem);
    });

    it("falls back to quick pick when no node provided", async () => {
      cache.getProblems.returns([makeProblem({ id: "p1", status: "open", title: "Bug" })]);
      cache.getMilestones.returns([makeMilestone({ id: "m1", title: "Sprint" })]);

      // First pick: problem
      showQuickPickStub.onFirstCall().resolves({ label: "p1: Bug", id: "p1" });
      // Second pick: milestone
      showQuickPickStub.onSecondCall().resolves({ label: "$(milestone) Sprint", id: "m1" });
      cli.milestoneAddProblem.resolves("ok");

      await invoke("jjj.moveProblemToMilestone");

      sinon.assert.calledOnceWithExactly(cli.milestoneAddProblem, "m1", "p1");
    });
  });

  // =========================================================================
  // newProblemInMilestone (node-aware)
  // =========================================================================

  describe("jjj.newProblemInMilestone", () => {
    it("creates problem in milestone using node arg", async () => {
      showInputBoxStub.resolves("New bug");
      cli.newProblem.resolves("Created");
      cache.getProblems.returns([makeProblem({ id: "p-new", title: "New bug" })]);
      cli.milestoneAddProblem.resolves("ok");

      const node = { milestone: { id: "m1", title: "Sprint 1" } };
      await invoke("jjj.newProblemInMilestone", node);

      sinon.assert.calledOnceWithExactly(cli.newProblem, "New bug");
      sinon.assert.calledOnceWithExactly(cli.milestoneAddProblem, "m1", "p-new");
      sinon.assert.calledOnce(showInfoStub);
      assert.ok((showInfoStub.firstCall.args[0] as string).includes("Sprint 1"));
    });

    it("creates problem in backlog when node has null milestone", async () => {
      showInputBoxStub.resolves("Backlog item");
      cli.newProblem.resolves("Created");

      const node = { milestone: null };
      await invoke("jjj.newProblemInMilestone", node);

      sinon.assert.calledOnceWithExactly(cli.newProblem, "Backlog item");
      sinon.assert.notCalled(cli.milestoneAddProblem);
      assert.ok((showInfoStub.firstCall.args[0] as string).includes("Backlog"));
    });

    it("does nothing when title input is cancelled", async () => {
      showInputBoxStub.resolves(undefined);

      const node = { milestone: { id: "m1", title: "Sprint 1" } };
      await invoke("jjj.newProblemInMilestone", node);

      sinon.assert.notCalled(cli.newProblem);
    });
  });

  // =========================================================================
  // Tag editing commands
  // =========================================================================

  describe("jjj.editProblemTags", () => {
    it("shows tag picker with node and calls cli.editProblemTags", async () => {
      cache.getProblems.returns([makeProblem({ id: "p1", tags: ["bug"] })]);
      cli.listTags.resolves([{ tag: "bug", count: 1 }, { tag: "feat", count: 2 }]);
      showQuickPickStub.resolves([{ label: "bug", picked: true }, { label: "feat", picked: false }]);
      cli.editProblemTags.resolves("ok");

      const node = { problem: { id: "p1" } };
      await invoke("jjj.editProblemTags", node);

      sinon.assert.calledOnceWithExactly(cli.editProblemTags, "p1", ["bug", "feat"]);
      sinon.assert.calledOnce(cache.refresh);
    });

    it("falls back to quick pick when no node", async () => {
      cache.getProblems.returns([makeProblem({ id: "p1", title: "Bug", tags: [] })]);
      showQuickPickStub.onFirstCall().resolves({ label: "Bug", description: "p1".slice(0, 8), id: "p1" });
      cli.listTags.resolves([]);
      showQuickPickStub.onSecondCall().resolves([]);
      cli.editProblemTags.resolves("ok");

      await invoke("jjj.editProblemTags");

      sinon.assert.calledOnceWithExactly(cli.editProblemTags, "p1", []);
    });

    it("does nothing when tag selection is cancelled", async () => {
      cache.getProblems.returns([makeProblem({ id: "p1", tags: [] })]);
      cli.listTags.resolves([]);
      showQuickPickStub.resolves(undefined);

      const node = { problem: { id: "p1" } };
      await invoke("jjj.editProblemTags", node);

      sinon.assert.notCalled(cli.editProblemTags);
    });
  });

  describe("jjj.editSolutionTags", () => {
    it("shows tag picker with node and calls cli.editSolutionTags", async () => {
      cache.getSolutions.returns([makeSolution({ id: "s1", tags: ["wip"] })]);
      cli.listTags.resolves([{ tag: "wip", count: 1 }]);
      showQuickPickStub.resolves([{ label: "wip", picked: true }]);
      cli.editSolutionTags.resolves("ok");

      const node = { solution: { id: "s1" } };
      await invoke("jjj.editSolutionTags", node);

      sinon.assert.calledOnceWithExactly(cli.editSolutionTags, "s1", ["wip"]);
      sinon.assert.calledOnce(cache.refresh);
    });
  });

  // =========================================================================
  // resumeSolution (via register())
  // =========================================================================

  describe("jjj.resumeSolution", () => {
    it("filters to submitted/proposed solutions and calls cli.resumeSolution", async () => {
      cache.getSolutions.returns([
        makeSolution({ id: "s1", status: "submitted" }),
        makeSolution({ id: "s2", status: "approved" }),
      ]);
      showQuickPickStub.resolves({ label: "s1: Test solution", id: "s1" });
      cli.resumeSolution.resolves("Resumed");

      await invoke("jjj.resumeSolution");

      const items = showQuickPickStub.firstCall.args[0] as { id: string }[];
      assert.strictEqual(items.length, 1);
      sinon.assert.calledOnceWithExactly(cli.resumeSolution, "s1");
    });
  });

  // =========================================================================
  // Registration completeness
  // =========================================================================

  describe("registration", () => {
    it("registers all expected commands", () => {
      const expectedCommands = [
        "jjj.newProblem",
        "jjj.solveProblem",
        "jjj.dissolveProblem",
        "jjj.newSolution",
        "jjj.submitSolution",
        "jjj.approveSolution",
        "jjj.withdrawSolution",
        "jjj.resumeSolution",
        "jjj.lgtmSolution",
        "jjj.newCritique",
        "jjj.addressCritique",
        "jjj.dismissCritique",
        "jjj.validateCritique",
        "jjj.newMilestone",
        "jjj.switchToChange",
        "jjj.addCritiqueHere",
        "jjj.syncGithub",
        "jjj.syncGithubImport",
        "jjj.syncGithubStatus",
        "jjj.assignToMe",
        "jjj.moveProblemToMilestone",
        "jjj.newProblemInMilestone",
        "jjj.reopenProblem",
        "jjj.editProblem",
        "jjj.editProblemTags",
        "jjj.editSolutionTags",
        "jjj.editSolution",
      ];

      for (const cmd of expectedCommands) {
        assert.ok(commandHandlers.has(cmd), `Expected command "${cmd}" to be registered`);
      }
    });
  });
});
