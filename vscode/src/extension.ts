import * as vscode from 'vscode';
import * as fs from 'fs';
import * as path from 'path';
import { exec } from 'child_process';
import { JJJ } from './jjj';
import { TaskProvider } from './views/taskProvider';
import { ReviewProvider } from './views/reviewProvider';
import { FeatureProvider } from './views/featureProvider';
import { MilestoneProvider } from './views/milestoneProvider';
import { BugProvider } from './views/bugProvider';
import { DashboardPanel } from './views/dashboardPanel';
import { KanbanPanel } from './views/kanbanPanel';
import { PlanningPanel } from './views/planningPanel';
import { JJJFileSystemProvider } from './editors/jjjDocumentProvider';

// This function is called when your extension is activated.
export function activate(context: vscode.ExtensionContext) {
  console.log('Congratulations, your extension "jjj" is now active!');

  let jjj: JJJ | undefined;
  let taskProvider: TaskProvider | undefined;
  let reviewProvider: ReviewProvider | undefined;
  let featureProvider: FeatureProvider | undefined;
  let milestoneProvider: MilestoneProvider | undefined;
  let bugProvider: BugProvider | undefined;
  let documentProvider: JJJFileSystemProvider | undefined;

  // Create status bar item
  const statusBarItem = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Left, 100);
  statusBarItem.command = 'jjj.openDashboard';
  context.subscriptions.push(statusBarItem);

  // Function to update status bar
  const updateStatusBar = async () => {
    if (!jjj) {
      statusBarItem.hide();
      return;
    }

    try {
      const dashboard = await jjj.getDashboard();
      const taskCount = dashboard.my_tasks.length;
      const reviewCount = dashboard.pending_reviews.length;

      statusBarItem.text = `$(beaker) JJJ: ${taskCount} tasks, ${reviewCount} reviews`;
      statusBarItem.tooltip = `Tasks: ${taskCount}\nPending Reviews: ${reviewCount}\nMy Reviews: ${dashboard.my_reviews.length}\n\nClick to open dashboard`;
      statusBarItem.show();
    } catch (error) {
      statusBarItem.text = '$(beaker) JJJ';
      statusBarItem.tooltip = 'JJJ not initialized';
      statusBarItem.show();
    }
  };

  // Function to check for jjj initialization and set context
  const checkJjjInitialization = () => {
    const workspaceFolders = vscode.workspace.workspaceFolders;
    if (workspaceFolders) {
      const workspacePath = workspaceFolders[0].uri.fsPath;
      // This might need to be adjusted based on how jjj init works now
      // Actually, jjj init creates a bookmark, so we should check if jjj binary works or if we can detect the bookmark
      // For now, let's assume if we can run `jjj board --json` it's initialized.

      jjj = new JJJ(workspacePath);

      jjj
        .listTasks()
        .then(() => {
          vscode.commands.executeCommand('setContext', 'jjj.initialized', true);

          taskProvider = new TaskProvider(jjj!);
          reviewProvider = new ReviewProvider(jjj!);
          featureProvider = new FeatureProvider(jjj!);
          milestoneProvider = new MilestoneProvider(jjj!);
          bugProvider = new BugProvider(jjj!);

          vscode.window.registerTreeDataProvider('jjj-tasks', taskProvider);
          vscode.window.registerTreeDataProvider('jjj-reviews', reviewProvider);
          vscode.window.registerTreeDataProvider('jjj-features', featureProvider);
          vscode.window.registerTreeDataProvider('jjj-milestones', milestoneProvider);
          vscode.window.registerTreeDataProvider('jjj-bugs', bugProvider);

          // Register file system provider for editing items
          documentProvider = new JJJFileSystemProvider(jjj!);
          context.subscriptions.push(
            vscode.workspace.registerFileSystemProvider('jjj', documentProvider, {
              isCaseSensitive: true,
            }),
          );

          updateStatusBar();
        })
        .catch(() => {
          vscode.commands.executeCommand('setContext', 'jjj.initialized', false);
          statusBarItem.hide();
        });
    } else {
      vscode.commands.executeCommand('setContext', 'jjj.initialized', false);
    }
  };

  // Initial check
  checkJjjInitialization();

  // Register save handler for jjj documents to refresh views
  context.subscriptions.push(
    vscode.workspace.onDidSaveTextDocument(async (document) => {
      if (document.uri.scheme === 'jjj') {
        // Refresh providers after successful save
        taskProvider?.refresh();
        featureProvider?.refresh();
        milestoneProvider?.refresh();
        bugProvider?.refresh();
        KanbanPanel.currentPanel?.refresh();
        PlanningPanel.currentPanel?.refresh();
        updateStatusBar();
      }
    }),
  );

  // Helper function to open a jjj document
  async function openJJJDocument(type: string, id: string, title: string) {
    if (!documentProvider) {
      vscode.window.showErrorMessage('JJJ not initialized');
      return;
    }

    const uri = vscode.Uri.parse(`jjj:///${type}/${id}.yaml`);

    try {
      // Open document
      const doc = await vscode.workspace.openTextDocument(uri);
      await vscode.window.showTextDocument(doc, {
        preview: false,
        viewColumn: vscode.ViewColumn.Active,
      });

      // Set language mode to YAML
      await vscode.languages.setTextDocumentLanguage(doc, 'yaml');
    } catch (error) {
      vscode.window.showErrorMessage(`Failed to open ${type} ${id}: ${error}`);
    }
  }

  // Register a command to initialize the repository
  const initCommand = vscode.commands.registerCommand('jjj.init', async () => {
    const workspaceFolders = vscode.workspace.workspaceFolders;
    if (workspaceFolders) {
      const workspacePath = workspaceFolders[0].uri.fsPath;
      const tempJjj = new JJJ(workspacePath);
      try {
        await tempJjj.init();
        vscode.window.showInformationMessage('JJJ repository initialized successfully.');
        checkJjjInitialization();
      } catch (error) {
        vscode.window.showErrorMessage(`Failed to initialize JJJ repository: ${error}`);
      }
    } else {
      vscode.window.showErrorMessage('No workspace folder open to initialize JJJ.');
    }
  });

  context.subscriptions.push(initCommand);

  // Register command to open a task
  context.subscriptions.push(
    vscode.commands.registerCommand('jjj.openTask', (task) => {
      openJJJDocument('task', task.id, task.title);
    }),
  );

  // Register command to open a review
  context.subscriptions.push(
    vscode.commands.registerCommand('jjj.openReview', (review) => {
      vscode.window.showInformationMessage(`Opening review: ${review.change_id}`);
    }),
  );

  // Register command to open a feature
  context.subscriptions.push(
    vscode.commands.registerCommand('jjj.openFeature', (feature) => {
      openJJJDocument('feature', feature.id, feature.title);
    }),
  );

  // Register command to open a milestone
  context.subscriptions.push(
    vscode.commands.registerCommand('jjj.openMilestone', (milestone) => {
      openJJJDocument('milestone', milestone.id, milestone.title);
    }),
  );

  // Register command to open a bug
  context.subscriptions.push(
    vscode.commands.registerCommand('jjj.openBug', (bug) => {
      openJJJDocument('bug', bug.id, bug.title);
    }),
  );

  // Register command to open dashboard
  context.subscriptions.push(
    vscode.commands.registerCommand('jjj.openDashboard', () => {
      if (jjj) {
        DashboardPanel.createOrShow(context.extensionUri, jjj);
      } else {
        vscode.window.showErrorMessage('JJJ not initialized');
      }
    }),
  );

  // Register command to open kanban board
  context.subscriptions.push(
    vscode.commands.registerCommand('jjj.openKanban', () => {
      if (jjj) {
        KanbanPanel.createOrShow(context.extensionUri, jjj);
      } else {
        vscode.window.showErrorMessage('JJJ not initialized');
      }
    }),
  );

  // Register command to open planning view
  context.subscriptions.push(
    vscode.commands.registerCommand('jjj.openPlanning', () => {
      if (jjj) {
        PlanningPanel.createOrShow(context.extensionUri, jjj);
      } else {
        vscode.window.showErrorMessage('JJJ not initialized');
      }
    }),
  );

  // Register create commands
  context.subscriptions.push(
    vscode.commands.registerCommand('jjj.createTask', async () => {
      if (!jjj) {
        vscode.window.showErrorMessage('JJJ not initialized');
        return;
      }

      try {
        // 1. Get Title
        const title = await vscode.window.showInputBox({
          prompt: 'Enter task title',
          placeHolder: 'e.g., Implement login page',
        });
        if (!title) return;

        // 2. Get Feature
        const features = await jjj.listFeatures();
        const featureItems = features.map((f) => ({
          label: f.title,
          description: f.id,
          detail: f.status,
        }));

        const selectedFeature = await vscode.window.showQuickPick(featureItems, {
          placeHolder: 'Select parent feature',
        });
        if (!selectedFeature) return;

        // 3. Get Tags
        const tagsInput = await vscode.window.showInputBox({
          prompt: 'Enter tags (comma separated)',
          placeHolder: 'e.g., frontend, auth',
        });
        const tags = tagsInput
          ? tagsInput
            .split(',')
            .map((t) => t.trim())
            .filter((t) => t.length > 0)
          : [];

        // Create Task
        await jjj.createTask(title, selectedFeature.description, tags);
        vscode.window.showInformationMessage(`Created task: ${title}`);
        taskProvider?.refresh();
        KanbanPanel.currentPanel?.refresh();
        PlanningPanel.currentPanel?.refresh();
      } catch (error) {
        vscode.window.showErrorMessage(`Failed to create task: ${error}`);
      }
    }),
  );

  context.subscriptions.push(
    vscode.commands.registerCommand('jjj.createFeature', async () => {
      if (!jjj) {
        vscode.window.showErrorMessage('JJJ not initialized');
        return;
      }

      try {
        // 1. Get Title
        const title = await vscode.window.showInputBox({
          prompt: 'Enter feature title',
          placeHolder: 'e.g., User Authentication',
        });
        if (!title) return;

        // 2. Get Milestone (Optional)
        const milestones = await jjj.listMilestones();
        const milestoneItems = milestones.map((m) => ({
          label: m.title,
          description: m.id,
        }));
        // Add "None" option
        milestoneItems.unshift({ label: 'None', description: '' });

        const selectedMilestone = await vscode.window.showQuickPick(milestoneItems, {
          placeHolder: 'Select milestone (optional)',
        });

        const milestoneId =
          selectedMilestone && selectedMilestone.description
            ? selectedMilestone.description
            : undefined;

        // 3. Get Priority
        const priorities = ['Low', 'Medium', 'High', 'Critical'];
        const selectedPriority = await vscode.window.showQuickPick(priorities, {
          placeHolder: 'Select priority',
        });
        if (!selectedPriority) return;

        // Create Feature
        await jjj.createFeature(title, {
          milestone: milestoneId,
          priority: selectedPriority,
        });
        vscode.window.showInformationMessage(`Created feature: ${title}`);
        featureProvider?.refresh();
        KanbanPanel.currentPanel?.refresh();
        PlanningPanel.currentPanel?.refresh();
      } catch (error) {
        vscode.window.showErrorMessage(`Failed to create feature: ${error}`);
      }
    }),
  );

  context.subscriptions.push(
    vscode.commands.registerCommand('jjj.createBug', async () => {
      if (!jjj) {
        vscode.window.showErrorMessage('JJJ not initialized');
        return;
      }

      try {
        // 1. Get Title
        const title = await vscode.window.showInputBox({
          prompt: 'Enter bug title',
          placeHolder: 'e.g., Login button not working',
        });
        if (!title) return;

        // 2. Get Severity
        const severities = ['Low', 'Medium', 'High', 'Critical'];
        const selectedSeverity = await vscode.window.showQuickPick(severities, {
          placeHolder: 'Select severity',
        });
        if (!selectedSeverity) return;

        // 3. Get Feature (Optional)
        const features = await jjj.listFeatures();
        const featureItems = features.map((f) => ({
          label: f.title,
          description: f.id,
        }));
        featureItems.unshift({ label: 'None', description: '' });

        const selectedFeature = await vscode.window.showQuickPick(featureItems, {
          placeHolder: 'Select related feature (optional)',
        });

        const featureId =
          selectedFeature && selectedFeature.description ? selectedFeature.description : undefined;

        // Create Bug
        await jjj.createBug(title, {
          severity: selectedSeverity,
          feature: featureId,
        });
        vscode.window.showInformationMessage(`Created bug: ${title}`);
        bugProvider?.refresh();
        KanbanPanel.currentPanel?.refresh();
        PlanningPanel.currentPanel?.refresh();
      } catch (error) {
        vscode.window.showErrorMessage(`Failed to create bug: ${error}`);
      }
    }),
  );

  context.subscriptions.push(
    vscode.commands.registerCommand('jjj.createMilestone', async () => {
      if (!jjj) {
        vscode.window.showErrorMessage('JJJ not initialized');
        return;
      }

      try {
        // 1. Get Title
        const title = await vscode.window.showInputBox({
          prompt: 'Enter milestone title',
          placeHolder: 'e.g., v1.0 Release',
        });
        if (!title) return;

        // 2. Get Date (Optional)
        const date = await vscode.window.showInputBox({
          prompt: 'Target date (YYYY-MM-DD) (optional)',
          placeHolder: 'e.g., 2023-12-31',
        });

        // 3. Get Description (Optional)
        const description = await vscode.window.showInputBox({
          prompt: 'Description (optional)',
          placeHolder: 'e.g., First public release',
        });

        // Create Milestone
        await jjj.createMilestone(title, {
          date: date || undefined,
          description: description || undefined,
        });
        vscode.window.showInformationMessage(`Created milestone: ${title}`);
        milestoneProvider?.refresh();
        KanbanPanel.currentPanel?.refresh();
        PlanningPanel.currentPanel?.refresh();
      } catch (error) {
        vscode.window.showErrorMessage(`Failed to create milestone: ${error}`);
      }
    }),
  );

  // Register refresh commands
  context.subscriptions.push(
    vscode.commands.registerCommand('jjj.refreshTasks', () => {
      taskProvider?.refresh();
    }),
  );

  context.subscriptions.push(
    vscode.commands.registerCommand('jjj.refreshReviews', () => {
      reviewProvider?.refresh();
    }),
  );

  context.subscriptions.push(
    vscode.commands.registerCommand('jjj.refreshFeatures', () => {
      featureProvider?.refresh();
    }),
  );

  context.subscriptions.push(
    vscode.commands.registerCommand('jjj.refreshMilestones', () => {
      milestoneProvider?.refresh();
    }),
  );

  context.subscriptions.push(
    vscode.commands.registerCommand('jjj.refreshBugs', () => {
      bugProvider?.refresh();
    }),
  );

  context.subscriptions.push(
    vscode.commands.registerCommand('jjj.refreshAll', () => {
      taskProvider?.refresh();
      reviewProvider?.refresh();
      featureProvider?.refresh();
      milestoneProvider?.refresh();
      bugProvider?.refresh();
      updateStatusBar();
    }),
  );

  context.subscriptions.push(
    vscode.commands.registerCommand('jjj.toggleTaskGrouping', () => {
      taskProvider?.toggleGrouping();
    }),
  );

  // Watch for changes in the workspace that might indicate initialization
  // This is tricky with bookmarks, maybe we just poll or rely on user action for now
}

// This function is called when your extension is deactivated
// eslint-disable-next-line @typescript-eslint/no-empty-function
export function deactivate() { }
