# VS Code Unified Tree View Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace the two-panel VS Code UI (Next Actions + Project Tree) with a single unified tree that supports filtering and Tab navigation.

**Architecture:** Single `ProjectTreeProvider` with filter mode state. Filter toggle via header button + keyboard shortcut. Tab/Shift+Tab navigation commands that find next/prev open items in depth-first tree order.

**Tech Stack:** TypeScript, VS Code Extension API, Mocha tests

---

## Task 1: Add filter state and toggle to ProjectTreeProvider

**Files:**
- Modify: `vscode/src/views/projectTreeProvider.ts`
- Test: `vscode/test/projectTreeProvider.test.ts`

**Step 1: Write failing test for filter mode toggle**

Create `vscode/test/projectTreeProvider.test.ts`:

```typescript
import * as assert from "assert";
import { ProjectTreeProvider } from "../src/views/projectTreeProvider";

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
```

**Step 2: Run test to verify it fails**

Run: `cd vscode && npm test`
Expected: FAIL - `filterMode` property doesn't exist

**Step 3: Add filterMode property and toggleFilter method**

In `vscode/src/views/projectTreeProvider.ts`, add after line 91 (inside class):

```typescript
  private _filterMode: "all" | "open" = "open";

  get filterMode(): "all" | "open" {
    return this._filterMode;
  }

  toggleFilter(): void {
    this._filterMode = this._filterMode === "all" ? "open" : "all";
    this._onDidChangeTreeData.fire(undefined);
  }
```

**Step 4: Run test to verify it passes**

Run: `cd vscode && npm test`
Expected: PASS

**Step 5: Commit**

```bash
cd vscode && git add src/views/projectTreeProvider.ts test/projectTreeProvider.test.ts
git commit -m "feat(vscode): add filter mode state to ProjectTreeProvider"
```

---

## Task 2: Implement filtering logic in getChildren

**Files:**
- Modify: `vscode/src/views/projectTreeProvider.ts`
- Modify: `vscode/test/projectTreeProvider.test.ts`

**Step 1: Write failing test for filtering**

Add to `vscode/test/projectTreeProvider.test.ts`:

```typescript
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
```

**Step 2: Run test to verify it fails**

Run: `cd vscode && npm test`
Expected: FAIL - filtering not implemented

**Step 3: Add helper functions and update getChildren**

In `vscode/src/views/projectTreeProvider.ts`, add helper functions before the class:

```typescript
function isOpenProblem(p: Problem): boolean {
  return p.status !== "solved" && p.status !== "dissolved";
}

function isOpenSolution(s: Solution): boolean {
  return s.status !== "accepted" && s.status !== "refuted";
}

function isOpenCritique(c: Critique): boolean {
  return c.status === "open";
}
```

Then update `getChildren` method to filter based on mode:

```typescript
  getChildren(element?: TreeNode): TreeNode[] {
    if (!element) {
      // Root: milestones + backlog
      const milestones = this.cache.getMilestones();
      const nodes: TreeNode[] = [];

      for (const m of milestones) {
        const problems = this.cache.getProblemsForMilestone(m.id);
        const openProblems = this._filterMode === "open"
          ? problems.filter(isOpenProblem)
          : problems;

        // Skip milestone if no open problems in open mode
        if (this._filterMode === "open" && openProblems.length === 0) {
          continue;
        }

        const solved = problems.filter(p => p.status === "solved").length;
        nodes.push(new MilestoneNode(m, problems.length, solved));
      }

      const backlog = this.cache.getBacklogProblems();
      const openBacklog = this._filterMode === "open"
        ? backlog.filter(isOpenProblem)
        : backlog;

      if (this._filterMode === "all" || openBacklog.length > 0) {
        nodes.push(new MilestoneNode(null, backlog.length, 0));
      }

      return nodes;
    }

    if (element instanceof MilestoneNode) {
      const problems = element.milestone
        ? this.cache.getProblemsForMilestone(element.milestone.id)
        : this.cache.getBacklogProblems();

      const filtered = this._filterMode === "open"
        ? problems.filter(isOpenProblem)
        : problems;

      return filtered.map(p => new ProblemNode(p));
    }

    if (element instanceof ProblemNode) {
      const solutions = this.cache.getSolutionsForProblem(element.problem.id);
      const filtered = this._filterMode === "open"
        ? solutions.filter(isOpenSolution)
        : solutions;

      return filtered.map(s => {
        const critiques = this.cache.getCritiquesForSolution(s.id);
        return new SolutionNode(s, critiques.filter(c => c.status === "open").length);
      });
    }

    if (element instanceof SolutionNode) {
      const critiques = this.cache.getCritiquesForSolution(element.solution.id);
      const filtered = this._filterMode === "open"
        ? critiques.filter(isOpenCritique)
        : critiques;

      return filtered.map(c => new CritiqueNode(c));
    }

    return [];
  }
```

