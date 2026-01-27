import * as vscode from "vscode";
import { DataCache } from "../cache";

export class EntityDocumentProvider implements vscode.TextDocumentContentProvider {
  private _onDidChange = new vscode.EventEmitter<vscode.Uri>();
  readonly onDidChange = this._onDidChange.event;

  constructor(private cache: DataCache) {
    cache.onDidChange(() => {
      // Refresh all open entity documents
      vscode.workspace.textDocuments.forEach(doc => {
        if (doc.uri.scheme === "jjj") {
          this._onDidChange.fire(doc.uri);
        }
      });
    });
  }

  provideTextDocumentContent(uri: vscode.Uri): string {
    const [type, id] = uri.path.split("/").filter(Boolean);

    switch (type) {
      case "problem": return this.renderProblem(id);
      case "solution": return this.renderSolution(id);
      case "critique": return this.renderCritique(id);
      case "milestone": return this.renderMilestone(id);
      default: return `Unknown entity type: ${type}`;
    }
  }

  private renderProblem(id: string): string {
    const p = this.cache.getProblem(id);
    if (!p) { return `Problem ${id} not found`; }

    const solutions = this.cache.getSolutionsForProblem(id);
    const solutionList = solutions.length > 0
      ? solutions.map(s => {
        const critiques = this.cache.getCritiquesForSolution(s.id);
        const openCount = critiques.filter(c => c.status === "open").length;
        return `  ${s.id}: ${s.title} [${s.status}] — ${openCount} open critiques`;
      }).join("\n")
      : "  (none)";

    return [
      `${p.title}`,
      "\u2501".repeat(60),
      `Status: ${p.status}  \u2502  Milestone: ${p.milestone_id || "Backlog"}  \u2502  Assignee: ${p.assignee || "unassigned"}`,
      `Tags: ${p.tags.length > 0 ? p.tags.join(", ") : "none"}`,
      "",
      "## Description",
      "",
      p.description || "(no description)",
      "",
      p.context ? `## Context\n\n${p.context}\n` : "",
      `## Solutions (${solutions.length})`,
      solutionList,
    ].filter(Boolean).join("\n");
  }

  private renderSolution(id: string): string {
    const s = this.cache.getSolution(id);
    if (!s) { return `Solution ${id} not found`; }

    const critiques = this.cache.getCritiquesForSolution(id);
    const openCritiques = critiques.filter(c => c.status === "open");
    const critiqueList = critiques.length > 0
      ? critiques.map(c => {
        const loc = c.file_path ? ` \u2014 ${c.file_path}:${c.line_start}` : "";
        return `  ${c.id}: ${c.title} [${c.severity}] (${c.status})${loc}`;
      }).join("\n")
      : "  (none)";

    const reviewers = s.requested_reviewers.length > 0
      ? s.requested_reviewers.map(r => {
        const lgtm = s.reviewed_by.includes(r) ? " (LGTM)" : " (requested)";
        return `@${r}${lgtm}`;
      }).join(", ")
      : "none";

    return [
      `${s.title}`,
      "\u2501".repeat(60),
      `Status: ${s.status}  \u2502  Problem: ${s.problem_id}  \u2502  Assignee: ${s.assignee || "unassigned"}`,
      `Reviewers: ${reviewers}`,
      `Tags: ${s.tags.length > 0 ? s.tags.join(", ") : "none"}`,
      "",
      "## Approach",
      "",
      s.approach || "(no approach described)",
      "",
      s.tradeoffs ? `## Tradeoffs\n\n${s.tradeoffs}\n` : "",
      `## Critiques (${openCritiques.length} open)`,
      critiqueList,
      "",
      s.change_ids.length > 0 ? `## Changes\n${s.change_ids.map(c => `  ${c}`).join("\n")}` : "",
    ].filter(Boolean).join("\n");
  }

  private renderCritique(id: string): string {
    const c = this.cache.getCritique(id);
    if (!c) { return `Critique ${id} not found`; }

    const location = c.file_path ? `${c.file_path}:${c.line_start}` : "none";
    const replies = c.replies.length > 0
      ? c.replies.map(r =>
        `@${r.author} \u2014 ${new Date(r.created_at).toLocaleString()}\n${r.body}`
      ).join("\n\n")
      : "(no replies)";

    return [
      `${c.title}`,
      "\u2501".repeat(60),
      `Status: ${c.status}  \u2502  Solution: ${c.solution_id}  \u2502  Severity: ${c.severity}`,
      `Location: ${location}  \u2502  Author: ${c.author || "unknown"}`,
      "",
      "## Argument",
      "",
      c.argument || "(no argument)",
      "",
      c.evidence ? `## Evidence\n\n${c.evidence}\n` : "",
      `## Discussion (${c.replies.length} replies)`,
      "",
      replies,
    ].filter(Boolean).join("\n");
  }

  private renderMilestone(id: string): string {
    const m = this.cache.getMilestone(id);
    if (!m) { return `Milestone ${id} not found`; }

    const problems = this.cache.getProblemsForMilestone(id);
    const solved = problems.filter(p => p.status === "solved").length;
    const problemList = problems.length > 0
      ? problems.map(p => `  ${p.id}: ${p.title} [${p.status}]`).join("\n")
      : "  (none)";

    return [
      `${m.title}`,
      "\u2501".repeat(60),
      `Status: ${m.status}  \u2502  Target: ${m.target_date ? new Date(m.target_date).toLocaleDateString() : "none"}`,
      `Progress: ${solved}/${problems.length} solved  \u2502  Assignee: ${m.assignee || "unassigned"}`,
      `Tags: ${m.tags.length > 0 ? m.tags.join(", ") : "none"}`,
      "",
      "## Goals",
      "",
      m.goals || "(no goals)",
      "",
      m.success_criteria ? `## Success Criteria\n\n${m.success_criteria}\n` : "",
      `## Problems (${problems.length})`,
      problemList,
    ].filter(Boolean).join("\n");
  }
}
