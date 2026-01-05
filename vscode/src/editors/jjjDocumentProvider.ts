import * as vscode from 'vscode';
import * as yaml from 'js-yaml';
import { JJJ, Task, Feature, Milestone, Bug } from '../jjj';

/**
 * Virtual file system provider for editing JJJ items (tasks, features, milestones, bugs)
 * Items are edited as YAML files with validation before saving
 */
export class JJJFileSystemProvider implements vscode.FileSystemProvider {
  private _emitter = new vscode.EventEmitter<vscode.FileChangeEvent[]>();
  readonly onDidChangeFile: vscode.Event<vscode.FileChangeEvent[]> = this._emitter.event;

  private documents = new Map<string, Uint8Array>();

  constructor(private jjj: JJJ) { }

  /**
   * Parse URI to extract type and ID
   * Format: jjj:///task/T-1.yaml -> ['task', 'T-1']
   */
  private parseUri(uri: vscode.Uri): { type: string; id: string } {
    // Path is like "/task/T-1.yaml"
    const parts = uri.path.split('/').filter(Boolean);

    if (parts.length < 2) {
      throw new Error(`Invalid URI format: ${uri.toString()}`);
    }

    const type = parts[0];
    let id = parts[1];

    // Remove .yaml extension if present
    if (id.endsWith('.yaml')) {
      id = id.slice(0, -5);
    }

    return { type, id };
  }

  watch(
    uri: vscode.Uri,
    options: { recursive: boolean; excludes: string[] },
  ): vscode.Disposable {
    // Not implementing file watching for now
    return new vscode.Disposable(() => { });
  }

  stat(uri: vscode.Uri): vscode.FileStat {
    return {
      type: vscode.FileType.File,
      ctime: Date.now(),
      mtime: Date.now(),
      size: 0,
    };
  }

  readDirectory(uri: vscode.Uri): [string, vscode.FileType][] {
    throw vscode.FileSystemError.NoPermissions('Reading directories is not supported');
  }

  createDirectory(uri: vscode.Uri): void {
    throw vscode.FileSystemError.NoPermissions('Creating directories is not supported');
  }

  async readFile(uri: vscode.Uri): Promise<Uint8Array> {
    const cached = this.documents.get(uri.toString());
    if (cached) {
      return cached;
    }

    const content = await this.loadContent(uri);
    const bytes = Buffer.from(content, 'utf8');
    this.documents.set(uri.toString(), bytes);
    return bytes;
  }

  async writeFile(
    uri: vscode.Uri,
    content: Uint8Array,
    options: { create: boolean; overwrite: boolean },
  ): Promise<void> {
    const text = Buffer.from(content).toString('utf8');
    await this.saveDocument(uri, text);
    this.documents.set(uri.toString(), content);
    this._emitter.fire([{ type: vscode.FileChangeType.Changed, uri }]);
  }

  delete(uri: vscode.Uri, options: { recursive: boolean }): void {
    throw vscode.FileSystemError.NoPermissions('Deleting is not supported');
  }

  rename(oldUri: vscode.Uri, newUri: vscode.Uri, options: { overwrite: boolean }): void {
    throw vscode.FileSystemError.NoPermissions('Renaming is not supported');
  }

  async loadContent(uri: vscode.Uri): Promise<string> {
    const { type, id } = this.parseUri(uri);

    try {
      let content: any;

      switch (type) {
        case 'task': {
          content = await this.jjj.listTasks();
          const task = content.find((t: Task) => t.id === id);
          if (!task) {
            throw new Error(`Task ${id} not found`);
          }
          return this.formatTask(task);
        }

        case 'feature':
          content = await this.jjj.getFeature(id);
          return this.formatFeature(content);

        case 'milestone':
          content = await this.jjj.getMilestone(id);
          return this.formatMilestone(content);

        case 'bug':
          content = await this.jjj.getBug(id);
          return this.formatBug(content);

        default:
          throw new Error(`Unknown type: ${type}`);
      }
    } catch (error) {
      return `# Error loading ${type} ${id}\n# ${error}`;
    }
  }

  private formatTask(task: Task): string {
    const data = {
      id: task.id,
      title: task.title,
      feature_id: task.feature_id,
      column: task.column,
      assignee: task.assignee || null,
      tags: task.tags,
      change_ids: task.change_ids,
      comment_count: task.comment_count,
      version: task.version,
    };

    return this.toYAML(data, 'Task');
  }

  private formatFeature(feature: Feature): string {
    const data = {
      id: feature.id,
      title: feature.title,
      milestone_id: feature.milestone_id || null,
      status: feature.status,
      priority: feature.priority,
      assignee: feature.assignee || null,
      story_points: feature.story_points || null,
      task_ids: feature.task_ids,
      bug_ids: feature.bug_ids,
      created_at: feature.created_at,
      updated_at: feature.updated_at,
    };

    return this.toYAML(data, 'Feature');
  }

  private formatMilestone(milestone: Milestone): string {
    const data = {
      id: milestone.id,
      title: milestone.title,
      description: milestone.description || null,
      target_date: milestone.target_date || null,
      status: milestone.status,
      version: milestone.version || null,
      feature_ids: milestone.feature_ids,
      bug_ids: milestone.bug_ids,
      tags: milestone.tags,
      created_at: milestone.created_at,
      updated_at: milestone.updated_at,
    };

    return this.toYAML(data, 'Milestone');
  }