**Step 4: Run test to verify it passes**

Run: `cd vscode && npm test`
Expected: PASS

**Step 5: Commit**

```bash
cd vscode && git add src/views/projectTreeProvider.ts test/projectTreeProvider.test.ts
git commit -m "feat(vscode): implement filtering in ProjectTreeProvider"
```

---

## Task 3: Add commands and keybindings to package.json

**Files:**
- Modify: `vscode/package.json`

**Step 1: Add new commands**

In `vscode/package.json`, add to the `commands` array:

```json
{ "command": "jjj.toggleTreeFilter", "title": "JJJ: Toggle Filter", "icon": "$(filter)" },
{ "command": "jjj.nextOpenItem", "title": "JJJ: Next Open Item" },
{ "command": "jjj.prevOpenItem", "title": "JJJ: Previous Open Item" }
```

**Step 2: Remove jjj-next-actions view and update jjj-project-tree**

Replace the `views` section:

```json
"views": {
  "jjj-sidebar": [
    { "id": "jjj-project-tree", "name": "Project" }
  ]
}
```

**Step 3: Add keybindings**

Add a new `keybindings` section after `menus`:

```json
"keybindings": [
  {
    "command": "jjj.toggleTreeFilter",
    "key": "cmd+shift+o",
    "mac": "cmd+shift+o",
    "win": "ctrl+shift+o",
    "linux": "ctrl+shift+o"
  },
  {
    "command": "jjj.nextOpenItem",
    "key": "tab",
    "when": "focusedView == jjj-project-tree"
  },
  {
    "command": "jjj.prevOpenItem",
    "key": "shift+tab",
    "when": "focusedView == jjj-project-tree"
  }
]
```

**Step 4: Update view/title menu**

Update the `view/title` section in `menus`:

```json
"view/title": [
  { "command": "jjj.toggleTreeFilter", "when": "view == jjj-project-tree", "group": "navigation@1" },
  { "command": "jjj.refreshAll", "when": "view == jjj-project-tree", "group": "navigation@2" },
  { "command": "jjj.newProblem", "when": "view == jjj-project-tree", "group": "navigation@3" },
  { "command": "jjj.newMilestone", "when": "view == jjj-project-tree", "group": "navigation@4" }
]
```

**Step 5: Verify JSON is valid**

Run: `cd vscode && node -e "require('./package.json')"`
Expected: No error

**Step 6: Commit**

```bash
cd vscode && git add package.json
git commit -m "feat(vscode): add commands and keybindings for unified tree"
```

---

## Task 4: Register commands in extension.ts

**Files:**
- Modify: `vscode/src/extension.ts`

**Step 1: Remove NextActionsProvider and StatusBar imports and registrations**

In `vscode/src/extension.ts`:

Remove these imports (lines 4 and 7):
```typescript
import { NextActionsProvider } from "./views/nextActionsProvider";
import { StatusBar } from "./statusBar";
```

Remove these lines from activate function:
```typescript
  const nextActions = new NextActionsProvider(cache);
  vscode.window.registerTreeDataProvider("jjj-next-actions", nextActions);
  context.subscriptions.push(nextActions);
```

And:
```typescript
  const statusBar = new StatusBar(cache);
  context.subscriptions.push(statusBar);
```

**Step 2: Store treeView reference and register toggle command**

Update the projectTree section to store a reference and add toggle command:

```typescript
  const projectTree = new ProjectTreeProvider(cache, cli);
  const treeView = vscode.window.createTreeView("jjj-project-tree", {
    treeDataProvider: projectTree,
    dragAndDropController: projectTree,
    canSelectMany: true,
  });
  context.subscriptions.push(treeView, projectTree);

  // Filter toggle command
  context.subscriptions.push(
    vscode.commands.registerCommand("jjj.toggleTreeFilter", () => {
      projectTree.toggleFilter();
      // Update tree view title based on mode
      treeView.title = projectTree.filterMode === "open" ? "Project (Open)" : "Project";
    }),
  );

  // Tab navigation commands
  context.subscriptions.push(
    vscode.commands.registerCommand("jjj.nextOpenItem", async () => {
      const nextItem = projectTree.getNextOpenItem(treeView.selection[0]);
      if (nextItem) {
        await treeView.reveal(nextItem, { select: true, focus: true });
      }
    }),
    vscode.commands.registerCommand("jjj.prevOpenItem", async () => {
      const prevItem = projectTree.getPrevOpenItem(treeView.selection[0]);
      if (prevItem) {
        await treeView.reveal(prevItem, { select: true, focus: true });
      }
    }),
  );

  // Set initial title
  treeView.title = "Project (Open)";
```

