import * as assert from "assert";
import * as sinon from "sinon";
import * as vscode from "vscode";
import { CritiqueCommentController } from "../../src/editor/critiqueComments";
import { DataCache } from "../../src/cache";
import { JjjCli, Critique, Solution } from "../../src/cli";

// --- Fixtures ---

function makeCritique(overrides: Partial<Critique> = {}): Critique {
  return {
    id: "c1",
    title: "Test critique",
    solution_id: "s1",
    status: "open",
    severity: "medium",
    author: "alice",
    reviewer: undefined,
    created_at: "2024-01-01T00:00:00Z",
    updated_at: "2024-01-01T00:00:00Z",
    argument: "Something is wrong",
    evidence: "",
    file_path: "src/foo.rs",
    line_start: 10,
    line_end: null,
    code_context: [],
    replies: [],
    ...overrides,
  };
}

function makeSolution(overrides: Partial<Solution> = {}): Solution {
  return {
    id: "s1",
    title: "Fix the bug",
    problem_id: "p1",
    status: "testing",
    critique_ids: [],
    change_ids: [],
    assignee: null,
    force_accepted: false,
    created_at: "",
    updated_at: "",
    approach: "",
    tradeoffs: "",
    supersedes: null,
    ...overrides,
  };
}

// --- Mock thread factory ---

function makeMockThread() {
  return {
    label: undefined as string | undefined,
    state: vscode.CommentThreadState.Unresolved as number,
    canReply: true,
    contextValue: undefined as string | undefined,
    collapsibleState: vscode.CommentThreadCollapsibleState.Expanded as number,
    comments: [] as unknown[],
    dispose: sinon.stub(),
  };
}

// Minimal manual cache mock (onDidChange is an instance property, not a prototype method,
// so sinon.createStubInstance cannot stub it automatically)
interface MockCache {
  onDidChange: (cb: () => void) => { dispose: () => void };
  getCritiquesWithLocations: sinon.SinonStub;
  getSolution: sinon.SinonStub;
  getSolutions: sinon.SinonStub;
  getStatus: sinon.SinonStub;
  refresh: sinon.SinonStub;
}

function makeMockCache(): MockCache & { _fireCacheChange: () => void } {
  let _cb: (() => void) | undefined;
  return {
    onDidChange: sinon.stub().callsFake((cb: () => void) => {
      _cb = cb;
      return { dispose: sinon.stub() };
    }),
    getCritiquesWithLocations: sinon.stub().returns([]),
    getSolution: sinon.stub().returns(undefined),
    getSolutions: sinon.stub().returns([]),
    getStatus: sinon.stub().returns(null),
    refresh: sinon.stub().resolves(),
    _fireCacheChange: () => { if (_cb) { _cb(); } },
  };
}

// Temp thread factory — simulates a thread created by VS Code when the user clicks gutter +
function makeTempThread(filePath = "src/foo.rs", line = 9) {
  const wsUri = vscode.workspace.workspaceFolders![0].uri as vscode.Uri;
  const uri = vscode.Uri.joinPath(wsUri, filePath);
  return {
    uri,
    range: { start: { line, character: 0 }, end: { line, character: 0 } },
    label: undefined as string | undefined,
    state: 0,
    canReply: true,
    contextValue: undefined as string | undefined,
    collapsibleState: 0,
    comments: [] as unknown[],
    dispose: sinon.stub(),
  };
}

// --- Tests ---

