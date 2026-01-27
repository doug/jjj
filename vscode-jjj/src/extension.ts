import * as vscode from "vscode";
import { JjjCli } from "./cli";
import { DataCache } from "./cache";
import { NextActionsProvider } from "./views/nextActionsProvider";
import { ProjectTreeProvider } from "./views/projectTreeProvider";
import { EntityDocumentProvider } from "./documents/entityDocumentProvider";
import { StatusBar } from "./statusBar";
import { CritiqueDecorationManager } from "./editor/critiqueDecorations";
import { registerCommands } from "./commands";

export function activate(context: vscode.ExtensionContext) {
  const cli = new JjjCli();
  const cache = new DataCache(cli);
  context.subscriptions.push(cache);

  // --- Views ---
  const nextActions = new NextActionsProvider(cache);
  vscode.window.registerTreeDataProvider("jjj-next-actions", nextActions);
  context.subscriptions.push(nextActions);

  const projectTree = new ProjectTreeProvider(cache, cli);
  const treeView = vscode.window.createTreeView("jjj-project-tree", {
    treeDataProvider: projectTree,
    dragAndDropController: projectTree,
    canSelectMany: true,
  });
  context.subscriptions.push(treeView, projectTree);

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

  // --- Status Bar ---
  const statusBar = new StatusBar(cache);
  context.subscriptions.push(statusBar);

  // --- Gutter Decorations ---
  const decorations = new CritiqueDecorationManager(cache);
  context.subscriptions.push(decorations);

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