**Step 3: Verify it compiles**

Run: `cd vscode && npm run compile`
Expected: Errors about missing methods (getNextOpenItem, getPrevOpenItem) - we'll add those next

**Step 4: Commit partial progress**

```bash
cd vscode && git add src/extension.ts
git commit -m "feat(vscode): register filter toggle and navigation commands"
```

---

## Task 5: Implement Tab navigation methods

**Files:**
- Modify: `vscode/src/views/projectTreeProvider.ts`
- Modify: `vscode/test/projectTreeProvider.test.ts`

**Step 1: Write failing tests for navigation**

Add to `vscode/test/projectTreeProvider.test.ts`:

```typescript
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
    assert.strictEqual(next, allItems[0]);
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
    assert.strictEqual(prev, allItems[allItems.length - 1]);
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
      ? [{ id: "s1", title: "Solution", status: "proposed" }]
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
```

**Step 2: Run test to verify it fails**

Run: `cd vscode && npm test`
Expected: FAIL - methods don't exist

**Step 3: Implement navigation methods**

Add to `vscode/src/views/projectTreeProvider.ts` inside the class:

```typescript
  /**
   * Get all open (actionable) items in depth-first order.
   * Milestones are skipped as they're containers, not actionable.
   */
  getAllOpenItems(): TreeNode[] {
    const items: TreeNode[] = [];
    const milestones = this.cache.getMilestones();

    for (const m of milestones) {
      const problems = this.cache.getProblemsForMilestone(m.id);
      for (const p of problems) {
        if (isOpenProblem(p)) {
          items.push(new ProblemNode(p));
        }
        const solutions = this.cache.getSolutionsForProblem(p.id);
        for (const s of solutions) {
          if (isOpenSolution(s)) {
            const critiques = this.cache.getCritiquesForSolution(s.id);
            items.push(new SolutionNode(s, critiques.filter(c => c.status === "open").length));
          }
          const critiques = this.cache.getCritiquesForSolution(s.id);
          for (const c of critiques) {
            if (isOpenCritique(c)) {
              items.push(new CritiqueNode(c));
            }
          }
        }
      }
    }

    // Backlog
    const backlog = this.cache.getBacklogProblems();
    for (const p of backlog) {
      if (isOpenProblem(p)) {
        items.push(new ProblemNode(p));
      }
      const solutions = this.cache.getSolutionsForProblem(p.id);
      for (const s of solutions) {
        if (isOpenSolution(s)) {
          const critiques = this.cache.getCritiquesForSolution(s.id);
          items.push(new SolutionNode(s, critiques.filter(c => c.status === "open").length));
        }
        const critiques = this.cache.getCritiquesForSolution(s.id);
        for (const c of critiques) {
          if (isOpenCritique(c)) {
            items.push(new CritiqueNode(c));
          }
        }
      }
    }

    return items;
  }

  private getItemId(node: TreeNode | undefined): string | undefined {
    if (!node) return undefined;
    if (node instanceof ProblemNode) return `p:${node.problem.id}`;
    if (node instanceof SolutionNode) return `s:${node.solution.id}`;
    if (node instanceof CritiqueNode) return `c:${node.critique.id}`;
    return undefined;
  }

  getNextOpenItem(current: TreeNode | undefined): TreeNode | undefined {
    const items = this.getAllOpenItems();
    if (items.length === 0) return undefined;

    if (!current) return items[0];

    const currentId = this.getItemId(current);
    const currentIndex = items.findIndex(item => this.getItemId(item) === currentId);

    if (currentIndex === -1) return items[0];

    const nextIndex = (currentIndex + 1) % items.length;
    return items[nextIndex];
  }

  getPrevOpenItem(current: TreeNode | undefined): TreeNode | undefined {
    const items = this.getAllOpenItems();
    if (items.length === 0) return undefined;

    if (!current) return items[items.length - 1];

    const currentId = this.getItemId(current);
    const currentIndex = items.findIndex(item => this.getItemId(item) === currentId);

    if (currentIndex === -1) return items[items.length - 1];

    const prevIndex = (currentIndex - 1 + items.length) % items.length;
    return items[prevIndex];
  }
```

**Step 4: Run test to verify it passes**

Run: `cd vscode && npm test`
Expected: PASS

