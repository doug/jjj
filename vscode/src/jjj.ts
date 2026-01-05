import * as cp from 'child_process';
import * as path from 'path';
import * as vscode from 'vscode';

export interface Task {
  id: string;
  title: string;
  feature_id: string;
  column: string;
  tags: string[];
  assignee?: string;
  change_ids: string[];
  comment_count: number;
  version: number;
}

export interface Review {
  change_id: string;
  author: string;
  reviewers: string[];
  status: string;
  requested_at: string;
  updated_at: string;
  comment_count: number;
  is_stack: boolean;
}

export interface Feature {
  id: string;
  title: string;
  milestone_id?: string;
  status: 'Backlog' | 'InProgress' | 'Review' | 'Done' | 'Blocked';
  priority: 'Low' | 'Medium' | 'High' | 'Critical';
  task_ids: string[];
  bug_ids: string[];
  assignee?: string;
  story_points?: number;
  created_at: string;
  updated_at: string;
}

export interface Milestone {
  id: string;
  title: string;
  description?: string;
  target_date?: string;
  status: 'Planning' | 'Active' | 'Released' | 'Cancelled';
  feature_ids: string[];
  bug_ids: string[];
  tags: string[];
  version?: string;
  created_at: string;
  updated_at: string;
}

export interface Bug {
  id: string;
  title: string;
  severity: 'Low' | 'Medium' | 'High' | 'Critical';
  status: 'New' | 'Confirmed' | 'InProgress' | 'Fixed' | 'Closed' | 'WontFix' | 'Duplicate';
  feature_id?: string;
  milestone_id?: string;
  assignee?: string;
  reporter?: string;
  repro_steps?: string;
  created_at: string;
  updated_at: string;
}

export interface DashboardData {
  my_tasks: Task[];
  pending_reviews: Review[];
  my_reviews: Review[];
}

export class JJJ {
  private workspaceRoot: string;

  constructor(workspaceRoot: string) {
    this.workspaceRoot = workspaceRoot;
  }

  private async exec(args: string[]): Promise<string> {
    return new Promise((resolve, reject) => {
      // Use jjj from the system path
      const command = `jjj ${args.join(' ')}`;

      cp.exec(command, { cwd: this.workspaceRoot }, (error, stdout, stderr) => {
        if (error) {
          console.error(`jjj error: ${stderr}`);
          reject(stderr);
          return;
        }
        resolve(stdout.trim());
      });
    });
  }

  async init(): Promise<void> {
    await this.exec(['init']);
  }

  async listTasks(): Promise<Task[]> {
    const output = await this.exec(['task', 'list', '--json']);
    return JSON.parse(output);
  }

  async listReviews(): Promise<Review[]> {
    const output = await this.exec(['review', 'list', '--json']);
    return JSON.parse(output);
  }

  async getDashboard(): Promise<DashboardData> {
    const output = await this.exec(['dashboard', '--json']);
    return JSON.parse(output);
  }

  async moveTask(taskId: string, column: string): Promise<void> {
    await this.exec(['task', 'move', taskId, `"${column}"`]);
  }

  async createTask(title: string, feature: string, tags: string[]): Promise<void> {
    const args = ['task', 'new', `"${title}"`, '--feature', `"${feature}"`];
    for (const tag of tags) {
      args.push('--tag', tag);
    }
    await this.exec(args);
  }

  async listFeatures(options?: { milestone?: string; status?: string }): Promise<Feature[]> {
    const args = ['feature', 'list', '--json'];
    if (options?.milestone) {
      args.push('--milestone', options.milestone);
    }
    if (options?.status) {
      args.push('--status', options.status);
    }
    const output = await this.exec(args);
    return JSON.parse(output);
  }

  async createFeature(
    title: string,
    options?: { milestone?: string; priority?: string },
  ): Promise<void> {
    const args = ['feature', 'new', `"${title}"`];
    if (options?.milestone) {
      args.push('--milestone', options.milestone);
    }
    if (options?.priority) {
      args.push('--priority', options.priority);
    }
    await this.exec(args);
  }

  async getFeature(featureId: string): Promise<Feature> {
    const output = await this.exec(['feature', 'show', featureId, '--json']);
    return JSON.parse(output);
  }

  async listMilestones(status?: string): Promise<Milestone[]> {
    const args = ['milestone', 'list', '--json'];
    if (status) {
      args.push('--status', status);
    }
    const output = await this.exec(args);
    return JSON.parse(output);
  }

  async createMilestone(
    title: string,
    options?: { date?: string; description?: string },
  ): Promise<void> {
    const args = ['milestone', 'new', `"${title}"`];
    if (options?.date) {
      args.push('--date', options.date);
    }
    if (options?.description) {
      args.push('--description', `"${options.description}"`);
    }
    await this.exec(args);
  }

  async getMilestone(milestoneId: string): Promise<Milestone> {
    const output = await this.exec(['milestone', 'show', milestoneId, '--json']);
    return JSON.parse(output);
  }

  async listBugs(options?: { severity?: string; open?: boolean }): Promise<Bug[]> {
    const args = ['bug', 'list', '--json'];
    if (options?.severity) {
      args.push('--severity', options.severity);
    }
    if (options?.open) {
      args.push('--open');
    }
    const output = await this.exec(args);
    return JSON.parse(output);
  }

  async createBug(title: string, options?: { severity?: string; feature?: string }): Promise<void> {
    const args = ['bug', 'new', `"${title}"`];
    if (options?.severity) {
      args.push('--severity', options.severity);
    }
    if (options?.feature) {
      args.push('--feature', options.feature);
    }
    await this.exec(args);
  }

  async getBug(bugId: string): Promise<Bug> {
    const output = await this.exec(['bug', 'show', bugId, '--json']);
    return JSON.parse(output);
  }
}