describe("CritiqueCommentController", () => {
  let cache: ReturnType<typeof makeMockCache>;
  let cli: sinon.SinonStubbedInstance<JjjCli>;
  let mockController: {
    options: Record<string, unknown>;
    createCommentThread: sinon.SinonStub;
    dispose: sinon.SinonStub;
    _threads: ReturnType<typeof makeMockThread>[];
  };
  let createControllerStub: sinon.SinonStub;

  beforeEach(() => {
    cache = makeMockCache();
    cli = sinon.createStubInstance(JjjCli);

    // Build mock controller that tracks created threads
    const threads: ReturnType<typeof makeMockThread>[] = [];
    mockController = {
      options: {},
      createCommentThread: sinon.stub().callsFake(() => {
        const t = makeMockThread();
        threads.push(t);
        return t;
      }),
      dispose: sinon.stub(),
      _threads: threads,
    };

    createControllerStub = sinon
      .stub(vscode.comments, "createCommentController")
      .returns(mockController as unknown as vscode.CommentController);
  });

  afterEach(() => {
    sinon.restore();
  });

  describe("syncThreads", () => {
    it("creates a thread for a critique with file_path and line_start", () => {
      cache.getCritiquesWithLocations.returns([makeCritique()]);
      new CritiqueCommentController(cache as unknown as DataCache, cli as unknown as JjjCli);

      assert.strictEqual(mockController.createCommentThread.callCount, 1);
      assert.strictEqual(mockController._threads.length, 1);
    });

    it("skips critiques without file_path", () => {
      cache.getCritiquesWithLocations.returns([makeCritique({ file_path: null, line_start: null })]);
      new CritiqueCommentController(cache as unknown as DataCache, cli as unknown as JjjCli);

      assert.strictEqual(mockController.createCommentThread.callCount, 0);
    });

    it("skips critiques without line_start", () => {
      cache.getCritiquesWithLocations.returns([makeCritique({ line_start: null })]);
      new CritiqueCommentController(cache as unknown as DataCache, cli as unknown as JjjCli);

      assert.strictEqual(mockController.createCommentThread.callCount, 0);
    });

    it("disposes stale threads on re-sync", () => {
      cache.getCritiquesWithLocations.returns([makeCritique()]);
      const controller = new CritiqueCommentController(
        cache as unknown as DataCache,
        cli as unknown as JjjCli,
      );

      assert.strictEqual(mockController._threads.length, 1);
      const firstThread = mockController._threads[0];

      // Remove the critique from cache
      cache.getCritiquesWithLocations.returns([]);
      controller.syncThreads();

      sinon.assert.calledOnce(firstThread.dispose);
    });

    it("updates existing thread on re-sync without creating a new one", () => {
      cache.getCritiquesWithLocations.returns([makeCritique({ status: "open" })]);
      const controller = new CritiqueCommentController(
        cache as unknown as DataCache,
        cli as unknown as JjjCli,
      );

      assert.strictEqual(mockController.createCommentThread.callCount, 1);

      // Update critique status
      cache.getCritiquesWithLocations.returns([makeCritique({ status: "addressed" })]);
      controller.syncThreads();

      // No new thread created
      assert.strictEqual(mockController.createCommentThread.callCount, 1);
      // Existing thread updated
      assert.strictEqual(mockController._threads[0].state, vscode.CommentThreadState.Resolved);
      assert.strictEqual(mockController._threads[0].canReply, false);
      assert.strictEqual(mockController._threads[0].contextValue, "addressed");
    });

    it("sets thread state to Resolved for addressed critique", () => {
      cache.getCritiquesWithLocations.returns([makeCritique({ status: "addressed" })]);
      new CritiqueCommentController(cache as unknown as DataCache, cli as unknown as JjjCli);

      assert.strictEqual(mockController._threads[0].state, vscode.CommentThreadState.Resolved);
    });

    it("sets thread state to Resolved for dismissed critique", () => {
      cache.getCritiquesWithLocations.returns([makeCritique({ status: "dismissed" })]);
      new CritiqueCommentController(cache as unknown as DataCache, cli as unknown as JjjCli);

      assert.strictEqual(mockController._threads[0].state, vscode.CommentThreadState.Resolved);
    });

    it("sets thread state to Unresolved for open critique", () => {
      cache.getCritiquesWithLocations.returns([makeCritique({ status: "open" })]);
      new CritiqueCommentController(cache as unknown as DataCache, cli as unknown as JjjCli);

      assert.strictEqual(mockController._threads[0].state, vscode.CommentThreadState.Unresolved);
    });

    it("sets thread state to Unresolved for valid critique", () => {
      cache.getCritiquesWithLocations.returns([makeCritique({ status: "valid" })]);
      new CritiqueCommentController(cache as unknown as DataCache, cli as unknown as JjjCli);

      assert.strictEqual(mockController._threads[0].state, vscode.CommentThreadState.Unresolved);
    });

    it("sets canReply=false for resolved critiques", () => {
      cache.getCritiquesWithLocations.returns([makeCritique({ status: "dismissed" })]);
      new CritiqueCommentController(cache as unknown as DataCache, cli as unknown as JjjCli);

      assert.strictEqual(mockController._threads[0].canReply, false);
    });

    it("sets canReply=true for open critiques", () => {
      cache.getCritiquesWithLocations.returns([makeCritique({ status: "open" })]);
      new CritiqueCommentController(cache as unknown as DataCache, cli as unknown as JjjCli);

      assert.strictEqual(mockController._threads[0].canReply, true);
    });

    it("sets contextValue to critique status string", () => {
      cache.getCritiquesWithLocations.returns([makeCritique({ status: "valid" })]);
      new CritiqueCommentController(cache as unknown as DataCache, cli as unknown as JjjCli);

      assert.strictEqual(mockController._threads[0].contextValue, "valid");
    });

    it("sets thread label to solution title when available", () => {
      cache.getCritiquesWithLocations.returns([makeCritique()]);
      cache.getSolution.returns(makeSolution({ title: "Fix the bug" }));
      new CritiqueCommentController(cache as unknown as DataCache, cli as unknown as JjjCli);

      assert.strictEqual(mockController._threads[0].label, "Fix the bug");
    });

    it("sets thread label to 'jjj Critique' when solution not found", () => {
      cache.getCritiquesWithLocations.returns([makeCritique()]);
      cache.getSolution.returns(undefined);
      new CritiqueCommentController(cache as unknown as DataCache, cli as unknown as JjjCli);

      assert.strictEqual(mockController._threads[0].label, "jjj Critique");
    });

    it("re-syncs when onDidChange fires", () => {
      cache.getCritiquesWithLocations.returns([]);
      new CritiqueCommentController(cache as unknown as DataCache, cli as unknown as JjjCli);

      assert.strictEqual(mockController.createCommentThread.callCount, 0);

      // Fire cache change with a new critique
      cache.getCritiquesWithLocations.returns([makeCritique()]);
      cache._fireCacheChange();

      assert.strictEqual(mockController.createCommentThread.callCount, 1);
    });
  });

  describe("findCritiqueIdForThread", () => {
    it("returns the critique id for a known thread", () => {
      cache.getCritiquesWithLocations.returns([makeCritique({ id: "c1" })]);
      const controller = new CritiqueCommentController(
        cache as unknown as DataCache,
        cli as unknown as JjjCli,
      );

      const thread = mockController._threads[0];
      assert.strictEqual(controller.findCritiqueIdForThread(thread as unknown as vscode.CommentThread), "c1");
    });

    it("returns undefined for an unknown thread", () => {
      cache.getCritiquesWithLocations.returns([makeCritique()]);
      const controller = new CritiqueCommentController(
        cache as unknown as DataCache,
        cli as unknown as JjjCli,
      );

      const strangerThread = makeMockThread();
      assert.strictEqual(
        controller.findCritiqueIdForThread(strangerThread as unknown as vscode.CommentThread),
        undefined,
      );
    });
  });

  describe("addressCritique", () => {
    it("calls cli.addressCritique with the critique id then refreshes cache", async () => {
      cache.getCritiquesWithLocations.returns([makeCritique({ id: "c1" })]);
      cli.addressCritique.resolves("ok");

      const controller = new CritiqueCommentController(
        cache as unknown as DataCache,
        cli as unknown as JjjCli,
      );
      const thread = mockController._threads[0];

      await controller.addressCritique(thread as unknown as vscode.CommentThread);

      sinon.assert.calledOnceWithExactly(cli.addressCritique, "c1");
      sinon.assert.calledOnce(cache.refresh);
    });

    it("does nothing for an unknown thread", async () => {
      cache.getCritiquesWithLocations.returns([makeCritique()]);
      const controller = new CritiqueCommentController(
        cache as unknown as DataCache,
        cli as unknown as JjjCli,
      );

      const unknown = makeMockThread();
      await controller.addressCritique(unknown as unknown as vscode.CommentThread);

      sinon.assert.notCalled(cli.addressCritique);
      sinon.assert.notCalled(cache.refresh);
    });
  });

  describe("dismissCritique", () => {
    it("calls cli.dismissCritique with the critique id then refreshes cache", async () => {
      cache.getCritiquesWithLocations.returns([makeCritique({ id: "c2" })]);
      cli.dismissCritique.resolves("ok");

      const controller = new CritiqueCommentController(
        cache as unknown as DataCache,
        cli as unknown as JjjCli,
      );
      const thread = mockController._threads[0];

      await controller.dismissCritique(thread as unknown as vscode.CommentThread);

      sinon.assert.calledOnceWithExactly(cli.dismissCritique, "c2");
      sinon.assert.calledOnce(cache.refresh);
    });
  });

  describe("validateCritique", () => {
    it("calls cli.validateCritique with the critique id then refreshes cache", async () => {
      cache.getCritiquesWithLocations.returns([makeCritique({ id: "c3" })]);
      cli.validateCritique.resolves("ok");

      const controller = new CritiqueCommentController(
        cache as unknown as DataCache,
        cli as unknown as JjjCli,
      );
      const thread = mockController._threads[0];

      await controller.validateCritique(thread as unknown as vscode.CommentThread);

      sinon.assert.calledOnceWithExactly(cli.validateCritique, "c3");
      sinon.assert.calledOnce(cache.refresh);
    });
  });

  describe("replyToCritique", () => {
    it("calls cli.replyCritique with the critique id and text then refreshes cache", async () => {
      cache.getCritiquesWithLocations.returns([makeCritique({ id: "c1" })]);
      cli.replyCritique.resolves("ok");

      const controller = new CritiqueCommentController(
        cache as unknown as DataCache,
        cli as unknown as JjjCli,
      );
      const thread = mockController._threads[0];

      const reply = {
        thread: thread as unknown as vscode.CommentThread,
        text: "Good point, I will fix this",
      };

      await controller.replyToCritique(reply);

      sinon.assert.calledOnceWithExactly(cli.replyCritique, "c1", "Good point, I will fix this");
      sinon.assert.calledOnce(cache.refresh);
    });

    it("routes to new critique creation when thread is not in the known map", async () => {
      // An unknown thread triggers _createCritique, NOT replyCritique.
      // showQuickPick returns undefined (mock default) → severity cancelled → dispose + early return.
      cache.getCritiquesWithLocations.returns([makeCritique()]);
      const controller = new CritiqueCommentController(
        cache as unknown as DataCache,
        cli as unknown as JjjCli,
      );

      const unknown = makeMockThread();
      await controller.replyToCritique({
        thread: unknown as unknown as vscode.CommentThread,
        text: "hello",
      });

      sinon.assert.notCalled(cli.replyCritique);
      sinon.assert.notCalled(cli.newCritique);
      sinon.assert.calledOnce(unknown.dispose); // temp thread disposed on cancellation
    });
  });

  // ---------------------------------------------------------------------------
  // Phase 2: inline critique creation via gutter + button
  // ---------------------------------------------------------------------------

  describe("replyToCritique — new critique creation (gutter +)", () => {
    it("creates a critique with correct solution, title, severity, file, and line", async () => {
      cache.getCritiquesWithLocations.returns([]);
      cache.getStatus.returns({
        active_solution: { id: "s-active", title: "Fix bug", problem_id: "p1", status: "testing" },
        items: [], total_count: 0, user: "test",
        summary: { open_problems: 0, testing_solutions: 0, open_critiques: 0 },
      });
      cli.newCritique.resolves("new-id");

      sinon.stub(vscode.window, "showQuickPick").resolves("high" as unknown as vscode.QuickPickItem);

      const controller = new CritiqueCommentController(
        cache as unknown as DataCache,
        cli as unknown as JjjCli,
      );
      const tempThread = makeTempThread("src/foo.rs", 9); // line 9 (0-indexed) → line 10 (1-indexed)

      await controller.replyToCritique({
        thread: tempThread as unknown as vscode.CommentThread,
        text: "This logic is wrong",
      });

      sinon.assert.calledOnceWithExactly(cli.newCritique, "s-active", "This logic is wrong", "high", "src/foo.rs", 10);
      sinon.assert.calledOnce(cache.refresh);
      sinon.assert.calledOnce(tempThread.dispose);
    });

    it("disposes temp thread and skips creation when title is empty/whitespace", async () => {
      cache.getCritiquesWithLocations.returns([]);
      cache.getStatus.returns(null);

      const controller = new CritiqueCommentController(
        cache as unknown as DataCache,
        cli as unknown as JjjCli,
      );
      const tempThread = makeTempThread();

      await controller.replyToCritique({
        thread: tempThread as unknown as vscode.CommentThread,
        text: "   ",
      });

      sinon.assert.calledOnce(tempThread.dispose);
      sinon.assert.notCalled(cli.newCritique);
      sinon.assert.notCalled(cache.refresh);
    });

    it("disposes temp thread when severity pick is cancelled", async () => {
      cache.getCritiquesWithLocations.returns([]);
      cache.getStatus.returns(null);
      sinon.stub(vscode.window, "showQuickPick").resolves(undefined);

      const controller = new CritiqueCommentController(
        cache as unknown as DataCache,
        cli as unknown as JjjCli,
      );
      const tempThread = makeTempThread();

      await controller.replyToCritique({
        thread: tempThread as unknown as vscode.CommentThread,
        text: "Some issue",
      });

      sinon.assert.calledOnce(tempThread.dispose);
      sinon.assert.notCalled(cli.newCritique);
      sinon.assert.notCalled(cache.refresh);
    });

    it("disposes temp thread when no solution candidates exist", async () => {
      cache.getCritiquesWithLocations.returns([]);
      cache.getStatus.returns(null);
      cache.getSolutions.returns([]);
      sinon.stub(vscode.window, "showQuickPick").resolves("medium" as unknown as vscode.QuickPickItem);

      const controller = new CritiqueCommentController(
        cache as unknown as DataCache,
        cli as unknown as JjjCli,
      );
      const tempThread = makeTempThread();

      await controller.replyToCritique({
        thread: tempThread as unknown as vscode.CommentThread,
        text: "Some issue",
      });

      sinon.assert.calledOnce(tempThread.dispose);
      sinon.assert.notCalled(cli.newCritique);
    });

    it("uses single candidate solution without a second quick pick", async () => {
      cache.getCritiquesWithLocations.returns([]);
      cache.getStatus.returns(null);
      cache.getSolutions.returns([makeSolution({ id: "s-solo", status: "testing" })]);
      cli.newCritique.resolves("nid");

      const qpStub = sinon.stub(vscode.window, "showQuickPick")
        .resolves("critical" as unknown as vscode.QuickPickItem);

      const controller = new CritiqueCommentController(
        cache as unknown as DataCache,
        cli as unknown as JjjCli,
      );
      const tempThread = makeTempThread();

      await controller.replyToCritique({
        thread: tempThread as unknown as vscode.CommentThread,
        text: "Bug here",
      });

      // showQuickPick called exactly once (severity only; solution resolved without prompt)
      assert.strictEqual(qpStub.callCount, 1);
      assert.strictEqual((cli.newCritique.firstCall.args as string[])[0], "s-solo");
    });

    it("shows solution quick pick when multiple candidates exist", async () => {
      cache.getCritiquesWithLocations.returns([]);
      cache.getStatus.returns(null);
      cache.getSolutions.returns([
        makeSolution({ id: "s1", title: "Fix A", status: "testing" }),
        makeSolution({ id: "s2", title: "Fix B", status: "proposed" }),
      ]);
      cli.newCritique.resolves("nid");

      const qpStub = sinon.stub(vscode.window, "showQuickPick");
      qpStub.onFirstCall().resolves("medium" as unknown as vscode.QuickPickItem); // severity
      qpStub.onSecondCall().resolves({ label: "Fix A", description: "s1", id: "s1" } as unknown as vscode.QuickPickItem); // solution

      const controller = new CritiqueCommentController(
        cache as unknown as DataCache,
        cli as unknown as JjjCli,
      );
      const tempThread = makeTempThread();

      await controller.replyToCritique({
        thread: tempThread as unknown as vscode.CommentThread,
        text: "Issue",
      });

      assert.strictEqual(qpStub.callCount, 2);
      assert.strictEqual((cli.newCritique.firstCall.args as string[])[0], "s1");
    });

    it("disposes temp thread when solution quick pick is cancelled", async () => {
      cache.getCritiquesWithLocations.returns([]);
      cache.getStatus.returns(null);
      cache.getSolutions.returns([
        makeSolution({ id: "s1", status: "testing" }),
        makeSolution({ id: "s2", status: "testing" }),
      ]);

      const qpStub = sinon.stub(vscode.window, "showQuickPick");
      qpStub.onFirstCall().resolves("low" as unknown as vscode.QuickPickItem); // severity
      qpStub.onSecondCall().resolves(undefined); // solution cancelled

      const controller = new CritiqueCommentController(
        cache as unknown as DataCache,
        cli as unknown as JjjCli,
      );
      const tempThread = makeTempThread();

      await controller.replyToCritique({
        thread: tempThread as unknown as vscode.CommentThread,
        text: "Issue",
      });

      sinon.assert.calledOnce(tempThread.dispose);
      sinon.assert.notCalled(cli.newCritique);
    });

    it("active solution takes priority over candidates — no solution quick pick shown", async () => {
      cache.getCritiquesWithLocations.returns([]);
      cache.getStatus.returns({
        active_solution: { id: "s-active", title: "Active", problem_id: "p1", status: "testing" },
        items: [], total_count: 0, user: "test",
        summary: { open_problems: 0, testing_solutions: 0, open_critiques: 0 },
      });
      // Even with multiple solutions available, active_solution should win
      cache.getSolutions.returns([
        makeSolution({ id: "s1", status: "testing" }),
        makeSolution({ id: "s2", status: "testing" }),
      ]);
      cli.newCritique.resolves("nid");

      const qpStub = sinon.stub(vscode.window, "showQuickPick")
        .resolves("high" as unknown as vscode.QuickPickItem);

      const controller = new CritiqueCommentController(
        cache as unknown as DataCache,
        cli as unknown as JjjCli,
      );
      const tempThread = makeTempThread();

      await controller.replyToCritique({
        thread: tempThread as unknown as vscode.CommentThread,
        text: "Big issue",
      });

      // Only severity pick — solution resolved from active_solution
      assert.strictEqual(qpStub.callCount, 1);
      assert.strictEqual((cli.newCritique.firstCall.args as string[])[0], "s-active");
    });
  });

  describe("dispose", () => {
    it("disposes all threads and the controller", () => {
      cache.getCritiquesWithLocations.returns([
        makeCritique({ id: "c1" }),
        makeCritique({ id: "c2", file_path: "src/bar.rs", line_start: 20 }),
      ]);
      const controller = new CritiqueCommentController(
        cache as unknown as DataCache,
        cli as unknown as JjjCli,
      );

      assert.strictEqual(mockController._threads.length, 2);
      controller.dispose();

      for (const t of mockController._threads) {
        sinon.assert.calledOnce(t.dispose);
      }
      sinon.assert.calledOnce(mockController.dispose);
    });
  });

  // Verify the controller is created with the right ID
  it("creates comment controller with id 'jjj-critiques'", () => {
    new CritiqueCommentController(cache as unknown as DataCache, cli as unknown as JjjCli);
    sinon.assert.calledOnceWithExactly(createControllerStub, "jjj-critiques", "jjj Critiques");
  });
});
