import * as vscode from "vscode";
import { JjjCli } from "./cli";
import { DataCache } from "./cache";
import { ProjectTreeProvider } from "./views/projectTreeProvider";
import { EntityDocumentProvider } from "./documents/entityDocumentProvider";
import { CritiqueCommentController } from "./editor/critiqueComments";
import { registerCommands } from "./commands";

export function activate(context: vscode.ExtensionContext) {
  const cli = new JjjCli();
  const cache = new DataCache(cli);
  context.subscriptions.push(cache);

  // --- Views ---
  const projectTree = new ProjectTreeProvider(cache, cli);
  const treeView = vscode.window.createTreeView("jjj-project-tree", {
    treeDataProvider: projectTree,
    dragAndDropController: projectTree,
    canSelectMany: true,
  });
  context.subscriptions.push(treeView, projectTree);

  // --- Filter State ---
  // Restore persisted filter mode (default: "open" for new workspaces)
  const config = vscode.workspace.getConfiguration("jjj");
  const savedFilterMode = config.get<"all" | "open">("filterMode", "open");
  projectTree.setFilterMode(savedFilterMode);

  function applyFilterMode(mode: "all" | "open"): void {
    treeView.title = mode === "open" ? "Project (Open)" : "Project";
    vscode.commands.executeCommand("setContext", "jjj.filterMode", mode);
    config.update("filterMode", mode, vscode.ConfigurationTarget.Workspace);
  }

  applyFilterMode(savedFilterMode);

  // Toggle filter command (keyboard shortcut: Cmd+Shift+O)
  context.subscriptions.push(
    vscode.commands.registerCommand("jjj.toggleTreeFilter", () => {
      projectTree.toggleFilter();
      applyFilterMode(projectTree.filterMode);
    }),
  );

  // Icon-specific commands for view/title buttons (swap icons based on current mode)
  context.subscriptions.push(
    vscode.commands.registerCommand("jjj.showOpenOnly", () => {
      projectTree.setFilterMode("open");
      applyFilterMode("open");
    }),
    vscode.commands.registerCommand("jjj.showAll", () => {
      projectTree.setFilterMode("all");
      applyFilterMode("all");
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

  // --- Virtual Documents ---
  const docProvider = new EntityDocumentProvider(cache);
  context.subscriptions.push(
    vscode.workspace.registerTextDocumentContentProvider("jjj", docProvider),
    docProvider,
  );

  context.subscriptions.push(
    vscode.commands.registerCommand("jjj.openEntity", async (type: string, id: string) => {
      const uri = vscode.Uri.parse(`jjj:///${type}/${id}.md`);
      const doc = await vscode.workspace.openTextDocument(uri);
      await vscode.window.showTextDocument(doc, { preview: false });
    }),
  );

  // --- Inline Comment Threads ---
  const critiqueComments = new CritiqueCommentController(cache, cli);
  context.subscriptions.push(
    critiqueComments,
    vscode.commands.registerCommand("jjj.commentAddressCritique",
      (thread: vscode.CommentThread) => critiqueComments.addressCritique(thread)),
    vscode.commands.registerCommand("jjj.commentDismissCritique",
      (thread: vscode.CommentThread) => critiqueComments.dismissCritique(thread)),
    vscode.commands.registerCommand("jjj.commentValidateCritique",
      (thread: vscode.CommentThread) => critiqueComments.validateCritique(thread)),
    vscode.commands.registerCommand("jjj.commentReplyCritique",
      (reply: vscode.CommentReply) => critiqueComments.replyToCritique(reply)),
  );

  // --- Commands ---
  context.subscriptions.push(
    vscode.commands.registerCommand("jjj.refreshAll", () => cache.refresh()),
  );
  registerCommands(context, cli, cache);

  // --- Auto-Refresh ---
  const interval = setInterval(() => cache.refresh(), 30000);
  context.subscriptions.push({ dispose: () => clearInterval(interval) });

  let debounceTimer: ReturnType<typeof setTimeout> | undefined;
  context.subscriptions.push(
    vscode.workspace.onDidSaveTextDocument(() => {
      if (debounceTimer) { clearTimeout(debounceTimer); }
      debounceTimer = setTimeout(() => cache.refresh(), 500);
    }),
  );
  context.subscriptions.push({ dispose: () => { if (debounceTimer) { clearTimeout(debounceTimer); } } });

  // --- Initial Load ---
  cache.refresh();

  console.log("JJJ extension activated");
}

export function deactivate() {}
