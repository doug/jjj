import * as vscode from "vscode";
import { JjjCli } from "./cli";
import { DataCache } from "./cache";
import { NextActionsProvider } from "./views/nextActionsProvider";

export function activate(context: vscode.ExtensionContext) {
  const cli = new JjjCli();
  const cache = new DataCache(cli);

  // Views
  const nextActions = new NextActionsProvider(cache);
  vscode.window.registerTreeDataProvider("jjj-next-actions", nextActions);

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
