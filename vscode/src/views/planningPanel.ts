import * as vscode from 'vscode';
import { JJJ } from '../jjj';

export class PlanningPanel {
    public static currentPanel: PlanningPanel | undefined;
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
            async (message) => {
                switch (message.command) {
                    case 'createMilestone':
                        vscode.commands.executeCommand('jjj.createMilestone');
                        return;
                    case 'createFeature':
                        // Pre-fill milestone in feature creation
                        vscode.commands.executeCommand('jjj.createFeature', message.milestoneId);
                        return;
                    case 'updateDescription':
                        try {
                            // Update milestone description
                            await this.jjj.updateMilestone(message.milestoneId, { description: message.description });
                            vscode.window.showInformationMessage(`Updated ${message.milestoneId} description`);
                        } catch (error) {
                            vscode.window.showErrorMessage(`Failed to update description: ${error}`);
                        }
                        return;
                    case 'moveFeature':
                        try {
                            // Move feature to different milestone
                            const milestoneId = message.milestoneId === 'unassigned' ? null : message.milestoneId;
                            await this.jjj.updateFeature(message.featureId, { milestone: milestoneId });
                            vscode.window.showInformationMessage(`Moved ${message.featureId} to ${message.milestoneId}`);
                            this._update();
                        } catch (error) {
                            vscode.window.showErrorMessage(`Failed to move feature: ${error}`);
                        }
                        return;
                    case 'openFeature':
                        vscode.commands.executeCommand('jjj.openFeature', message.feature);
                        return;
                    case 'openMilestone':
                        vscode.commands.executeCommand('jjj.openMilestone', message.milestone);
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

        if (PlanningPanel.currentPanel) {
            PlanningPanel.currentPanel._panel.reveal(column);
            return;
        }

        const panel = vscode.window.createWebviewPanel(
            'jjjPlanning',
            'JJJ Planning View',
            column || vscode.ViewColumn.One,
            {
                enableScripts: true,
                localResourceRoots: [vscode.Uri.joinPath(extensionUri, 'media')],
            },
        );

        PlanningPanel.currentPanel = new PlanningPanel(panel, extensionUri, jjj);
    }

    public dispose() {
        PlanningPanel.currentPanel = undefined;

        this._panel.dispose();

        while (this._disposables.length) {
            const x = this._disposables.pop();
            if (x) {
                x.dispose();
            }
        }
    }

    public refresh() {
        this._update();
    }

    private async _update() {
        const webview = this._panel.webview;
        this._panel.title = 'JJJ Planning View';

        let milestones: any[] = [];
        let features: any[] = [];

        try {
            [milestones, features] = await Promise.all([
                this.jjj.listMilestones(),
                this.jjj.listFeatures(),
            ]);
        } catch (error) {
            console.error(error);
        }

        this._panel.webview.html = this._getHtmlForWebview(webview, milestones, features);
    }

    private _getHtmlForWebview(webview: vscode.Webview, milestones: any[], features: any[]) {
        const nonce = getNonce();

        const styleResetUri = webview.asWebviewUri(
            vscode.Uri.joinPath(this._extensionUri, 'media', 'reset.css'),
        );
        const styleVSCodeUri = webview.asWebviewUri(
            vscode.Uri.joinPath(this._extensionUri, 'media', 'vscode.css'),
        );
        const stylePlanningUri = webview.asWebviewUri(
            vscode.Uri.joinPath(this._extensionUri, 'media', 'planning.css'),
        );

        // Group features by milestone
        const featuresByMilestone = new Map<string, any[]>();
        featuresByMilestone.set('unassigned', []);

        milestones.forEach(m => featuresByMilestone.set(m.id, []));

        features.forEach(f => {
            const mid = f.milestone_id || 'unassigned';
            if (!featuresByMilestone.has(mid)) {
                featuresByMilestone.set(mid, []);
            }
            featuresByMilestone.get(mid)!.push(f);
        });

        // Sort milestones by date
        const sortedMilestones = [...milestones].sort((a, b) => {
            if (a.date && b.date) {
                return a.date.localeCompare(b.date);
            }
            return a.id.localeCompare(b.id);
        });

        return `<!DOCTYPE html>
            <html lang="en">
            <head>
                <meta charset="UTF-8">
                <meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src ${webview.cspSource}; script-src 'nonce-${nonce}';">
                <meta name="viewport" content="width=device-width, initial-scale=1.0">
                <link href="${styleResetUri}" rel="stylesheet">
                <link href="${styleVSCodeUri}" rel="stylesheet">
                <link href="${stylePlanningUri}" rel="stylesheet">
                <title>JJJ Planning View</title>
            </head>
            <body>
                <div class="planning-header">
                    <h1>Planning View</h1>
                    <div class="header-actions">
                        <button id="add-milestone-btn" class="add-milestone-btn">+ Add Milestone</button>
                        <button id="refresh-btn" class="refresh-button">↻</button>
                    </div>
                </div>
                
                <div class="planning-container">
                    ${this._renderMilestoneColumn('unassigned', 'Unassigned', '', featuresByMilestone.get('unassigned') || [])}
                    ${sortedMilestones.map(m =>
            this._renderMilestoneColumn(m.id, m.title, m.description || '', featuresByMilestone.get(m.id) || [], m.date)
        ).join('')}
                </div>
                
                <script nonce="${nonce}">
                    const vscode = acquireVsCodeApi();
                    
                    // Add milestone button
                    document.getElementById('add-milestone-btn').addEventListener('click', () => {
                        vscode.postMessage({ command: 'createMilestone' });
                    });
                    
                    // Refresh button
                    document.getElementById('refresh-btn').addEventListener('click', () => {
                        vscode.postMessage({ command: 'refresh' });
                    });
                    
                    // Description editing
                    document.querySelectorAll('.milestone-description').forEach(textarea => {
                        textarea.addEventListener('blur', () => {
                            const milestoneId = textarea.dataset.milestoneId;
                            if (milestoneId !== 'unassigned') {
                                vscode.postMessage({
                                    command: 'updateDescription',
                                    milestoneId: milestoneId,
                                    description: textarea.value
                                });
                            }
                        });
                        
                        // Also save on Ctrl+Enter
                        textarea.addEventListener('keydown', (e) => {
                            if (e.ctrlKey && e.key === 'Enter') {
                                textarea.blur();
                            }
                        });
                    });
                    
                    // Drag and drop for features
                    let draggedFeatureId = null;
                    
                    document.querySelectorAll('.feature-card').forEach(card => {
                        card.addEventListener('dragstart', (e) => {
                            draggedFeatureId = e.target.dataset.featureId;
                            e.target.classList.add('dragging');
                        });
                        
                        card.addEventListener('dragend', (e) => {
                            e.target.classList.remove('dragging');
                            draggedFeatureId = null;
                        });
                        
                        card.addEventListener('click', () => {
                            const featureId = card.dataset.featureId;
                            const featureTitle = card.dataset.featureTitle;
                            vscode.postMessage({
                                command: 'openFeature',
                                feature: { id: featureId, title: featureTitle }
                            });
                        });
                    });
                    
                    document.querySelectorAll('.feature-list').forEach(list => {
                        list.addEventListener('dragover', (e) => {
                            e.preventDefault();
                            list.classList.add('drag-over');
                        });
                        
                        list.addEventListener('dragleave', (e) => {
                            if (e.target === list) {
                                list.classList.remove('drag-over');
                            }
                        });
                        
                        list.addEventListener('drop', (e) => {
                            e.preventDefault();
                            list.classList.remove('drag-over');
                            
                            if (draggedFeatureId) {
                                const targetMilestoneId = list.dataset.milestoneId;
                                vscode.postMessage({
                                    command: 'moveFeature',
                                    featureId: draggedFeatureId,
                                    milestoneId: targetMilestoneId
                                });
                            }
                        });
                    });
                    
                    // Add feature buttons
                    document.querySelectorAll('.add-feature-btn').forEach(btn => {
                        btn.addEventListener('click', (e) => {
                            e.stopPropagation();
                            const milestoneId = btn.dataset.milestoneId;
                            vscode.postMessage({
                                command: 'createFeature',
                                milestoneId: milestoneId !== 'unassigned' ? milestoneId : null
                            });
                        });
                    });
                    
                    // Milestone title click
                    document.querySelectorAll('.milestone-title').forEach(title => {
                        title.addEventListener('click', () => {
                            const milestoneId = title.dataset.milestoneId;
                            const milestoneTitle = title.textContent;
                            if (milestoneId !== 'unassigned') {
                                vscode.postMessage({
                                    command: 'openMilestone',
                                    milestone: { id: milestoneId, title: milestoneTitle }
                                });
                            }
                        });
                    });
                </script>
            </body>
            </html>`;
    }

    private _renderMilestoneColumn(id: string, title: string, description: string, features: any[], date?: string): string {
        const isUnassigned = id === 'unassigned';
        const dateLabel = date ? `<div class="milestone-date">${date}</div>` : '';

        return `
            <div class="milestone-column">
                <div class="milestone-header">
                    <div class="milestone-title ${isUnassigned ? '' : 'clickable'}" data-milestone-id="${id}">${this._escapeHtml(title)}</div>
                    ${dateLabel}
                </div>
                ${isUnassigned ?
                '<div class="milestone-placeholder">Features without a milestone</div>' :
                `<textarea 
                        class="milestone-description" 
                        data-milestone-id="${id}"
                        placeholder="Add description..."
                        rows="3">${this._escapeHtml(description)}</textarea>`
            }
                <div class="feature-list" data-milestone-id="${id}">
                    ${features.map(f => this._renderFeatureCard(f)).join('')}
                </div>
                <button class="add-feature-btn" data-milestone-id="${id}">+ Add feature</button>
            </div>
        `;
    }

    private _renderFeatureCard(feature: any): string {
        const priorityBadge = feature.priority ?
            `<span class="feature-priority priority-${feature.priority.toLowerCase()}">${feature.priority}</span>` : '';

        return `
            <div class="feature-card" 
                 draggable="true"
                 data-feature-id="${feature.id}"
                 data-feature-title="${this._escapeHtml(feature.title)}">
                <div class="feature-header">
                    <span class="feature-id">${feature.id}</span>
                    ${priorityBadge}
                </div>
                <div class="feature-title">${this._escapeHtml(feature.title)}</div>
            </div>
        `;
    }

    private _escapeHtml(text: string): string {
        const map: { [key: string]: string } = {
            '&': '&amp;',
            '<': '&lt;',
            '>': '&gt;',
            '"': '&quot;',
            "'": '&#039;',
        };
        return text.replace(/[&<>"']/g, (m) => map[m]);
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
