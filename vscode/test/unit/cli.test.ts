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
      execStub.resolves('{"active_solution":null,"items":[],"total_count":0,"user":"test","summary":{"open_problems":0,"review_solutions":0,"open_critiques":0}}');
      await cli.status(true);
      assert.deepStrictEqual(execStub.firstCall.args[0], ["status", "--json", "--all"]);
    });

    it("status omits --all flag by default", async () => {
      execStub.resolves('{"active_solution":null,"items":[],"total_count":0,"user":"test","summary":{"open_problems":0,"review_solutions":0,"open_critiques":0}}');
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

    it("approveSolution with force flag", async () => {
      execStub.resolves("ok");
      await cli.approveSolution("s1", true);
      assert.deepStrictEqual(execStub.firstCall.args[0], ["solution", "approve", "s1", "--force"]);
    });

    it("approveSolution without force", async () => {
      execStub.resolves("ok");
      await cli.approveSolution("s1");
      assert.deepStrictEqual(execStub.firstCall.args[0], ["solution", "approve", "s1"]);
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

  describe("error handling", () => {
    it("propagates errors from exec to callers", async () => {
      execStub.rejects(new Error("something went wrong"));
      await assert.rejects(() => cli.newProblem("test"), { message: "something went wrong" });
    });

    it("propagates errors with stderr message", async () => {
      execStub.rejects(new Error("command not found"));
      await assert.rejects(() => cli.listProblems(), { message: "command not found" });
    });
  });

  describe("remaining mutations", () => {
    it("submitSolution passes correct args", async () => {
      execStub.resolves("ok");
      await cli.submitSolution("s1");
      assert.deepStrictEqual(execStub.firstCall.args[0], ["solution", "submit", "s1"]);
    });

    it("withdrawSolution passes correct args", async () => {
      execStub.resolves("ok");
      await cli.withdrawSolution("s1");
      assert.deepStrictEqual(execStub.firstCall.args[0], ["solution", "withdraw", "s1"]);
    });

    it("lgtmSolution passes correct args", async () => {
      execStub.resolves("ok");
      await cli.lgtmSolution("s1");
      assert.deepStrictEqual(execStub.firstCall.args[0], ["solution", "lgtm", "s1"]);
    });

    it("solveProblem passes correct args", async () => {
      execStub.resolves("ok");
      await cli.solveProblem("p1");
      assert.deepStrictEqual(execStub.firstCall.args[0], ["problem", "solve", "p1"]);
    });

    it("dissolveProblem passes correct args", async () => {
      execStub.resolves("ok");
      await cli.dissolveProblem("p1");
      assert.deepStrictEqual(execStub.firstCall.args[0], ["problem", "dissolve", "p1"]);
    });

    it("newMilestone passes correct args", async () => {
      execStub.resolves("ok");
      await cli.newMilestone("Q1 Release");
      assert.deepStrictEqual(execStub.firstCall.args[0], ["milestone", "new", "Q1 Release"]);
    });

    it("addressCritique passes correct args", async () => {
      execStub.resolves("ok");
      await cli.addressCritique("c1");
      assert.deepStrictEqual(execStub.firstCall.args[0], ["critique", "address", "c1"]);
    });

    it("dismissCritique passes correct args", async () => {
      execStub.resolves("ok");
      await cli.dismissCritique("c1");
      assert.deepStrictEqual(execStub.firstCall.args[0], ["critique", "dismiss", "c1"]);
    });

    it("validateCritique passes correct args", async () => {
      execStub.resolves("ok");
      await cli.validateCritique("c1");
      assert.deepStrictEqual(execStub.firstCall.args[0], ["critique", "validate", "c1"]);
    });

    it("replyCritique passes correct args", async () => {
      execStub.resolves("ok");
      await cli.replyCritique("c1", "I agree");
      assert.deepStrictEqual(execStub.firstCall.args[0], ["critique", "reply", "c1", "I agree"]);
    });

    it("reopenProblem passes correct args", async () => {
      execStub.resolves("ok");
      await cli.reopenProblem("p1");
      assert.deepStrictEqual(execStub.firstCall.args[0], ["problem", "reopen", "p1"]);
    });

    it("editProblem passes correct args", async () => {
      execStub.resolves("ok");
      await cli.editProblem("p1", "New title");
      assert.deepStrictEqual(execStub.firstCall.args[0], ["problem", "edit", "p1", "--title", "New title"]);
    });

    it("editSolution passes correct args", async () => {
      execStub.resolves("ok");
      await cli.editSolution("s1", "Better title");
      assert.deepStrictEqual(execStub.firstCall.args[0], ["solution", "edit", "s1", "--title", "Better title"]);
    });

    it("editProblemTags passes correct args", async () => {
      execStub.resolves("ok");
      await cli.editProblemTags("p1", ["bug", "urgent"]);
      assert.deepStrictEqual(execStub.firstCall.args[0], ["problem", "edit", "p1", "--set-tags", "bug,urgent"]);
    });

    it("editSolutionTags passes correct args", async () => {
      execStub.resolves("ok");
      await cli.editSolutionTags("s1", ["refactor", "v2"]);
      assert.deepStrictEqual(execStub.firstCall.args[0], ["solution", "edit", "s1", "--set-tags", "refactor,v2"]);
    });

    it("listTags passes correct args", async () => {
      execStub.resolves('[{"tag":"bug","count":3}]');
      await cli.listTags();
      assert.deepStrictEqual(execStub.firstCall.args[0], ["tags", "--json"]);
    });
  });

  describe("github sync", () => {
    it("syncGithub passes correct args", async () => {
      execStub.resolves("ok");
      await cli.syncGithub();
      assert.deepStrictEqual(execStub.firstCall.args[0], ["github"]);
    });

    it("syncGithubImport passes issue number", async () => {
      execStub.resolves("ok");
      await cli.syncGithubImport("42");
      assert.deepStrictEqual(execStub.firstCall.args[0], ["github", "import", "42"]);
    });

    it("syncGithubStatus passes correct args", async () => {
      execStub.resolves("ok");
      await cli.syncGithubStatus();
      assert.deepStrictEqual(execStub.firstCall.args[0], ["github", "status"]);
    });
  });

  describe("show methods", () => {
    it("showSolution passes correct args", async () => {
      execStub.resolves('{"id":"s1"}');
      await cli.showSolution("s1");
      assert.deepStrictEqual(execStub.firstCall.args[0], ["solution", "show", "s1", "--json"]);
    });

    it("showCritique passes correct args", async () => {
      execStub.resolves('{"id":"c1"}');
      await cli.showCritique("c1");
      assert.deepStrictEqual(execStub.firstCall.args[0], ["critique", "show", "c1", "--json"]);
    });

    it("showMilestone passes correct args", async () => {
      execStub.resolves('{"id":"m1"}');
      await cli.showMilestone("m1");
      assert.deepStrictEqual(execStub.firstCall.args[0], ["milestone", "show", "m1", "--json"]);
    });
  });

  describe("newSolution with reviewers", () => {
    it("newSolution includes reviewers", async () => {
      execStub.resolves("ok");
      await cli.newSolution("Fix", "p1", ["alice", "bob"]);
      assert.deepStrictEqual(execStub.firstCall.args[0],
        ["solution", "new", "Fix", "--problem", "p1", "--reviewer", "alice", "--reviewer", "bob"]);
    });
  });

  describe("newCritique with reviewer", () => {
    it("newCritique includes reviewer", async () => {
      execStub.resolves("ok");
      await cli.newCritique("s1", "Issue", "high", "src/foo.rs", 10, "alice");
      assert.deepStrictEqual(execStub.firstCall.args[0],
        ["critique", "new", "s1", "Issue", "--severity", "high", "--file", "src/foo.rs", "--line", "10", "--reviewer", "alice"]);
    });
  });
});
