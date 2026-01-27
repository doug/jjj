import * as vscode from "vscode";
import { JjjCli } from "./cli";
import { DataCache } from "./cache";
import { NextActionsProvider } from "./views/nextActionsProvider";
import { ProjectTreeProvider } from "./views/projectTreeProvider";
import { EntityDocumentProvider } from "./documents/entityDocumentProvider";
import { StatusBar } from "./statusBar";

export function activate(context: vscode.ExtensionContext) {
  const cli = new JjjCli();
  const cache = new DataCache(cli);

  // Views
  const nextActions = new NextActionsProvider(cache);
  vscode.window.registerTreeDataProvider("jjj-next-actions", nextActions);

  const projectTree = new ProjectTreeProvider(cache, cli);
  const treeView = vscode.window.createTreeView("jjj-project-tree", {
    treeDataProvider: projectTree,
    dragAndDropController: projectTree,
    canSelectMany: true,
  });
  context.subscriptions.push(treeView);

  // Virtual Documents
  const docProvider = new EntityDocumentProvider(cache);
  context.subscriptions.push(
    vscode.workspace.registerTextDocumentContentProvider("jjj", docProvider),
  );

  context.subscriptions.push(
    vscode.commands.registerCommand("jjj.openEntity", async (type: string, id: string) => {
      const uri = vscode.Uri.parse(`jjj:///${type}/${id}`);
      const doc = await vscode.workspace.openTextDocument(uri);
      await vscode.window.showTextDocument(doc, { preview: false });
    }),
  );

  // Status Bar
  const statusBar = new StatusBar(cache);
  context.subscriptions.push(statusBar);

  // Commands
  context.subscriptions.push(
    vscode.commands.registerCommand("jjj.refreshAll", () => cache.refresh()),
  );

  // Auto-refresh
  const interval = setInterval(() => cache.refresh(), 30000);
  context.subscriptions.push({ dispose: () => clearInterval(interval) });

  // Refresh on file save
  context.subscriptions.push(
    vscode.workspace.onDidSaveTextDocument(() => cache.refresh()),
  );

  // Initial load
  cache.refresh();
}

export function deactivate() {}
