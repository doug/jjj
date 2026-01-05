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
            // Assuming jjj is in the path or we can find it relative to the workspace for dev
            // For development, we'll try to find the binary in the target directory
            const devBinaryPath = path.join(this.workspaceRoot, '..', 'target', 'release', 'jjj');
            const command = `"${devBinaryPath}" ${args.join(' ')}`;

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
        let args = ['task', 'new', `"${title}"`, '--feature', `"${feature}"`];
        for (const tag of tags) {
            args.push('--tag', tag);
        }
        await this.exec(args);
    }
}
