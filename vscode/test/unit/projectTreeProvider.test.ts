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