  private formatBug(bug: Bug): string {
    const data = {
      id: bug.id,
      title: bug.title,
      severity: bug.severity,
      status: bug.status,
      feature_id: bug.feature_id || null,
      milestone_id: bug.milestone_id || null,
      assignee: bug.assignee || null,
      reporter: bug.reporter || null,
      repro_steps: bug.repro_steps || null,
      created_at: bug.created_at,
      updated_at: bug.updated_at,
    };

    return this.toYAML(data, 'Bug');
  }

  private toYAML(data: any, type: string): string {
    const header = `# JJJ ${type}\n# Edit and save to update\n# Fields marked with * are required\n\n`;
    return header + yaml.dump(data, { indent: 2, lineWidth: 120 });
  }

  async saveDocument(uri: vscode.Uri, content: string): Promise<void> {
    try {
      const { type, id } = this.parseUri(uri);

      // Parse YAML content
      const data = yaml.load(content) as any;

      // Validate and save based on type
      switch (type) {
        case 'task':
          await this.saveTask(id, data);
          break;
        case 'feature':
          await this.saveFeature(id, data);
          break;
        case 'milestone':
          await this.saveMilestone(id, data);
          break;
        case 'bug':
          await this.saveBug(id, data);
          break;
        default:
          throw new Error(`Unknown type: ${type}`);
      }
    } catch (error) {
      vscode.window.showErrorMessage(`Failed to save: ${error}`);
      throw error;
    }
  }

  private async saveTask(id: string, data: any): Promise<void> {
    // Validate required fields
    this.validateRequired(data, ['id', 'title', 'feature_id', 'column'], 'Task');

    // Validate column values
    const validColumns = ['Backlog', 'In Progress', 'Review', 'Done'];
    if (!validColumns.includes(data.column)) {
      throw new Error(`Invalid column: ${data.column}. Must be one of: ${validColumns.join(', ')}`);
    }

    // Call jjj CLI to update task
    // Note: This would need actual CLI commands for updating
    // For now, we'll show a message
    vscode.window.showInformationMessage(`Task ${id} would be updated with new values`);
  }

  private async saveFeature(id: string, data: any): Promise<void> {
    // Validate required fields
    this.validateRequired(data, ['id', 'title', 'status', 'priority'], 'Feature');

    // Validate status
    const validStatuses = ['backlog', 'inprogress', 'review', 'done', 'blocked'];
    if (!validStatuses.includes(data.status)) {
      throw new Error(
        `Invalid status: ${data.status}. Must be one of: ${validStatuses.join(', ')}`,
      );
    }

    // Validate priority
    const validPriorities = ['low', 'medium', 'high', 'critical'];
    if (!validPriorities.includes(data.priority)) {
      throw new Error(
        `Invalid priority: ${data.priority}. Must be one of: ${validPriorities.join(', ')}`,
      );
    }

    vscode.window.showInformationMessage(`Feature ${id} would be updated with new values`);
  }

  private async saveMilestone(id: string, data: any): Promise<void> {
    // Validate required fields
    this.validateRequired(data, ['id', 'title', 'status'], 'Milestone');

    // Validate status
    const validStatuses = ['planning', 'active', 'released', 'cancelled'];
    if (!validStatuses.includes(data.status)) {
      throw new Error(
        `Invalid status: ${data.status}. Must be one of: ${validStatuses.join(', ')}`,
      );
    }

    // Validate target_date format if provided
    if (data.target_date && !/^\d{4}-\d{2}-\d{2}$/.test(data.target_date)) {
      throw new Error(`Invalid target_date format: ${data.target_date}. Must be YYYY-MM-DD`);
    }

    vscode.window.showInformationMessage(`Milestone ${id} would be updated with new values`);
  }

  private async saveBug(id: string, data: any): Promise<void> {
    // Validate required fields
    this.validateRequired(data, ['id', 'title', 'severity', 'status'], 'Bug');

    // Validate severity
    const validSeverities = ['low', 'medium', 'high', 'critical'];
    if (!validSeverities.includes(data.severity)) {
      throw new Error(
        `Invalid severity: ${data.severity}. Must be one of: ${validSeverities.join(', ')}`,
      );
    }

    // Validate status
    const validStatuses = [
      'new',
      'confirmed',
      'inprogress',
      'fixed',
      'closed',
      'wontfix',
      'duplicate',
    ];
    if (!validStatuses.includes(data.status)) {
      throw new Error(
        `Invalid status: ${data.status}. Must be one of: ${validStatuses.join(', ')}`,
      );
    }

    vscode.window.showInformationMessage(`Bug ${id} would be updated with new values`);
  }

  private validateRequired(data: any, fields: string[], type: string): void {
    const missing = fields.filter((field) => !data[field]);
    if (missing.length > 0) {
      throw new Error(`${type} is missing required fields: ${missing.join(', ')}`);
    }
  }

  refresh(): void {
    this.documents.clear();
  }
}
