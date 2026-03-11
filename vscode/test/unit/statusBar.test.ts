import * as assert from "assert";
import * as sinon from "sinon";
import * as vscode from "vscode";
import { NextActionStatusBar, SolutionStatusBar } from "../../src/statusBar";

function makeMockStatusBarItem() {
  return {
    text: "",
    command: "",
    tooltip: "" as string | undefined,
    show: sinon.stub(),
    hide: sinon.stub(),
    dispose: sinon.stub(),
  };
}

function makeMockCache() {
  let _cb: (() => void) | undefined;
  return {
    onDidChange: sinon.stub().callsFake((cb: () => void) => {
      _cb = cb;
      return { dispose: sinon.stub() };
    }),
    getStatus: sinon.stub().returns(null),
    getCritiques: sinon.stub().returns([]),
    _fireCacheChange: () => { if (_cb) _cb(); },
  };
}

function makeSummary(open_problems = 0, review_solutions = 0, open_critiques = 0) {
  return { open_problems, review_solutions, open_critiques };
}

function makeStatusResult(overrides: Record<string, unknown> = {}) {
  return {
    active_solution: null,
    items: [],
    total_count: 0,
    user: "test",
    summary: makeSummary(),
    ...overrides,
  };
}

describe("NextActionStatusBar", () => {
  let mockItem: ReturnType<typeof makeMockStatusBarItem>;
  let mockCache: ReturnType<typeof makeMockCache>;
  let createStub: sinon.SinonStub;

  beforeEach(() => {
    mockItem = makeMockStatusBarItem();
    mockCache = makeMockCache();
    createStub = sinon.stub(vscode.window, "createStatusBarItem").returns(mockItem as unknown as vscode.StatusBarItem);
  });

  afterEach(() => {
    sinon.restore();
  });

  it("hides when no status data", () => {
    mockCache.getStatus.returns(null);
    new NextActionStatusBar(mockCache as never);
    assert.ok(mockItem.hide.called, "expected hide() to be called");
    assert.ok(!mockItem.show.called, "expected show() not to be called");
  });

  it("shows open problems count", () => {
    mockCache.getStatus.returns(makeStatusResult({ summary: makeSummary(3, 0, 0) }));
    new NextActionStatusBar(mockCache as never);
    assert.strictEqual(mockItem.text, "$(issues) 3");
    assert.ok(mockItem.show.called);
  });

  it("shows review solutions count", () => {
    mockCache.getStatus.returns(makeStatusResult({ summary: makeSummary(0, 2, 0) }));
    new NextActionStatusBar(mockCache as never);
    assert.strictEqual(mockItem.text, "$(beaker) 2");
    assert.ok(mockItem.show.called);
  });

  it("shows open critiques count", () => {
    mockCache.getStatus.returns(makeStatusResult({ summary: makeSummary(0, 0, 5) }));
    new NextActionStatusBar(mockCache as never);
    assert.strictEqual(mockItem.text, "$(warning) 5");
    assert.ok(mockItem.show.called);
  });

  it("shows combined segments", () => {
    mockCache.getStatus.returns(makeStatusResult({ summary: makeSummary(2, 1, 4) }));
    new NextActionStatusBar(mockCache as never);
    assert.strictEqual(mockItem.text, "$(issues) 2  $(beaker) 1  $(warning) 4");
    assert.ok(mockItem.show.called);
  });

  it("hides when all counts are zero", () => {
    mockCache.getStatus.returns(makeStatusResult({ summary: makeSummary(0, 0, 0) }));
    new NextActionStatusBar(mockCache as never);
    assert.ok(mockItem.hide.called);
    assert.ok(!mockItem.show.called);
  });

  it("updates when cache fires onDidChange", () => {
    mockCache.getStatus.returns(null);
    new NextActionStatusBar(mockCache as never);
    assert.ok(mockItem.hide.called);
    mockItem.hide.resetHistory();
    mockItem.show.resetHistory();

    // Now change the status and fire the event
    mockCache.getStatus.returns(makeStatusResult({ summary: makeSummary(1, 0, 0) }));
    mockCache._fireCacheChange();

    assert.strictEqual(mockItem.text, "$(issues) 1");
    assert.ok(mockItem.show.called);
  });

  it("correct tooltip text with pluralization", () => {
    // Singular forms
    mockCache.getStatus.returns(makeStatusResult({ summary: makeSummary(1, 1, 1) }));
    new NextActionStatusBar(mockCache as never);
    assert.strictEqual(mockItem.tooltip, "1 open problem \u00b7 1 solution in review \u00b7 1 open critique");

    // Plural forms
    mockCache.getStatus.returns(makeStatusResult({ summary: makeSummary(3, 2, 5) }));
    mockCache._fireCacheChange();
    assert.strictEqual(mockItem.tooltip, "3 open problems \u00b7 2 solutions in review \u00b7 5 open critiques");
  });

  it("command is set to jjj.nextOpenItem", () => {
    mockCache.getStatus.returns(null);
    new NextActionStatusBar(mockCache as never);
    assert.strictEqual(mockItem.command, "jjj.nextOpenItem");
  });
});

