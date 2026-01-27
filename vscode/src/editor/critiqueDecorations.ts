import * as vscode from "vscode";
import { DataCache } from "../cache";

const criticalDecoration = vscode.window.createTextEditorDecorationType({
  overviewRulerColor: "red",
  overviewRulerLane: vscode.OverviewRulerLane.Left,
  before: {
    contentText: "\u25cf",
    color: new vscode.ThemeColor("errorForeground"),
    margin: "0 4px 0 0",
  },
});

const warningDecoration = vscode.window.createTextEditorDecorationType({
  overviewRulerColor: "yellow",
  overviewRulerLane: vscode.OverviewRulerLane.Left,
  before: {
    contentText: "\u25cf",
    color: new vscode.ThemeColor("editorWarning.foreground"),
    margin: "0 4px 0 0",
  },
});

const resolvedDecoration = vscode.window.createTextEditorDecorationType({
  overviewRulerColor: "gray",
  overviewRulerLane: vscode.OverviewRulerLane.Left,
  before: {
    contentText: "\u25cb",
    color: new vscode.ThemeColor("disabledForeground"),
    margin: "0 4px 0 0",
  },
});

export class CritiqueDecorationManager {
  private disposables: vscode.Disposable[] = [];

  constructor(private cache: DataCache) {
    this.disposables.push(
      vscode.window.onDidChangeActiveTextEditor(() => this.updateDecorations()),
      cache.onDidChange(() => this.updateDecorations()),
    );
    this.updateDecorations();
  }

  private updateDecorations() {
    const editor = vscode.window.activeTextEditor;
    if (!editor) { return; }

    const filePath = vscode.workspace.asRelativePath(editor.document.uri);
    const critiques = this.cache.getCritiquesWithLocations()
      .filter(c => c.file_path === filePath);

    const critical: vscode.DecorationOptions[] = [];
    const warning: vscode.DecorationOptions[] = [];
    const resolved: vscode.DecorationOptions[] = [];

    for (const c of critiques) {
      if (!c.line_start) { continue; }
      const line = c.line_start - 1; // 0-based
      const range = new vscode.Range(line, 0, line, 0);
      const option: vscode.DecorationOptions = {
        range,
        hoverMessage: new vscode.MarkdownString(
          `**${c.id}**: ${c.title} [${c.severity}]\n\n${c.argument}\n\n*${c.status}*`,
        ),
      };

      if (c.status !== "open") {
        resolved.push(option);
      } else if (c.severity === "high" || c.severity === "critical") {
        critical.push(option);
      } else {
        warning.push(option);
      }
    }

    editor.setDecorations(criticalDecoration, critical);
    editor.setDecorations(warningDecoration, warning);
    editor.setDecorations(resolvedDecoration, resolved);
  }

  dispose() {
    this.disposables.forEach(d => d.dispose());
    criticalDecoration.dispose();
    warningDecoration.dispose();
    resolvedDecoration.dispose();
  }
}