**Step 5: Commit**

```bash
cd vscode && git add src/views/projectTreeProvider.ts test/projectTreeProvider.test.ts
git commit -m "feat(vscode): implement Tab navigation for open items"
```

---

## Task 6: Delete removed files

**Files:**
- Delete: `vscode/src/views/nextActionsProvider.ts`
- Delete: `vscode/src/statusBar.ts`

**Step 1: Delete the files**

```bash
cd vscode && rm src/views/nextActionsProvider.ts src/statusBar.ts
```

**Step 2: Verify compilation succeeds**

Run: `cd vscode && npm run compile`
Expected: PASS (no errors)

**Step 3: Verify tests pass**

Run: `cd vscode && npm test`
Expected: PASS

**Step 4: Commit**

```bash
cd vscode && git add -A
git commit -m "chore(vscode): remove NextActionsProvider and StatusBar"
```

---

## Task 7: Update dynamic filter icon

**Files:**
- Modify: `vscode/src/extension.ts`

**Step 1: Add setContext for dynamic icon**

The VS Code API requires using `setContext` to dynamically change menu icons. Update the toggle command in `extension.ts`:

```typescript
  // Set initial context for filter icon
  vscode.commands.executeCommand("setContext", "jjj.filterMode", "open");

  // Filter toggle command
  context.subscriptions.push(
    vscode.commands.registerCommand("jjj.toggleTreeFilter", () => {
      projectTree.toggleFilter();
      const mode = projectTree.filterMode;
      treeView.title = mode === "open" ? "Project (Open)" : "Project";
      vscode.commands.executeCommand("setContext", "jjj.filterMode", mode);
    }),
  );
```

**Step 2: Update package.json commands for dynamic icon**

In `vscode/package.json`, update the toggleTreeFilter command to use when clauses:

First, split into two commands in the commands array:
```json
{ "command": "jjj.toggleTreeFilter", "title": "JJJ: Show All", "icon": "$(filter)" }
```

Then update view/title menu to show different icons:
```json
"view/title": [
  {
    "command": "jjj.toggleTreeFilter",
    "when": "view == jjj-project-tree && jjj.filterMode == open",
    "group": "navigation@1"
  },
  {
    "command": "jjj.toggleTreeFilter",
    "when": "view == jjj-project-tree && jjj.filterMode == all",
    "group": "navigation@1"
  },
  { "command": "jjj.refreshAll", "when": "view == jjj-project-tree", "group": "navigation@2" },
  { "command": "jjj.newProblem", "when": "view == jjj-project-tree", "group": "navigation@3" },
  { "command": "jjj.newMilestone", "when": "view == jjj-project-tree", "group": "navigation@4" }
]
```

Actually, VS Code doesn't support dynamic icons in menus easily. Let's simplify - just use a static filter icon and rely on the title change to indicate mode.

**Step 3: Verify compilation**

Run: `cd vscode && npm run compile`
Expected: PASS

**Step 4: Commit**

```bash
cd vscode && git add src/extension.ts package.json
git commit -m "feat(vscode): add context for filter mode"
```

---

## Task 8: Final cleanup and lint

**Files:**
- All modified files

**Step 1: Run linter**

Run: `cd vscode && npm run lint`
Expected: No errors (fix any that appear)

**Step 2: Run all tests**

Run: `cd vscode && npm test`
Expected: All tests pass

**Step 3: Compile for final verification**

Run: `cd vscode && npm run compile`
Expected: No errors

**Step 4: Final commit if any lint fixes**

```bash
cd vscode && git add -A
git commit -m "chore(vscode): lint fixes" --allow-empty
```

---

## Task 9: Update test file imports

**Files:**
- Modify: `vscode/test/projectTreeProvider.test.ts`

**Step 1: Add tsconfig.test.json entry if needed**

Verify `vscode/tsconfig.test.json` includes the test directory. If tests fail to find modules, add:

```json
{
  "extends": "./tsconfig.json",
  "compilerOptions": {
    "outDir": "./out-test",
    "rootDir": "."
  },
  "include": ["src/**/*", "test/**/*"]
}
```

**Step 2: Run tests to verify**

Run: `cd vscode && npm test`
Expected: PASS

**Step 3: Commit if changes made**

```bash
cd vscode && git add -A
git commit -m "chore(vscode): update test config" --allow-empty
```

---

## Summary

After completing all tasks:
- Single unified "Project" tree view
- Toggle between "All" and "Open" modes via button or `Cmd+Shift+O`
- Tab/Shift+Tab navigates between open items
- NextActionsProvider and StatusBar removed
- All tests passing
