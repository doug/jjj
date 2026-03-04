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

  register("jjj.submitSolution", async () => {
    const solutions = cache.getSolutions().filter(s => s.status === "proposed");
    const pick = await vscode.window.showQuickPick(
      solutions.map(s => ({ label: s.title, description: s.id.slice(0, 8), id: s.id })),
      { placeHolder: "Select solution to submit for review" },
    );
    if (!pick) { return; }
    await cli.submitSolution(pick.id);
    vscode.window.showInformationMessage(`Solution "${pick.label}" submitted for review.`);
  });

  register("jjj.approveSolution", async () => {
    const solutions = cache.getSolutions().filter(s => s.status === "submitted");
    const pick = await vscode.window.showQuickPick(
      solutions.map(s => ({ label: s.title, description: s.id.slice(0, 8), id: s.id })),
      { placeHolder: "Select solution to approve" },
    );
    if (!pick) { return; }
    await cli.approveSolution(pick.id);
    vscode.window.showInformationMessage(`Solution "${pick.label}" approved.`);
  });

  register("jjj.withdrawSolution", async () => {
    const solutions = cache.getSolutions().filter(s => s.status === "submitted" || s.status === "proposed");
    const pick = await vscode.window.showQuickPick(
      solutions.map(s => ({ label: s.title, description: s.id.slice(0, 8), id: s.id })),
      { placeHolder: "Select solution to withdraw" },
    );
    if (!pick) { return; }
    await cli.withdrawSolution(pick.id);
    vscode.window.showInformationMessage(`Solution "${pick.label}" withdrawn.`);
  });

  register("jjj.resumeSolution", async () => {
    const solutions = cache.getSolutions().filter(s => s.status === "submitted" || s.status === "proposed");
    const pick = await vscode.window.showQuickPick(
      solutions.map(s => ({ label: `${s.id}: ${s.title}`, id: s.id })),
      { placeHolder: "Select solution to resume" },
    );
    if (!pick) { return; }
    const result = await cli.resumeSolution(pick.id);
    vscode.window.showInformationMessage(result);
  });

  register("jjj.lgtmSolution", async () => {
    // Default to active solution; fall back to picker if none active
    const status = cache.getStatus();
    const activeSolution = status?.active_solution;
    let solutionId: string | undefined;
    let solutionTitle: string | undefined;

    if (activeSolution) {
      solutionId = activeSolution.id;
      solutionTitle = activeSolution.title;
    } else {
      const solutions = cache.getSolutions().filter(s => s.status === "submitted");
      if (solutions.length === 0) {
        vscode.window.showInformationMessage("No solutions currently submitted for review.");
        return;
      }
      const pick = await vscode.window.showQuickPick(
        solutions.map(s => ({ label: s.title, description: s.id.slice(0, 8), id: s.id })),
        { placeHolder: "Select solution to sign off on" },
      );
      if (!pick) { return; }
      solutionId = pick.id;
      solutionTitle = pick.label;
    }

    await cli.lgtmSolution(solutionId);
    vscode.window.showInformationMessage(`LGTM — signed off on "${solutionTitle}".`);
  });

  // --- Critique ---

  register("jjj.newCritique", async () => {
    const solutions = cache.getSolutions().filter(s => s.status === "submitted" || s.status === "proposed");
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

    const solutions = cache.getSolutions().filter(s => s.status === "submitted" || s.status === "proposed");
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
  // --- Assign ---

  // Node-aware assign-to-me: receives tree node from context menu
  context.subscriptions.push(
    vscode.commands.registerCommand("jjj.assignToMe", async (node?: unknown) => {
      try {
        const status = cache.getStatus();
        const currentUser = status?.user;
        if (!currentUser) {
          vscode.window.showWarningMessage("Could not determine current user.");
          return;
        }

        let result: string;
        // Check if called from context menu with a node
        if (node && typeof node === "object" && "problem" in node) {
          const problemNode = node as { problem: { id: string; title: string } };
          result = await cli.assignProblem(problemNode.problem.id, currentUser);
        } else if (node && typeof node === "object" && "solution" in node) {
          const solutionNode = node as { solution: { id: string; title: string } };
          result = await cli.assignSolution(solutionNode.solution.id, currentUser);
        } else {
          // Fallback: quickpick
          const problems = cache.getProblems().filter(p => p.status === "open" || p.status === "in_progress");
          const pick = await vscode.window.showQuickPick(
            problems.map(p => ({ label: `${p.id}: ${p.title}`, id: p.id, type: "problem" as const })),
            { placeHolder: `Assign to ${currentUser}` },
          );
          if (!pick) { return; }
          result = await cli.assignProblem(pick.id, currentUser);
        }

        vscode.window.showInformationMessage(result || `Assigned to ${currentUser}`);
        await cache.refresh();
      } catch (e: unknown) {
        const message = e instanceof Error ? e.message : String(e);
        vscode.window.showErrorMessage(`JJJ: ${message}`);
      }
    }),
  );

  // --- Move to Milestone ---

  context.subscriptions.push(
    vscode.commands.registerCommand("jjj.moveProblemToMilestone", async (node?: unknown) => {
      try {
        let problemId: string | undefined;
        if (node && typeof node === "object" && "problem" in node) {
          problemId = (node as { problem: { id: string } }).problem.id;
        } else {
          const problems = cache.getProblems().filter(p => p.status === "open" || p.status === "in_progress");
          const pick = await vscode.window.showQuickPick(
            problems.map(p => ({ label: `${p.id}: ${p.title}`, id: p.id })),
            { placeHolder: "Select problem to move" },
          );
          problemId = pick?.id;
        }
        if (!problemId) { return; }

        const problem = cache.getProblems().find(p => p.id === problemId);
        const milestones = cache.getMilestones();

        const items: ({ label: string; id: string | null })[] = [
          { label: "$(inbox) Backlog (no milestone)", id: null },
          ...milestones.map(m => ({ label: `$(milestone) ${m.title}`, id: m.id })),
        ];
        const pick2 = await vscode.window.showQuickPick(items, { placeHolder: "Move to milestone" });
        if (!pick2) { return; }

        // Remove from old milestone if any
        if (problem?.milestone_id) {
          await cli.milestoneRemoveProblem(problem.milestone_id, problemId);
        }
        // Add to new milestone (unless Backlog)
        if (pick2.id) {
          await cli.milestoneAddProblem(pick2.id, problemId);
        }

        await cache.refresh();
        vscode.window.showInformationMessage(`Problem moved to ${pick2.label.replace(/^\$\([^)]+\) /, "")}.`);
      } catch (e: unknown) {
        const message = e instanceof Error ? e.message : String(e);
        vscode.window.showErrorMessage(`JJJ: ${message}`);
      }
    }),
  );

  // --- New Problem in Milestone ---

  context.subscriptions.push(
    vscode.commands.registerCommand("jjj.newProblemInMilestone", async (node?: unknown) => {
      try {
        let milestoneId: string | null = null;
        let milestoneName = "Backlog";

        if (node && typeof node === "object" && "milestone" in node) {
          const milestoneNode = node as { milestone: { id: string; title: string } | null };
          if (milestoneNode.milestone) {
            milestoneId = milestoneNode.milestone.id;
            milestoneName = milestoneNode.milestone.title;
          }
        }

        const title = await vscode.window.showInputBox({ prompt: `Problem title (will be added to ${milestoneName})` });
        if (!title) { return; }

        await cli.newProblem(title);

        // If a milestone is targeted, find the newly created problem and add it
        if (milestoneId) {
          await cache.refresh();
          const problems = cache.getProblems();
          const newest = problems.find(p => p.title === title);
          if (newest) {
            await cli.milestoneAddProblem(milestoneId, newest.id);
          }
        }

        await cache.refresh();
        vscode.window.showInformationMessage(`Problem "${title}" created in ${milestoneName}.`);
      } catch (e: unknown) {
        const message = e instanceof Error ? e.message : String(e);
        vscode.window.showErrorMessage(`JJJ: ${message}`);
      }
    }),
  );

  // --- Reopen Problem ---

  register("jjj.reopenProblem", async () => {
    const problems = cache.getProblems().filter(p => p.status === "solved" || p.status === "dissolved");
    if (problems.length === 0) {
      vscode.window.showInformationMessage("No solved or dissolved problems to reopen.");
      return;
    }
    const pick = await vscode.window.showQuickPick(
      problems.map(p => ({ label: `${p.id}: ${p.title}`, description: p.status, id: p.id })),
      { placeHolder: "Select problem to reopen" },
    );
    if (!pick) { return; }
    const result = await cli.reopenProblem(pick.id);
    vscode.window.showInformationMessage(result || `Problem "${pick.label}" reopened.`);
  });

  // --- Edit (Rename) ---

  context.subscriptions.push(
    vscode.commands.registerCommand("jjj.editProblem", async (node?: unknown) => {
      try {
        let problemId: string;
        let currentTitle: string;

        if (node && typeof node === "object" && "problem" in node) {
          const problemNode = node as { problem: { id: string; title: string } };
          problemId = problemNode.problem.id;
          currentTitle = problemNode.problem.title;
        } else {
          const problems = cache.getProblems();
          const pick = await vscode.window.showQuickPick(
            problems.map(p => ({ label: p.title, description: p.id.slice(0, 8), id: p.id, title: p.title })),
            { placeHolder: "Select problem to rename" },
          );
          if (!pick) { return; }
          problemId = pick.id;
          currentTitle = pick.title;
        }

        const newTitle = await vscode.window.showInputBox({ prompt: "New title", value: currentTitle });
        if (!newTitle) { return; }

        await cli.editProblem(problemId, newTitle);
        vscode.window.showInformationMessage(`Renamed to '${newTitle}'`);
        await cache.refresh();
      } catch (e: unknown) {
        const message = e instanceof Error ? e.message : String(e);
        vscode.window.showErrorMessage(`JJJ: ${message}`);
      }
    }),
  );

  context.subscriptions.push(
    vscode.commands.registerCommand("jjj.editSolution", async (node?: unknown) => {
      try {
        let solutionId: string;
        let currentTitle: string;

        if (node && typeof node === "object" && "solution" in node) {
          const solutionNode = node as { solution: { id: string; title: string } };
          solutionId = solutionNode.solution.id;
          currentTitle = solutionNode.solution.title;
        } else {
          const solutions = cache.getSolutions();
          const pick = await vscode.window.showQuickPick(
            solutions.map(s => ({ label: s.title, description: s.id.slice(0, 8), id: s.id, title: s.title })),
            { placeHolder: "Select solution to rename" },
          );
          if (!pick) { return; }
          solutionId = pick.id;
          currentTitle = pick.title;
        }

        const newTitle = await vscode.window.showInputBox({ prompt: "New title", value: currentTitle });
        if (!newTitle) { return; }

        await cli.editSolution(solutionId, newTitle);
        vscode.window.showInformationMessage(`Renamed to '${newTitle}'`);
        await cache.refresh();
      } catch (e: unknown) {
        const message = e instanceof Error ? e.message : String(e);
        vscode.window.showErrorMessage(`JJJ: ${message}`);
      }
    }),
  );

}