describe("SolutionStatusBar", () => {
  let mockItem: ReturnType<typeof makeMockStatusBarItem>;
  let mockCache: ReturnType<typeof makeMockCache>;
  let createStub: sinon.SinonStub;

  beforeEach(() => {
    mockItem = makeMockStatusBarItem();
    mockCache = makeMockCache();
    createStub = sinon.stub(vscode.window, "createStatusBarItem").returns(mockItem as unknown as vscode.StatusBarItem);
  });

  afterEach(() => {
    sinon.restore();
  });

  it("hides when no active solution", () => {
    mockCache.getStatus.returns(makeStatusResult({ active_solution: null }));
    new SolutionStatusBar(mockCache as never);
    assert.ok(mockItem.hide.called, "expected hide() to be called");
    assert.ok(!mockItem.show.called, "expected show() not to be called");
  });

  it("shows active solution title with lightbulb icon", () => {
    mockCache.getStatus.returns(makeStatusResult({
      active_solution: { id: "s1", title: "Fix login", problem_id: "p1", status: "proposed" },
    }));
    mockCache.getCritiques.returns([]);
    new SolutionStatusBar(mockCache as never);
    assert.strictEqual(mockItem.text, "$(lightbulb) Fix login");
    assert.ok(mockItem.show.called);
  });

  it("shows beaker icon for submitted solution", () => {
    mockCache.getStatus.returns(makeStatusResult({
      active_solution: { id: "s1", title: "Fix login", problem_id: "p1", status: "submitted" },
    }));
    mockCache.getCritiques.returns([]);
    new SolutionStatusBar(mockCache as never);
    assert.strictEqual(mockItem.text, "$(beaker) Fix login");
    assert.ok(mockItem.show.called);
  });

  it("shows critique badge count", () => {
    mockCache.getStatus.returns(makeStatusResult({
      active_solution: { id: "s1", title: "Fix login", problem_id: "p1", status: "proposed" },
    }));
    mockCache.getCritiques.returns([
      { id: "c1", solution_id: "s1", status: "open" },
      { id: "c2", solution_id: "s1", status: "valid" },
      { id: "c3", solution_id: "s1", status: "addressed" },  // should be excluded
      { id: "c4", solution_id: "s2", status: "open" },       // different solution, excluded
    ]);
    new SolutionStatusBar(mockCache as never);
    assert.strictEqual(mockItem.text, "$(lightbulb) Fix login $(warning)2");
  });

  it("no badge when no open critiques", () => {
    mockCache.getStatus.returns(makeStatusResult({
      active_solution: { id: "s1", title: "Fix login", problem_id: "p1", status: "proposed" },
    }));
    mockCache.getCritiques.returns([
      { id: "c1", solution_id: "s1", status: "addressed" },
      { id: "c2", solution_id: "s2", status: "open" },
    ]);
    new SolutionStatusBar(mockCache as never);
    assert.strictEqual(mockItem.text, "$(lightbulb) Fix login");
  });

  it("updates when cache fires onDidChange", () => {
    mockCache.getStatus.returns(makeStatusResult({ active_solution: null }));
    new SolutionStatusBar(mockCache as never);
    assert.ok(mockItem.hide.called);
    mockItem.hide.resetHistory();
    mockItem.show.resetHistory();

    // Now set an active solution and fire the event
    mockCache.getStatus.returns(makeStatusResult({
      active_solution: { id: "s1", title: "New feature", problem_id: "p1", status: "proposed" },
    }));
    mockCache.getCritiques.returns([]);
    mockCache._fireCacheChange();

    assert.strictEqual(mockItem.text, "$(lightbulb) New feature");
    assert.ok(mockItem.show.called);
  });

  it("command is set to jjj.openActiveSolution", () => {
    mockCache.getStatus.returns(null);
    new SolutionStatusBar(mockCache as never);
    assert.strictEqual(mockItem.command, "jjj.openActiveSolution");
  });
});
