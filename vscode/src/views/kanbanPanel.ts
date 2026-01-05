import * as vscode from 'vscode';
import { JJJ, Task } from '../jjj';

export class KanbanPanel {
    public static currentPanel: KanbanPanel | undefined;
    private readonly _panel: vscode.WebviewPanel;
    private readonly _extensionUri: vscode.Uri;
    private _disposables: vscode.Disposable[] = [];

    // Default kanban columns
    private readonly columns = ['TODO', 'In Progress', 'Review', 'Done'];
    private selectedMilestone: string | null = null; // null means "All"

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
                    case 'moveTask':
                        try {
                            await this.jjj.moveTask(message.taskId, message.newColumn);
                            vscode.window.showInformationMessage(
                                `Moved ${message.taskId} to ${message.newColumn}`,
                            );
                            this._update(); // Refresh the board
                        } catch (error) {
                            vscode.window.showErrorMessage(`Failed to move task: ${error}`);
                        }
                        return;
                    case 'openTask':
                        vscode.commands.executeCommand('jjj.openTask', message.task);
                        return;
                    case 'refresh':
                        this._update();
                        return;
                    case 'createTask':
                        vscode.commands.executeCommand('jjj.createTask');
                        return;
                    case 'changeMilestone':
                        this.selectedMilestone = message.milestoneId === 'all' ? null : message.milestoneId;
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

        if (KanbanPanel.currentPanel) {
            KanbanPanel.currentPanel._panel.reveal(column);
            return;
        }

        const panel = vscode.window.createWebviewPanel(
            'jjjKanban',
            'JJJ Kanban Board',
            column || vscode.ViewColumn.One,
            {
                enableScripts: true,
                localResourceRoots: [vscode.Uri.joinPath(extensionUri, 'media')],
            },
        );

