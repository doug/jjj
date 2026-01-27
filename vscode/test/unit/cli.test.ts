import * as assert from "assert";
import * as sinon from "sinon";
import { JjjCli } from "../../src/cli";

describe("JjjCli", () => {
  let cli: JjjCli;
  let execStub: sinon.SinonStub;

  beforeEach(() => {
    cli = new JjjCli();
    // Stub the private exec method on the instance
    execStub = sinon.stub(cli as any, "exec");
  });

  afterEach(() => {
    sinon.restore();
  });

  describe("queries", () => {
    it("listProblems passes correct args with --json", async () => {
      execStub.resolves("[]");
      await cli.listProblems();
      assert.deepStrictEqual(execStub.firstCall.args[0], ["problem", "list", "--json"]);
    });

    it("listSolutions passes correct args with --json", async () => {
      execStub.resolves("[]");
      await cli.listSolutions();
      assert.deepStrictEqual(execStub.firstCall.args[0], ["solution", "list", "--json"]);
    });

    it("listCritiques passes correct args with --json", async () => {
      execStub.resolves("[]");
      await cli.listCritiques();
      assert.deepStrictEqual(execStub.firstCall.args[0], ["critique", "list", "--json"]);
    });

    it("listMilestones passes correct args with --json", async () => {
      execStub.resolves("[]");
      await cli.listMilestones();
      assert.deepStrictEqual(execStub.firstCall.args[0], ["milestone", "list", "--json"]);
    });

    it("next passes --all flag when requested", async () => {
      execStub.resolves('{"items":[],"total_count":0,"user":"test"}');
      await cli.next(true);
      assert.deepStrictEqual(execStub.firstCall.args[0], ["next", "--json", "--all"]);
    });

    it("next omits --all flag by default", async () => {
      execStub.resolves('{"items":[],"total_count":0,"user":"test"}');
      await cli.next(false);
      assert.deepStrictEqual(execStub.firstCall.args[0], ["next", "--json"]);
    });

    it("showProblem passes id with --json", async () => {
      execStub.resolves('{"id":"P-1"}');
      await cli.showProblem("P-1");
      assert.deepStrictEqual(execStub.firstCall.args[0], ["problem", "show", "P-1", "--json"]);
    });
  });

  describe("mutations", () => {
    it("newProblem passes title", async () => {
      execStub.resolves("P-1");
      await cli.newProblem("Fix the bug");
      assert.deepStrictEqual(execStub.firstCall.args[0], ["problem", "new", "Fix the bug"]);
    });

    it("newSolution passes title and --problem flag", async () => {
      execStub.resolves("S-1");
      await cli.newSolution("Add index", "P-1");
      assert.deepStrictEqual(execStub.firstCall.args[0], ["solution", "new", "Add index", "--problem", "P-1"]);
    });

    it("newCritique includes severity, file, and line", async () => {
      execStub.resolves("CQ-1");
      await cli.newCritique("S-1", "SQL injection", "high", "src/db.rs", 42);
      assert.deepStrictEqual(execStub.firstCall.args[0], [
        "critique", "new", "S-1", "SQL injection",
        "--severity", "high", "--file", "src/db.rs", "--line", "42",
      ]);
    });

    it("newCritique without optional file/line", async () => {
      execStub.resolves("CQ-1");
      await cli.newCritique("S-1", "Missing tests", "medium");
      assert.deepStrictEqual(execStub.firstCall.args[0], [
        "critique", "new", "S-1", "Missing tests", "--severity", "medium",
      ]);
    });

    it("acceptSolution with force flag", async () => {
      execStub.resolves("ok");
      await cli.acceptSolution("S-1", true);
      assert.deepStrictEqual(execStub.firstCall.args[0], ["solution", "accept", "S-1", "--force"]);
    });

    it("acceptSolution without force", async () => {
      execStub.resolves("ok");
      await cli.acceptSolution("S-1");
      assert.deepStrictEqual(execStub.firstCall.args[0], ["solution", "accept", "S-1"]);
    });

    it("requestReview passes reviewers", async () => {
      execStub.resolves("ok");
      await cli.requestReview("S-1", ["@alice", "@bob"]);
      assert.deepStrictEqual(execStub.firstCall.args[0], ["solution", "review", "S-1", "@alice", "@bob"]);
    });

    it("assignProblem uses --to flag", async () => {
      execStub.resolves("ok");
      await cli.assignProblem("P-1", "alice");
      assert.deepStrictEqual(execStub.firstCall.args[0], ["problem", "assign", "P-1", "--to", "alice"]);
    });

    it("assignSolution uses --to flag", async () => {
      execStub.resolves("ok");
      await cli.assignSolution("S-1", "bob");
      assert.deepStrictEqual(execStub.firstCall.args[0], ["solution", "assign", "S-1", "--to", "bob"]);
    });

    it("startWorking with problem", async () => {
      execStub.resolves("ok");
      await cli.startWorking("New approach", "P-1");
      assert.deepStrictEqual(execStub.firstCall.args[0], ["start", "New approach", "--problem", "P-1"]);
    });

    it("startWorking without problem", async () => {
      execStub.resolves("ok");
      await cli.startWorking("S-1");
      assert.deepStrictEqual(execStub.firstCall.args[0], ["start", "S-1"]);
    });

    it("milestoneAddProblem passes correct ids", async () => {
      execStub.resolves("ok");
      await cli.milestoneAddProblem("M-1", "P-3");
      assert.deepStrictEqual(execStub.firstCall.args[0], ["milestone", "add-problem", "M-1", "P-3"]);
    });

    it("milestoneRemoveProblem passes correct ids", async () => {
      execStub.resolves("ok");
      await cli.milestoneRemoveProblem("M-2", "P-5");
      assert.deepStrictEqual(execStub.firstCall.args[0], ["milestone", "remove-problem", "M-2", "P-5"]);
    });
  });

  describe("JSON parsing", () => {
    it("parses JSON response from execJson", async () => {
      execStub.resolves('[{"id":"P-1","title":"Test"}]');
      const result = await cli.listProblems();
      assert.strictEqual(result.length, 1);
      assert.strictEqual(result[0].id, "P-1");
    });

    it("rejects on invalid JSON", async () => {
      execStub.resolves("not json");
      await assert.rejects(() => cli.listProblems());
    });
  });
});
