import * as vscode from "vscode";
import { DataCache } from "../cache";
import { JjjCli, Critique } from "../cli";

// ---------------------------------------------------------------------------
// Pure anchor-finding logic (no VS Code API dependencies)
// ---------------------------------------------------------------------------

/** Result of searching for a critique's code in the current document. */
export interface AnchorResult {
  /** 1-indexed line where the critiqued code was found (or best guess). */
  line: number;
  /** Match quality 0.0–1.0. Below ANCHOR_THRESHOLD means "outdated". */
  score: number;
  /** True if confidence is too low — code may have been deleted/rewritten. */
  outdated: boolean;
}

/** Minimum similarity fraction to consider a window a valid match. */
export const ANCHOR_THRESHOLD = 0.5;

/** Normalize a line for fuzzy comparison: trim + collapse whitespace. */
export function normalizeLine(line: string): string {
  return line.trim().replace(/\s+/g, ' ');
}

/** Score similarity between two normalized lines (0.0 or 1.0 for exact, 0.8 for substring). */
export function lineSimilarity(a: string, b: string): number {
  const na = normalizeLine(a);
  const nb = normalizeLine(b);
  if (na === nb) { return 1.0; }
  if (na === '' && nb === '') { return 1.0; }
  if (na.includes(nb) || nb.includes(na)) { return 0.8; }
  return 0.0;
}

/**
 * Search document lines for the best position matching the critique's context window.
 * Returns the 1-indexed line where `code_context` starts (i.e., after context_before).
 *
 * Algorithm:
 * 1. Build search window = context_before + code_context + context_after
 * 2. Slide window across document, score each position
 * 3. Return highest-scoring position, tiebreak by proximity to original line_start
 * 4. If best score < ANCHOR_THRESHOLD, mark as outdated
 */
export function findAnchorLine(
  docLines: string[],
  lineStart: number,          // 1-indexed original line
  codeContext: string[],
  contextBefore: string[],
  contextAfter: string[],
): AnchorResult {
  const searchWindow = [...contextBefore, ...codeContext, ...contextAfter];

  if (searchWindow.length === 0 || codeContext.length === 0) {
    return { line: lineStart, score: 0, outdated: true };
  }

  let bestScore = -1;
  let bestLine = lineStart;

  const maxStart = docLines.length - searchWindow.length;
  for (let i = 0; i <= maxStart; i++) {
    let total = 0;
    for (let j = 0; j < searchWindow.length; j++) {
      total += lineSimilarity(docLines[i + j], searchWindow[j]);
    }
    const score = total / searchWindow.length;

    // The 1-indexed line where code_context begins within this window
    const candidateLine = i + contextBefore.length + 1;
    const candidateDist = Math.abs(candidateLine - lineStart);
    const bestDist = Math.abs(bestLine - lineStart);

    if (score > bestScore || (score === bestScore && candidateDist < bestDist)) {
      bestScore = score;
      bestLine = candidateLine;
    }
  }

  return {
    line: bestLine,
    score: bestScore,
    outdated: bestScore < ANCHOR_THRESHOLD,
  };
}

// ---------------------------------------------------------------------------
// Comment controller
// ---------------------------------------------------------------------------

export class CritiqueCommentController implements vscode.Disposable {
  private controller: vscode.CommentController;
  // critiqueId → CommentThread for update/delete on cache refresh
  private threads = new Map<string, vscode.CommentThread>();
  private disposables: vscode.Disposable[] = [];

  constructor(private cache: DataCache, private cli: JjjCli) {
    this.controller = vscode.comments.createCommentController("jjj-critiques", "jjj Critiques");
    this.controller.options = {
      prompt: "Reply to this critique…",
      placeHolder: "Type a reply…",
    };
    this.controller.commentingRangeProvider = {
      provideCommentingRanges(document: vscode.TextDocument): vscode.Range[] {
        return [new vscode.Range(0, 0, Math.max(0, document.lineCount - 1), 0)];
      },
    };
    this.disposables.push(this.controller);
    this.disposables.push(cache.onDidChange(() => { void this.syncThreads(); }));
    void this.syncThreads();
  }

