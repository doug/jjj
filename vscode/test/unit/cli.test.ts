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

    it("status passes --all flag when requested", async () => {
      execStub.resolves('{"active_solution":null,"items":[],"total_count":0,"user":"test","summary":{"open_problems":0,"testing_solutions":0,"open_critiques":0}}');
      await cli.status(true);
      assert.deepStrictEqual(execStub.firstCall.args[0], ["status", "--json", "--all"]);
    });

    it("status omits --all flag by default", async () => {
      execStub.resolves('{"active_solution":null,"items":[],"total_count":0,"user":"test","summary":{"open_problems":0,"testing_solutions":0,"open_critiques":0}}');
      await cli.status(false);
      assert.deepStrictEqual(execStub.firstCall.args[0], ["status", "--json"]);
    });

    it("showProblem passes id with --json", async () => {
      execStub.resolves('{"id":"p1"}');
      await cli.showProblem("p1");
      assert.deepStrictEqual(execStub.firstCall.args[0], ["problem", "show", "p1", "--json"]);
    });
  });

  describe("mutations", () => {
    it("newProblem passes title", async () => {
      execStub.resolves("p1");
      await cli.newProblem("Fix the bug");
      assert.deepStrictEqual(execStub.firstCall.args[0], ["problem", "new", "Fix the bug"]);
    });

    it("newSolution passes title and --problem flag", async () => {
      execStub.resolves("s1");
      await cli.newSolution("Add index", "p1");
      assert.deepStrictEqual(execStub.firstCall.args[0], ["solution", "new", "Add index", "--problem", "p1"]);
    });

    it("newCritique includes severity, file, and line", async () => {
      execStub.resolves("c1");
      await cli.newCritique("s1", "SQL injection", "high", "src/db.rs", 42);
      assert.deepStrictEqual(execStub.firstCall.args[0], [
        "critique", "new", "s1", "SQL injection",
        "--severity", "high", "--file", "src/db.rs", "--line", "42",
      ]);
    });

    it("newCritique without optional file/line", async () => {
      execStub.resolves("c1");
      await cli.newCritique("s1", "Missing tests", "medium");
      assert.deepStrictEqual(execStub.firstCall.args[0], [
        "critique", "new", "s1", "Missing tests", "--severity", "medium",
      ]);
    });

    it("acceptSolution with force flag", async () => {
      execStub.resolves("ok");
      await cli.acceptSolution("s1", true);
      assert.deepStrictEqual(execStub.firstCall.args[0], ["solution", "accept", "s1", "--force"]);
    });

    it("acceptSolution without force", async () => {
      execStub.resolves("ok");
      await cli.acceptSolution("s1");
      assert.deepStrictEqual(execStub.firstCall.args[0], ["solution", "accept", "s1"]);
    });

    it("assignProblem uses --to flag", async () => {
      execStub.resolves("ok");
      await cli.assignProblem("p1", "alice");
      assert.deepStrictEqual(execStub.firstCall.args[0], ["problem", "assign", "p1", "--to", "alice"]);
    });

    it("assignSolution uses --to flag", async () => {
      execStub.resolves("ok");
      await cli.assignSolution("s1", "bob");
      assert.deepStrictEqual(execStub.firstCall.args[0], ["solution", "assign", "s1", "--to", "bob"]);
    });

    it("resumeSolution passes solution id", async () => {
      execStub.resolves("ok");
      await cli.resumeSolution("s1");
      assert.deepStrictEqual(execStub.firstCall.args[0], ["solution", "resume", "s1"]);
    });

    it("milestoneAddProblem passes correct ids", async () => {
      execStub.resolves("ok");
      await cli.milestoneAddProblem("m1", "p3");
      assert.deepStrictEqual(execStub.firstCall.args[0], ["milestone", "add-problem", "m1", "p3"]);
    });

    it("milestoneRemoveProblem passes correct ids", async () => {
      execStub.resolves("ok");
      await cli.milestoneRemoveProblem("m2", "p5");
      assert.deepStrictEqual(execStub.firstCall.args[0], ["milestone", "remove-problem", "m2", "p5"]);
    });
  });

  describe("JSON parsing", () => {
    it("parses JSON response from execJson", async () => {
      execStub.resolves('[{"id":"p1","title":"Test"}]');
      const result = await cli.listProblems();
      assert.strictEqual(result.length, 1);
      assert.strictEqual(result[0].id, "p1");
    });

    it("rejects on invalid JSON", async () => {
      execStub.resolves("not json");
      await assert.rejects(() => cli.listProblems());
    });
  });
});
