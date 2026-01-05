import { JJJ, Task, Review, Feature, Milestone, Bug, DashboardData } from '../../jjj';
import {
  mockTasks,
  mockFeatures,
  mockMilestones,
  mockBugs,
  mockReviews,
  mockDashboard,
} from '../fixtures/mockData';

export class MockJJJ extends JJJ {
  constructor() {
    super('/mock/workspace');
  }

  async init(): Promise<void> {
    return Promise.resolve();
  }

  async listTasks(): Promise<Task[]> {
    return Promise.resolve([...mockTasks]);
  }

  async listReviews(): Promise<Review[]> {
    return Promise.resolve([...mockReviews]);
  }

  async getDashboard(): Promise<DashboardData> {
    return Promise.resolve({ ...mockDashboard });
  }

  async moveTask(taskId: string, column: string): Promise<void> {
    return Promise.resolve();
  }

  async createTask(title: string, feature: string, tags: string[]): Promise<void> {
    return Promise.resolve();
  }

  async listFeatures(options?: { milestone?: string; status?: string }): Promise<Feature[]> {
    let features = [...mockFeatures];

    if (options?.milestone) {
      features = features.filter((f) => f.milestone_id === options.milestone);
    }

    if (options?.status) {
      features = features.filter((f) => f.status === options.status);
    }

    return Promise.resolve(features);
  }

  async createFeature(
    title: string,
    options?: { milestone?: string; priority?: string },
  ): Promise<void> {
    return Promise.resolve();
  }

  async getFeature(featureId: string): Promise<Feature> {
    const feature = mockFeatures.find((f) => f.id === featureId);
    if (!feature) {
      throw new Error(`Feature ${featureId} not found`);
    }
    return Promise.resolve({ ...feature });
  }

  async listMilestones(status?: string): Promise<Milestone[]> {
    let milestones = [...mockMilestones];

    if (status) {
      milestones = milestones.filter((m) => m.status === status);
    }

    return Promise.resolve(milestones);
  }

  async createMilestone(
    title: string,
    options?: { date?: string; description?: string },
  ): Promise<void> {
    return Promise.resolve();
  }

  async getMilestone(milestoneId: string): Promise<Milestone> {
    const milestone = mockMilestones.find((m) => m.id === milestoneId);
    if (!milestone) {
      throw new Error(`Milestone ${milestoneId} not found`);
    }
    return Promise.resolve({ ...milestone });
  }

  async listBugs(options?: { severity?: string; open?: boolean }): Promise<Bug[]> {
    let bugs = [...mockBugs];

    if (options?.severity) {
      bugs = bugs.filter((b) => b.severity === options.severity);
    }

    if (options?.open) {
      bugs = bugs.filter((b) => ['New', 'Confirmed', 'InProgress'].includes(b.status));
    }

    return Promise.resolve(bugs);
  }

  async createBug(title: string, options?: { severity?: string; feature?: string }): Promise<void> {
    return Promise.resolve();
  }

  async getBug(bugId: string): Promise<Bug> {
    const bug = mockBugs.find((b) => b.id === bugId);
    if (!bug) {
      throw new Error(`Bug ${bugId} not found`);
    }
    return Promise.resolve({ ...bug });
  }
}