  private getWorkspaceUri(filePath: string): vscode.Uri | undefined {
    const folders = vscode.workspace.workspaceFolders;
    if (!folders?.length) { return undefined; }
    return vscode.Uri.joinPath(folders[0].uri, filePath);
  }

  private buildBody(critique: Critique): vscode.MarkdownString {
    const lines: string[] = [];
    lines.push(`**[${critique.severity}]** ${critique.title}`);
    if (critique.file_path && critique.line_start) {
      const loc = critique.line_end && critique.line_end !== critique.line_start
        ? `${critique.file_path}:${critique.line_start}-${critique.line_end}`
        : `${critique.file_path}:${critique.line_start}`;
      lines.push("", `📍 **Location:** ${loc}`);
    }
    if (critique.code_context && critique.code_context.length > 0) {
      lines.push("", "```", ...critique.code_context, "```");
    }
    if (critique.argument) { lines.push("", critique.argument); }
    if (critique.evidence) { lines.push("", `**Evidence:** ${critique.evidence}`); }
    const solution = this.cache.getSolution(critique.solution_id);
    if (solution) { lines.push("", `*Solution: ${solution.title}*`); }
    const md = new vscode.MarkdownString(lines.join("\n"));
    md.isTrusted = false;
    return md;
  }

  private buildComments(critique: Critique): vscode.Comment[] {
    const primary: vscode.Comment = {
      author: { name: critique.author ?? critique.reviewer ?? "jjj" },
      body: this.buildBody(critique),
      mode: vscode.CommentMode.Preview,
      timestamp: new Date(critique.created_at),
    };
    const replies: vscode.Comment[] = critique.replies.map(r => ({
      author: { name: r.author },
      body: new vscode.MarkdownString(r.body),
      mode: vscode.CommentMode.Preview,
      timestamp: new Date(r.created_at),
    }));
    return [primary, ...replies];
  }

  async syncThreads(): Promise<void> {
    const critiques = this.cache.getCritiquesWithLocations();
    const activeIds = new Set(critiques.map(c => c.id));

    // Remove stale threads
    for (const [id, thread] of this.threads) {
      if (!activeIds.has(id)) {
        thread.dispose();
        this.threads.delete(id);
      }
    }

    for (const critique of critiques) {
      if (!critique.file_path || !critique.line_start) { continue; }
      const uri = this.getWorkspaceUri(critique.file_path);
      if (!uri) { continue; }

      // Try to re-anchor using fuzzy context search
      let lineStart = critique.line_start - 1; // 0-indexed for VS Code
      let lineEnd = critique.line_end ? critique.line_end - 1 : lineStart;
      let isOutdated = false;

      if (critique.code_context && critique.code_context.length > 0) {
        try {
          const doc = await vscode.workspace.openTextDocument(uri);
          const docLines = doc.getText().split('\n');
          const anchor = findAnchorLine(
            docLines,
            critique.line_start,
            critique.code_context,
            critique.context_before ?? [],
            critique.context_after ?? [],
          );
          lineStart = anchor.line - 1; // convert back to 0-indexed
          lineEnd = lineStart + (critique.line_end ?? critique.line_start) - critique.line_start;
          isOutdated = anchor.outdated;
        } catch {
          // File may not exist in workspace — use stored line numbers
        }
      }

      const range = new vscode.Range(lineStart, 0, lineEnd, Number.MAX_VALUE);
      const isResolved = critique.status === "addressed" || critique.status === "dismissed";
      const solution = this.cache.getSolution(critique.solution_id);

      const existing = this.threads.get(critique.id);
      if (existing) {
        existing.comments = this.buildComments(critique);
        existing.state = isResolved
          ? vscode.CommentThreadState.Resolved
          : vscode.CommentThreadState.Unresolved;
        existing.canReply = !isResolved;
        existing.contextValue = critique.status;
        existing.collapsibleState = isResolved
          ? vscode.CommentThreadCollapsibleState.Collapsed
          : vscode.CommentThreadCollapsibleState.Expanded;
        if (isOutdated) {
          existing.label = `⚠️ Outdated — ${solution?.title ?? "jjj Critique"}`;
        } else {
          existing.label = solution?.title ?? "jjj Critique";
        }
      } else {
        const thread = this.controller.createCommentThread(uri, range, this.buildComments(critique));
        if (isOutdated) {
          thread.label = `⚠️ Outdated — ${solution?.title ?? "jjj Critique"}`;
        } else {
          thread.label = solution?.title ?? "jjj Critique";
        }
        thread.state = isResolved
          ? vscode.CommentThreadState.Resolved
          : vscode.CommentThreadState.Unresolved;
        thread.canReply = !isResolved;
        thread.contextValue = critique.status;
        thread.collapsibleState = isResolved
          ? vscode.CommentThreadCollapsibleState.Collapsed
          : vscode.CommentThreadCollapsibleState.Expanded;
        this.threads.set(critique.id, thread);
      }
    }
  }

