import * as vscode from "vscode";
import { DataCache } from "../cache";
import { JjjCli, Critique } from "../cli";

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
    this.disposables.push(cache.onDidChange(() => this.syncThreads()));
    this.syncThreads();
  }

  private getWorkspaceUri(filePath: string): vscode.Uri | undefined {
    const folders = vscode.workspace.workspaceFolders;
    if (!folders?.length) { return undefined; }
    return vscode.Uri.joinPath(folders[0].uri, filePath);
  }

  private buildBody(critique: Critique): vscode.MarkdownString {
    const lines: string[] = [];
    lines.push(`**[${critique.severity}]** ${critique.title}`);
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

  syncThreads(): void {
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

      const lineStart = critique.line_start - 1; // 0-indexed
      const lineEnd = critique.line_end ? critique.line_end - 1 : lineStart;
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
      } else {
        const thread = this.controller.createCommentThread(uri, range, this.buildComments(critique));
        thread.label = solution?.title ?? "jjj Critique";
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

    // Fall back to solutions in review or proposed state
    const solutions = this.cache.getSolutions();
    const candidates = solutions.filter(s => s.status === "review" || s.status === "proposed");
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
