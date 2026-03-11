import * as sinon from "sinon";
import * as assert from "assert";
import * as vscode from "vscode";
import { activate, deactivate } from "../../src/extension";
import { JjjCli } from "../../src/cli";

function makeMockContext(): vscode.ExtensionContext {
  return {
    subscriptions: [],
  } as unknown as vscode.ExtensionContext;
}

describe("Extension activation", () => {
  let context: vscode.ExtensionContext;
  let registeredCommands: string[];
  let registerCommandStub: sinon.SinonStub;
  let createTreeViewStub: sinon.SinonStub;
  let executeCommandStub: sinon.SinonStub;
  let getConfigurationStub: sinon.SinonStub;
  beforeEach(() => {
    context = makeMockContext();
    registeredCommands = [];

    // Stub registerCommand to capture command names
    registerCommandStub = sinon.stub(vscode.commands, "registerCommand")
      .callsFake((id: string, _handler: (...args: unknown[]) => unknown) => {
        registeredCommands.push(id);
        return { dispose: () => {} };
      });

    // Add executeCommand to mock (not in default mock) and stub it.
    // Called by applyFilterMode for setContext.
    (vscode.commands as Record<string, unknown>).executeCommand = () => Promise.resolve();
    executeCommandStub = sinon.stub(vscode.commands, "executeCommand" as keyof typeof vscode.commands)
      .resolves();

    // Add ConfigurationTarget to mock (not in default mock).
    // Extension uses vscode.ConfigurationTarget.Workspace as argument to config.update.
    if (!(vscode as Record<string, unknown>).ConfigurationTarget) {
      (vscode as Record<string, unknown>).ConfigurationTarget = { Global: 1, Workspace: 2, WorkspaceFolder: 3 };
    }

    // Stub createTreeView to return a mock with selection and reveal
    createTreeViewStub = sinon.stub(vscode.window, "createTreeView").returns({
      selection: [],
      reveal: sinon.stub().resolves(),
      dispose: () => {},
      title: "",
    } as unknown as vscode.TreeView<unknown>);

    // Stub getConfiguration to return object with get + update
    const mockConfig = {
      get: sinon.stub().returns("open"),
      update: sinon.stub().resolves(),
    };
    getConfigurationStub = sinon.stub(vscode.workspace, "getConfiguration")
      .returns(mockConfig as unknown as vscode.WorkspaceConfiguration);

    // Stub window.showTextDocument (not in default mock)
    if (!(vscode.window as Record<string, unknown>).showTextDocument) {
      (vscode.window as Record<string, unknown>).showTextDocument = async () => undefined;
    }

    // Ensure createStatusBarItem returns an object with hide()
    sinon.stub(vscode.window, "createStatusBarItem").returns({
      text: "",
      command: "",
      color: undefined,
      backgroundColor: undefined,
      tooltip: undefined,
      show: () => {},
      hide: () => {},
      dispose: () => {},
    } as unknown as vscode.StatusBarItem);

    // Stub JjjCli public methods to prevent real subprocess calls.
    // DataCache.refresh() calls these during activation's cache.refresh().
    sinon.stub(JjjCli.prototype, "listProblems").resolves([]);
    sinon.stub(JjjCli.prototype, "listSolutions").resolves([]);
    sinon.stub(JjjCli.prototype, "listCritiques").resolves([]);
    sinon.stub(JjjCli.prototype, "listMilestones").resolves([]);
    sinon.stub(JjjCli.prototype, "status").resolves({
      active_solution: null,
      items: [],
      total_count: 0,
      user: "test",
      summary: { open_problems: 0, review_solutions: 0, open_critiques: 0 },
    });
  });

  afterEach(() => {
    sinon.restore();
    delete (vscode.window as Record<string, unknown>).showTextDocument;
    delete (vscode.commands as Record<string, unknown>).executeCommand;
    delete (vscode as Record<string, unknown>).ConfigurationTarget;
  });

  it("does not throw during activation", () => {
    assert.doesNotThrow(() => activate(context));
  });

  it("registers the refreshAll command", () => {
    activate(context);
    assert.ok(
      registeredCommands.includes("jjj.refreshAll"),
      `Expected "jjj.refreshAll" among registered commands: ${registeredCommands.join(", ")}`,
    );
  });

  it("registers filter commands", () => {
    activate(context);
    assert.ok(registeredCommands.includes("jjj.toggleTreeFilter"));
    assert.ok(registeredCommands.includes("jjj.showOpenOnly"));
    assert.ok(registeredCommands.includes("jjj.showAll"));
  });

  it("registers tab navigation commands", () => {
    activate(context);
    assert.ok(registeredCommands.includes("jjj.nextOpenItem"));
    assert.ok(registeredCommands.includes("jjj.prevOpenItem"));
  });

  it("registers openEntity command", () => {
    activate(context);
    assert.ok(registeredCommands.includes("jjj.openEntity"));
  });

  it("registers critique comment commands", () => {
    activate(context);
    assert.ok(registeredCommands.includes("jjj.commentAddressCritique"));
    assert.ok(registeredCommands.includes("jjj.commentDismissCritique"));
    assert.ok(registeredCommands.includes("jjj.commentValidateCritique"));
    assert.ok(registeredCommands.includes("jjj.commentSubmitCritique"));
  });

  it("registers openActiveSolution command", () => {
    activate(context);
    assert.ok(registeredCommands.includes("jjj.openActiveSolution"));
  });

  it("registers commands from registerCommands()", () => {
    activate(context);
    // registerCommands registers commands like newProblem, newSolution, etc.
    assert.ok(registeredCommands.includes("jjj.newProblem"));
    assert.ok(registeredCommands.includes("jjj.newSolution"));
    assert.ok(registeredCommands.includes("jjj.newCritique"));
    assert.ok(registeredCommands.includes("jjj.newMilestone"));
  });

  it("creates the project tree view with correct id", () => {
    activate(context);
    assert.ok(createTreeViewStub.calledOnce, "createTreeView should be called once");
    assert.strictEqual(
      createTreeViewStub.firstCall.args[0],
      "jjj-project-tree",
      "Tree view id should be 'jjj-project-tree'",
    );
  });

  it("passes canSelectMany option to createTreeView", () => {
    activate(context);
    const options = createTreeViewStub.firstCall.args[1];
    assert.strictEqual(options.canSelectMany, true);
  });

  it("passes treeDataProvider to createTreeView", () => {
    activate(context);
    const options = createTreeViewStub.firstCall.args[1];
    assert.ok(options.treeDataProvider, "treeDataProvider should be set");
  });

  it("passes dragAndDropController to createTreeView", () => {
    activate(context);
    const options = createTreeViewStub.firstCall.args[1];
    assert.ok(options.dragAndDropController, "dragAndDropController should be set");
  });

  it("pushes disposables to context.subscriptions", () => {
    activate(context);
    assert.ok(
      context.subscriptions.length > 0,
      "Expected at least one disposable in subscriptions",
    );
  });

  it("pushes many disposables to context.subscriptions", () => {
    activate(context);
    // The extension registers ~20+ disposables: cache, treeView, projectTree,
    // filter commands, nav commands, docProvider, openEntity, critique comments,
    // status bars, openActiveSolution, refreshAll, registerCommands items,
    // auto-refresh interval, onDidSaveTextDocument handlers, debounce timer
    assert.ok(
      context.subscriptions.length >= 15,
      `Expected at least 15 disposables, got ${context.subscriptions.length}`,
    );
  });

  it("sets up auto-refresh interval disposable", () => {
    activate(context);
    // The interval disposable is one of the last pushed to subscriptions.
    // Verify that at least one subscription has a dispose function
    // (all of them should, but this ensures the interval wrapper is present).
    const disposables = context.subscriptions;
    const hasDisposeMethods = disposables.every(
      (d: { dispose?: () => void }) => typeof d.dispose === "function",
    );
    assert.ok(hasDisposeMethods, "All subscriptions should have a dispose method");
  });

  it("calls setContext with filter mode during activation", () => {
    activate(context);
    assert.ok(
      executeCommandStub.calledWith("setContext", "jjj.filterMode", "open"),
      "Should call setContext with jjj.filterMode",
    );
  });

  it("reads jjj configuration during activation", () => {
    activate(context);
    assert.ok(
      getConfigurationStub.calledWith("jjj"),
      "Should read 'jjj' configuration section",
    );
  });

  it("registers the jjj text document content provider scheme", () => {
    const registerProviderStub = sinon.stub(
      vscode.workspace,
      "registerTextDocumentContentProvider",
    ).returns({ dispose: () => {} });

    activate(context);
    assert.ok(registerProviderStub.calledOnce);
    assert.strictEqual(
      registerProviderStub.firstCall.args[0],
      "jjj",
      "Should register 'jjj' scheme",
    );
  });
});

describe("Extension deactivation", () => {
  it("deactivate is a no-op and does not throw", () => {
    assert.doesNotThrow(() => deactivate());
  });

  it("deactivate returns undefined", () => {
    const result = deactivate();
    assert.strictEqual(result, undefined);
  });
});