        KanbanPanel.currentPanel = new KanbanPanel(panel, extensionUri, jjj);
    }

    public dispose() {
        KanbanPanel.currentPanel = undefined;

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
        this._panel.title = 'JJJ Kanban Board';

        let tasks: Task[] = [];
        let bugs: any[] = [];
        let features: any[] = [];

        try {
            [tasks, bugs, features] = await Promise.all([
                this.jjj.listTasks(),
                this.jjj.listBugs(),
                this.jjj.listFeatures(),
            ]);
        } catch (error) {
            console.error(error);
        }

        this._panel.webview.html = this._getHtmlForWebview(webview, tasks, bugs, features);
    }

    private _getHtmlForWebview(webview: vscode.Webview, tasks: Task[], bugs: any[], features: any[]) {
        const nonce = getNonce();

        const styleResetUri = webview.asWebviewUri(
            vscode.Uri.joinPath(this._extensionUri, 'media', 'reset.css'),
        );
        const styleVSCodeUri = webview.asWebviewUri(
            vscode.Uri.joinPath(this._extensionUri, 'media', 'vscode.css'),
        );
        const styleKanbanUri = webview.asWebviewUri(
            vscode.Uri.joinPath(this._extensionUri, 'media', 'kanban.css'),
        );

        // Create feature lookup map
        const featureMap = new Map<string, any>();
        features.forEach((f) => featureMap.set(f.id, f));

        // Map bug status to kanban columns
        const bugStatusToColumn = (status: string): string => {
            switch (status) {
                case 'New':
                case 'Confirmed':
                    return 'TODO';
                case 'InProgress':
                    return 'In Progress';
                case 'Fixed':
                    return 'Review';
                case 'Closed':
                    return 'Done';
                default:
                    return 'Other';
            }
        };

        // Combine tasks and bugs into kanban items
        interface KanbanItem {
            id: string;
            title: string;
            assignee?: string;
            column: string;
            tags: string[];
            featureId?: string;
            type: 'task' | 'bug';
            severity?: string;
        }

        const items: KanbanItem[] = [
            ...tasks.map((t) => ({
                id: t.id,
                title: t.title,
                assignee: t.assignee,
                column: t.column,
                tags: t.tags || [],
                featureId: t.feature_id,
                type: 'task' as const,
            })),
            ...bugs.map((b) => ({
                id: b.id,
                title: b.title,
                assignee: b.assignee,
                column: bugStatusToColumn(b.status),
                tags: [],
                featureId: b.feature_id,
                type: 'bug' as const,
                severity: b.severity,
            })),
        ];

        // Filter items by milestone if one is selected
        const filteredItems = this.selectedMilestone
            ? items.filter((item) => {
                // Item must have a feature
                if (!item.featureId) return false;
                // Feature must exist
                const feature = featureMap.get(item.featureId);
                if (!feature) return false;
                // Feature must belong to selected milestone
                return feature.milestone_id === this.selectedMilestone;
            })
            : items;

        // Group items by assignee and column
        const assignees = new Set<string>();
        const itemsByAssigneeAndColumn = new Map<string, Map<string, KanbanItem[]>>();

        // Collect all assignees
        filteredItems.forEach((item) => {
            const assignee = item.assignee || 'Unassigned';
            assignees.add(assignee);
        });

        // Sort assignees with Unassigned first
        const sortedAssignees = Array.from(assignees).sort((a, b) => {
            if (a === 'Unassigned') return -1;
            if (b === 'Unassigned') return 1;
            return a.localeCompare(b);
        });

        // Initialize structure for each assignee
        sortedAssignees.forEach((assignee) => {
            const columnMap = new Map<string, KanbanItem[]>();
            this.columns.forEach((col) => columnMap.set(col, []));
            columnMap.set('Other', []);
            itemsByAssigneeAndColumn.set(assignee, columnMap);
        });

        // Populate items
        filteredItems.forEach((item) => {
            const assignee = item.assignee || 'Unassigned';
            const column = item.column;
            const assigneeMap = itemsByAssigneeAndColumn.get(assignee)!;

            if (assigneeMap.has(column)) {
                assigneeMap.get(column)!.push(item);
            } else {
                assigneeMap.get('Other')!.push(item);
            }
        });

        return `<!DOCTYPE html>
            <html lang="en">
            <head>
                <meta charset="UTF-8">
                <meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src ${webview.cspSource}; script-src 'nonce-${nonce}';">
                <meta name="viewport" content="width=device-width, initial-scale=1.0">
                <link href="${styleResetUri}" rel="stylesheet">
                <link href="${styleVSCodeUri}" rel="stylesheet">
                <link href="${styleKanbanUri}" rel="stylesheet">
                <title>JJJ Kanban Board</title>
            </head>
            <body>
                <div class="kanban-header">
                    <div class="header-left">
                        <h1>Kanban Board</h1>
                        <select id="milestone-selector" class="milestone-selector">
                            <option value="all" ${!this.selectedMilestone ? 'selected' : ''}>All Milestones</option>
                            ${features
                .map(f => f.milestone_id)
                .filter((mid, idx, arr) => mid && arr.indexOf(mid) === idx) // unique milestone IDs
                .map(milestoneId => {
                    const milestone = features.find(f => f.milestone_id === milestoneId);
                    if (!milestone) return '';
                    const milestoneName = milestoneId; // Would need to fetch actual milestone names
                    return `<option value="${milestoneId}" ${this.selectedMilestone === milestoneId ? 'selected' : ''}>${milestoneName}</option>`;
                })
                .join('')}
                        </select>
                    </div>
                    <button id="refresh-btn" class="refresh-button">↻ Refresh</button>
                </div>

                <div class="kanban-container">
                    <div class="column-headers">
                        <div class="assignee-label-header"></div>
                        ${this.columns.map((col) => `<div class="column-header-label">${col}</div>`).join('')}
                    </div>

                    ${sortedAssignees
                .map((assignee) =>
                    this._renderSwimLane(assignee, itemsByAssigneeAndColumn.get(assignee)!, featureMap),
                )
                .join('')}
                </div>

                <script nonce="${nonce}">
                    const vscode = acquireVsCodeApi();

                    // Refresh button
                    document.getElementById('refresh-btn').addEventListener('click', () => {
                        vscode.postMessage({ command: 'refresh' });
                    });
                    
                    // Milestone selector
                    document.getElementById('milestone-selector').addEventListener('change', (e) => {
                        vscode.postMessage({ 
                            command: 'changeMilestone', 
                            milestoneId: e.target.value 
                        });
                    });
                    
                    // Drag and drop functionality
                    let draggedTaskId = null;
                    
                    // Make tasks draggable
                    document.querySelectorAll('.task-card').forEach(card => {
                        card.addEventListener('dragstart', (e) => {
                            draggedTaskId = e.target.dataset.taskId;
                            e.target.classList.add('dragging');
                        });
                        
                        card.addEventListener('dragend', (e) => {
                            e.target.classList.remove('dragging');
                            draggedTaskId = null;
                        });
                        
                        // Click to open task
                        card.addEventListener('click', (e) => {
                            if (!e.target.classList.contains('task-card')) return;
                            const taskId = e.target.dataset.taskId;
                            const taskTitle = e.target.dataset.taskTitle;
                            vscode.postMessage({ 
                                command: 'openTask', 
                                task: { id: taskId, title: taskTitle } 
                            });
                        });
                    });
                    
                    // Make columns accept drops
                    document.querySelectorAll('.task-list').forEach(list => {
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
                            
                            if (draggedTaskId) {
                                const newColumn = list.dataset.column;
                                vscode.postMessage({
                                    command: 'moveTask',
                                    taskId: draggedTaskId,
                                    newColumn: newColumn
                                });
                            }
                        });
                    });
                    
                    // Add task buttons
                    document.querySelectorAll('.add-task-btn').forEach(btn => {
                        btn.addEventListener('click', (e) => {
                            e.stopPropagation();
                            vscode.postMessage({ command: 'createTask' });
                        });
                    });
                </script>
            </body>
            </html>`;
    }

    private _renderSwimLane(assignee: string, itemsByColumn: Map<string, any[]>, featureMap: Map<string, any>): string {
        return `
            <div class="swim-lane">
                <div class="assignee-label">${this._escapeHtml(assignee)}</div>
                <div class="swim-lane-columns">
                    ${this.columns
                .map((column) => this._renderSwimLaneColumn(column, itemsByColumn.get(column) || [], assignee, featureMap))
                .join('')}
                </div>
            </div>
        `;
    }

    private _renderSwimLaneColumn(columnName: string, items: any[], assignee: string, featureMap: Map<string, any>): string {
        return `
            <div class="swim-lane-column">
                <div class="task-list" data-column="${columnName}">
                    ${items.map((item) => this._renderItem(item, featureMap)).join('')}
                    <button class="add-task-btn" data-column="${columnName}" title="Add task to ${columnName}">
                        + Add task
                    </button>
                </div>
            </div>
        `;
    }

    private _renderColumn(columnName: string, tasks: Task[]): string {
        return `
            <div class="kanban-column">
                <div class="column-header">
                    <h2>${columnName}</h2>
                    <span class="task-count">${tasks.length}</span>
                </div>
                <div class="task-list" data-column="${columnName}">
                    ${tasks.length > 0
                ? tasks.map((task) => this._renderTask(task)).join('')
                : '<div class="empty-column">No tasks</div>'
            }
                </div>
            </div>
        `;
    }

    private _renderItem(item: any, featureMap: Map<string, any>): string {
        const isBug = item.type === 'bug';
        const typeIndicator = isBug ? '🐛' : '';
        const severityBadge = isBug && item.severity
            ? `<span class="severity severity-${item.severity.toLowerCase()}">${item.severity}</span>`
            : '';

        const featureName = item.featureId && featureMap.has(item.featureId)
            ? featureMap.get(item.featureId).title
            : '';
        const featureLabel = featureName
            ? `<div class="feature-label" title="Feature: ${this._escapeHtml(featureName)}">📦 ${this._escapeHtml(featureName)}</div>`
            : '';

        const tagsLabel =
            item.tags && item.tags.length > 0
                ? `<div class="tags">${item.tags.map((tag: string) => `<span class="tag">${this._escapeHtml(tag)}</span>`).join('')}</div>`
                : '';

        return `
            <div class="task-card ${isBug ? 'bug-card' : 'task-card-type'}" 
                 draggable="true" 
                 data-task-id="${item.id}"
                 data-task-title="${this._escapeHtml(item.title)}">
                <div class="task-header">
                    <span class="task-id">${typeIndicator}${item.id}</span>
                    ${severityBadge}
                </div>
                ${featureLabel}
                <div class="task-title">${this._escapeHtml(item.title)}</div>
                ${tagsLabel}
            </div>
        `;
    }

    private _renderTask(task: Task): string {
        const assigneeLabel = task.assignee ? `<span class="assignee">${task.assignee}</span>` : '';
        const tagsLabel =
            task.tags && task.tags.length > 0
                ? `<div class="tags">${task.tags.map((tag) => `<span class="tag">${tag}</span>`).join('')}</div>`
                : '';

        return `
            <div class="task-card" 
                 draggable="true" 
                 data-task-id="${task.id}"
                 data-task-title="${this._escapeHtml(task.title)}">
                <div class="task-header">
                    <span class="task-id">${task.id}</span>
                    ${assigneeLabel}
                </div>
                <div class="task-title">${this._escapeHtml(task.title)}</div>
                ${tagsLabel}
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
