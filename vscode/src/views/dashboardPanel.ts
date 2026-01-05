import * as vscode from 'vscode';
import { JJJ, DashboardData } from '../jjj';

export class DashboardPanel {
  public static currentPanel: DashboardPanel | undefined;
  private readonly _panel: vscode.WebviewPanel;
  private readonly _extensionUri: vscode.Uri;
  private _disposables: vscode.Disposable[] = [];

  private constructor(
    panel: vscode.WebviewPanel,
    extensionUri: vscode.Uri,
    private jjj: JJJ,
  ) {
    this._panel = panel;
    this._extensionUri = extensionUri;

    this._update();

    this._panel.onDidDispose(() => this.dispose(), null, this._disposables);

    this._panel.webview.onDidReceiveMessage(
      (message) => {
        switch (message.command) {
          case 'openTask':
            vscode.commands.executeCommand('jjj.openTask', message.task);
            return;
          case 'openReview':
            vscode.commands.executeCommand('jjj.openReview', message.review);
            return;
          case 'refresh':
            this._update();
            return;
        }
      },
      null,
      this._disposables,
    );
  }

  public static createOrShow(extensionUri: vscode.Uri, jjj: JJJ) {
    const column = vscode.window.activeTextEditor
      ? vscode.window.activeTextEditor.viewColumn
      : undefined;

    if (DashboardPanel.currentPanel) {
      DashboardPanel.currentPanel._panel.reveal(column);
      return;
    }

    const panel = vscode.window.createWebviewPanel(
      'jjjDashboard',
      'JJJ Dashboard',
      column || vscode.ViewColumn.One,
      {
        enableScripts: true,
        localResourceRoots: [vscode.Uri.joinPath(extensionUri, 'media')],
      },
    );

    DashboardPanel.currentPanel = new DashboardPanel(panel, extensionUri, jjj);
  }

  public dispose() {
    DashboardPanel.currentPanel = undefined;

    this._panel.dispose();

    while (this._disposables.length) {
      const x = this._disposables.pop();
      if (x) {
        x.dispose();
      }
    }
  }

  private async _update() {
    const webview = this._panel.webview;
    this._panel.title = 'JJJ Dashboard';

    let data: DashboardData | null = null;
    try {
      data = await this.jjj.getDashboard();
    } catch (error) {
      console.error(error);
    }

    this._panel.webview.html = this._getHtmlForWebview(webview, data);
  }

  private _getHtmlForWebview(webview: vscode.Webview, data: DashboardData | null) {
    // Use a nonce to only allow specific scripts to be run
    const nonce = getNonce();

    const styleResetUri = webview.asWebviewUri(
      vscode.Uri.joinPath(this._extensionUri, 'media', 'reset.css'),
    );
    const styleVSCodeUri = webview.asWebviewUri(
      vscode.Uri.joinPath(this._extensionUri, 'media', 'vscode.css'),
    );
    const styleMainUri = webview.asWebviewUri(
      vscode.Uri.joinPath(this._extensionUri, 'media', 'main.css'),
    );

    return `<!DOCTYPE html>
            <html lang="en">
            <head>
                <meta charset="UTF-8">
                <meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src ${webview.cspSource}; script-src 'nonce-${nonce}';">
                <meta name="viewport" content="width=device-width, initial-scale=1.0">
                <link href="${styleResetUri}" rel="stylesheet">
                <link href="${styleVSCodeUri}" rel="stylesheet">
                <link href="${styleMainUri}" rel="stylesheet">
                <title>JJJ Dashboard</title>
            </head>
            <body>
                <div class="container">
                    <h1>Dashboard</h1>
                    <button id="refresh-btn">Refresh</button>
                    
                    ${data ? this._renderContent(data) : '<p>Loading...</p>'}
                </div>
                <script nonce="${nonce}">
                    const vscode = acquireVsCodeApi();
                    document.getElementById('refresh-btn').addEventListener('click', () => {
                        vscode.postMessage({ command: 'refresh' });
                    });
                    
                    // Add click handlers for tasks and reviews
                    document.querySelectorAll('.task-link').forEach(link => {
                        link.addEventListener('click', (e) => {
                            const taskId = e.target.dataset.id;
                            // We need to pass the full task object, but for now let's just pass ID
                            // In a real app we might store the full object in a map or data attribute
                            vscode.postMessage({ command: 'openTask', task: { id: taskId, title: e.target.innerText } });
                        });
                    });

                    document.querySelectorAll('.review-link').forEach(link => {
                        link.addEventListener('click', (e) => {
                            const changeId = e.target.dataset.id;
                            vscode.postMessage({ command: 'openReview', review: { change_id: changeId } });
                        });
                    });
                </script>
            </body>
            </html>`;
  }

  private _renderContent(data: DashboardData): string {
    return `
            <section>
                <h2>Pending Reviews (${data.pending_reviews.length})</h2>
                <ul>
                    ${data.pending_reviews
                      .map(
                        (r) => `
                        <li>
                            <a href="#" class="review-link" data-id="${r.change_id}">${r.change_id.substring(0, 10)}</a>
                            - ${r.author} (${r.status})
                        </li>
                    `,
                      )
                      .join('')}
                    ${data.pending_reviews.length === 0 ? '<li>No pending reviews</li>' : ''}
                </ul>
            </section>

            <section>
                <h2>My Tasks (${data.my_tasks.length})</h2>
                <ul>
                    ${data.my_tasks
                      .map(
                        (t) => `
                        <li>
                            <a href="#" class="task-link" data-id="${t.id}">${t.id}: ${t.title}</a>
                            [${t.column}]
                        </li>
                    `,
                      )
                      .join('')}
                    ${data.my_tasks.length === 0 ? '<li>No active tasks</li>' : ''}
                </ul>
            </section>
            
            <section>
                <h2>My Reviews (${data.my_reviews.length})</h2>
                <ul>
                    ${data.my_reviews
                      .map(
                        (r) => `
                        <li>
                            <a href="#" class="review-link" data-id="${r.change_id}">${r.change_id.substring(0, 10)}</a>
                            (${r.status})
                        </li>
                    `,
                      )
                      .join('')}
                    ${data.my_reviews.length === 0 ? '<li>No reviews requested</li>' : ''}
                </ul>
            </section>
        `;
  }
}

function getNonce() {
  let text = '';
  const possible = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789';
  for (let i = 0; i < 32; i++) {
    text += possible.charAt(Math.floor(Math.random() * possible.length));
  }
  return text;
}