  findCritiqueIdForThread(thread: vscode.CommentThread): string | undefined {
    for (const [id, t] of this.threads) {
      if (t === thread) { return id; }
    }
    return undefined;
  }

  async addressCritique(thread: vscode.CommentThread): Promise<void> {
    const id = this.findCritiqueIdForThread(thread);
    if (!id) { return; }
    await this.cli.addressCritique(id);
    await this.cache.refresh();
  }

  async dismissCritique(thread: vscode.CommentThread): Promise<void> {
    const id = this.findCritiqueIdForThread(thread);
    if (!id) { return; }
    await this.cli.dismissCritique(id);
    await this.cache.refresh();
  }

  async validateCritique(thread: vscode.CommentThread): Promise<void> {
    const id = this.findCritiqueIdForThread(thread);
    if (!id) { return; }
    await this.cli.validateCritique(id);
    await this.cache.refresh();
  }

  async replyToCritique(reply: vscode.CommentReply): Promise<void> {
    const id = this.findCritiqueIdForThread(reply.thread);
    if (!id) {
      await this.createCritique(reply);
      return;
    }
    await this.cli.replyCritique(id, reply.text);
    await this.cache.refresh();
  }

  private async createCritique(reply: vscode.CommentReply): Promise<void> {
    const thread = reply.thread;
    const title = reply.text.trim();
    if (!title) { thread.dispose(); return; }

    const severity = await vscode.window.showQuickPick(
      ["low", "medium", "high", "critical"],
      { placeHolder: "Critique severity" },
    );
    if (!severity) { thread.dispose(); return; }

    const solutionId = await this.resolveSolutionId();
    if (!solutionId) { thread.dispose(); return; }

    const filePath = vscode.workspace.asRelativePath(thread.uri);
    const line = thread.range ? thread.range.start.line + 1 : undefined; // convert 0-indexed to 1-indexed

    thread.dispose(); // remove temp thread; syncThreads will create the real one after refresh
    await this.cli.newCritique(solutionId, title, severity, filePath, line);
    await this.cache.refresh();
  }

  private async resolveSolutionId(): Promise<string | undefined> {
    // Prefer the active solution from status
    const activeSolution = this.cache.getStatus()?.active_solution;
    if (activeSolution) { return activeSolution.id; }

    // Fall back to solutions in submitted or proposed state
    const solutions = this.cache.getSolutions();
    const candidates = solutions.filter(s => s.status === "submitted" || s.status === "proposed");
    if (candidates.length === 0) { return undefined; }
    if (candidates.length === 1) { return candidates[0].id; }

    const picked = await vscode.window.showQuickPick(
      candidates.map(s => ({ label: s.title, description: s.id, id: s.id })),
      { placeHolder: "Select solution to critique" },
    );
    return picked?.id;
  }

  dispose(): void {
    for (const thread of this.threads.values()) { thread.dispose(); }
    this.threads.clear();
    this.disposables.forEach(d => d.dispose());
  }
}
