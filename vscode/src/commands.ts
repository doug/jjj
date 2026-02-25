import * as vscode from "vscode";
import { JjjCli } from "./cli";
import { DataCache } from "./cache";

export function registerCommands(
  context: vscode.ExtensionContext,
  cli: JjjCli,
  cache: DataCache,
) {
  const register = (name: string, handler: () => Promise<void>) => {
    context.subscriptions.push(
      vscode.commands.registerCommand(name, async () => {
        try {
          await handler();
          await cache.refresh();
        } catch (e: unknown) {
          const message = e instanceof Error ? e.message : String(e);
          vscode.window.showErrorMessage(`JJJ: ${message}`);
        }
      }),
    );
  };

  // --- Problem ---

  register("jjj.newProblem", async () => {
    const title = await vscode.window.showInputBox({ prompt: "Problem title" });
    if (!title) { return; }
    const result = await cli.newProblem(title);
    vscode.window.showInformationMessage(result);
  });

  register("jjj.solveProblem", async () => {
    const problems = cache.getProblems().filter(p => p.status === "open" || p.status === "in_progress");
    const pick = await vscode.window.showQuickPick(
      problems.map(p => ({ label: `${p.id}: ${p.title}`, id: p.id })),
      { placeHolder: "Select problem to solve" },
    );
    if (!pick) { return; }
    const result = await cli.solveProblem(pick.id);
    vscode.window.showInformationMessage(result);
  });

  register("jjj.dissolveProblem", async () => {
    const problems = cache.getProblems().filter(p => p.status === "open" || p.status === "in_progress");
    const pick = await vscode.window.showQuickPick(
      problems.map(p => ({ label: `${p.id}: ${p.title}`, id: p.id })),
      { placeHolder: "Select problem to dissolve" },
    );
    if (!pick) { return; }
    const result = await cli.dissolveProblem(pick.id);
    vscode.window.showInformationMessage(result);
  });

  // --- Solution ---

  register("jjj.newSolution", async () => {
    const problems = cache.getProblems().filter(p => p.status === "open" || p.status === "in_progress");
    const problemPick = await vscode.window.showQuickPick(
      problems.map(p => ({ label: `${p.id}: ${p.title}`, id: p.id })),
      { placeHolder: "Which problem does this solve?" },
    );
    if (!problemPick) { return; }
    const title = await vscode.window.showInputBox({ prompt: "Solution title" });
    if (!title) { return; }
    const result = await cli.newSolution(title, problemPick.id);
    vscode.window.showInformationMessage(result);
  });

  register("jjj.acceptSolution", async () => {
    const solutions = cache.getSolutions().filter(s => s.status === "testing");
    const pick = await vscode.window.showQuickPick(
      solutions.map(s => ({ label: `${s.id}: ${s.title}`, id: s.id })),
      { placeHolder: "Select solution to accept" },
    );
    if (!pick) { return; }
    const result = await cli.acceptSolution(pick.id);
    vscode.window.showInformationMessage(result);
  });

  register("jjj.refuteSolution", async () => {
    const solutions = cache.getSolutions().filter(s => s.status === "testing" || s.status === "proposed");
    const pick = await vscode.window.showQuickPick(
      solutions.map(s => ({ label: `${s.id}: ${s.title}`, id: s.id })),
      { placeHolder: "Select solution to refute" },
    );
    if (!pick) { return; }
    const result = await cli.refuteSolution(pick.id);
    vscode.window.showInformationMessage(result);
  });

  register("jjj.resumeSolution", async () => {
    const solutions = cache.getSolutions().filter(s => s.status === "testing" || s.status === "proposed");
    const pick = await vscode.window.showQuickPick(
      solutions.map(s => ({ label: `${s.id}: ${s.title}`, id: s.id })),
      { placeHolder: "Select solution to resume" },
    );
    if (!pick) { return; }
    const result = await cli.resumeSolution(pick.id);
    vscode.window.showInformationMessage(result);
  });

  // --- Critique ---

  register("jjj.newCritique", async () => {
    const solutions = cache.getSolutions().filter(s => s.status === "testing" || s.status === "proposed");
    const solutionPick = await vscode.window.showQuickPick(
      solutions.map(s => ({ label: `${s.id}: ${s.title}`, id: s.id })),
      { placeHolder: "Which solution to critique?" },
    );
    if (!solutionPick) { return; }
    const title = await vscode.window.showInputBox({ prompt: "Critique title" });
    if (!title) { return; }
    const severity = await vscode.window.showQuickPick(
      ["low", "medium", "high", "critical"],
      { placeHolder: "Severity" },
    );
    if (!severity) { return; }
    const result = await cli.newCritique(solutionPick.id, title, severity);
    vscode.window.showInformationMessage(result);
  });

  register("jjj.addressCritique", async () => {
    const critiques = cache.getCritiques().filter(c => c.status === "open" || c.status === "valid");
    const pick = await vscode.window.showQuickPick(
      critiques.map(c => ({ label: `${c.id}: ${c.title} [${c.severity}]`, id: c.id })),
      { placeHolder: "Select critique to address" },
    );
    if (!pick) { return; }
    const result = await cli.addressCritique(pick.id);
    vscode.window.showInformationMessage(result);
  });

  register("jjj.dismissCritique", async () => {
    const critiques = cache.getCritiques().filter(c => c.status === "open" || c.status === "valid");
    const pick = await vscode.window.showQuickPick(
      critiques.map(c => ({ label: `${c.id}: ${c.title} [${c.severity}]`, id: c.id })),
      { placeHolder: "Select critique to dismiss" },
    );
    if (!pick) { return; }
    const result = await cli.dismissCritique(pick.id);
    vscode.window.showInformationMessage(result);
  });

  register("jjj.validateCritique", async () => {
    const critiques = cache.getCritiques().filter(c => c.status === "open");
    const pick = await vscode.window.showQuickPick(
      critiques.map(c => ({ label: `${c.id}: ${c.title} [${c.severity}]`, id: c.id })),
      { placeHolder: "Select critique to validate (confirm as a real flaw)" },
    );
    if (!pick) { return; }
    const result = await cli.validateCritique(pick.id);
    vscode.window.showInformationMessage(result);
  });

  // --- Milestone ---

  register("jjj.newMilestone", async () => {
    const title = await vscode.window.showInputBox({ prompt: "Milestone title" });
    if (!title) { return; }
    const result = await cli.newMilestone(title);
    vscode.window.showInformationMessage(result);
  });

  // --- Navigate: Switch to Change ---

  // Registered separately (not via `register()`) to accept a tree node argument
  context.subscriptions.push(
    vscode.commands.registerCommand("jjj.switchToChange", async (node?: { solution?: { id: string } }) => {
      try {
        let solutionId: string;
        if (node && node.solution) {
          solutionId = node.solution.id;
        } else {
          // Fallback: QuickPick for solutions with changes
          const solutions = cache.getSolutions().filter(s => s.change_ids.length > 0);
          const pick = await vscode.window.showQuickPick(
            solutions.map(s => ({ label: s.title, description: `${s.change_ids.length} change(s)`, id: s.id })),
            { placeHolder: "Select solution to switch to" },
          );
          if (!pick) { return; }
          solutionId = pick.id;
        }

        const solution = cache.getSolutions().find(s => s.id === solutionId);
        if (!solution || solution.change_ids.length === 0) {
          vscode.window.showWarningMessage("No changes attached to this solution.");
          return;
        }

        const result = await cli.resumeSolution(solutionId);
        vscode.window.showInformationMessage(result);
        await cache.refresh();
      } catch (e: unknown) {
        const message = e instanceof Error ? e.message : String(e);
        vscode.window.showErrorMessage(`JJJ: ${message}`);
      }
    }),
  );

  // --- Editor: Add Critique Here ---

  register("jjj.addCritiqueHere", async () => {
    const editor = vscode.window.activeTextEditor;
    if (!editor) { return; }

    const line = editor.selection.active.line + 1; // 1-based
    const filePath = vscode.workspace.asRelativePath(editor.document.uri);

    const solutions = cache.getSolutions().filter(s => s.status === "testing" || s.status === "proposed");
    const solutionPick = await vscode.window.showQuickPick(
      solutions.map(s => ({ label: `${s.id}: ${s.title}`, id: s.id })),
      { placeHolder: "Which solution to critique?" },
    );
    if (!solutionPick) { return; }

    const title = await vscode.window.showInputBox({ prompt: "Critique title" });
    if (!title) { return; }

    const severity = await vscode.window.showQuickPick(
      ["low", "medium", "high", "critical"],
      { placeHolder: "Severity" },
    );
    if (!severity) { return; }

    const result = await cli.newCritique(solutionPick.id, title, severity, filePath, line);
    vscode.window.showInformationMessage(result);
  });

  // --- GitHub Sync Commands ---

  register("jjj.syncGithub", async () => {
    const result = await cli.syncGithub();
    vscode.window.showInformationMessage(result || "GitHub sync complete");
  });

  register("jjj.syncGithubImport", async () => {
    const issue = await vscode.window.showInputBox({
      prompt: "GitHub issue number (e.g., 123 or #123)",
    });
    if (!issue) { return; }
    const result = await cli.syncGithubImport(issue);
    vscode.window.showInformationMessage(result);
  });

  register("jjj.syncGithubStatus", async () => {
    const result = await cli.syncGithubStatus();
    const doc = await vscode.workspace.openTextDocument({
      content: result,
      language: "markdown",
    });
    await vscode.window.showTextDocument(doc, { preview: true });
  });
}
