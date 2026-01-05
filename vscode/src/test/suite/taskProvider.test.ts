import * as assert from 'assert';
import * as vscode from 'vscode';
import { TaskProvider, TaskItem, TaskGroup } from '../../views/taskProvider';
import { MockJJJ } from '../mocks/mockJJJ';
import { snapshotTest } from '../utils/snapshotUtils';

suite('TaskProvider Test Suite', () => {
  let mockJJJ: MockJJJ;
  let taskProvider: TaskProvider;

  setup(() => {
    mockJJJ = new MockJJJ();
    taskProvider = new TaskProvider(mockJJJ);
  });

  test('Should load tasks from JJJ', async () => {
    const children = await taskProvider.getChildren();
    assert.ok(children, 'Should return children');
    assert.ok(children.length > 0, 'Should have at least one child');
  });

  test('Should group tasks by column by default', async () => {
    const children = await taskProvider.getChildren();

    // All root children should be TaskGroups
    assert.ok(
      children.every((child) => child instanceof TaskGroup),
      'All root children should be TaskGroups',
    );

    const groups = children as TaskGroup[];
    const columnNames = groups.map((g) => g.column);

    // Check that we have expected columns
    assert.ok(columnNames.includes('In Progress'), 'Should have In Progress column');
    assert.ok(columnNames.includes('Done'), 'Should have Done column');
    assert.ok(columnNames.includes('Backlog'), 'Should have Backlog column');
  });

  test('Should show flat list when grouping is disabled', async () => {
    taskProvider.toggleGrouping(); // Disable grouping

    const children = await taskProvider.getChildren();

    // All children should be TaskItems
    assert.ok(
      children.every((child) => child instanceof TaskItem),
      'All children should be TaskItems',
    );
    assert.strictEqual(children.length, 4, 'Should have 4 tasks');
  });

  test('TaskGroup should contain correct tasks', async () => {
    const children = (await taskProvider.getChildren()) as TaskGroup[];

    const inProgressGroup = children.find((g) => g.column === 'In Progress');
    assert.ok(inProgressGroup, 'Should have In Progress group');
    assert.strictEqual(inProgressGroup.tasks.length, 2, 'In Progress should have 2 tasks');

    const groupChildren = await taskProvider.getChildren(inProgressGroup);
    assert.ok(
      groupChildren.every((child) => child instanceof TaskItem),
      'Group children should be TaskItems',
    );
    assert.strictEqual(groupChildren.length, 2, 'Should return 2 task items');
  });

  test('TaskItem should have correct properties', async () => {
    taskProvider.toggleGrouping(); // Disable grouping for flat list

    const children = (await taskProvider.getChildren()) as TaskItem[];
    const firstTask = children[0];

    assert.ok(firstTask.task, 'TaskItem should have task property');
    assert.strictEqual(firstTask.task.id, 'T-1', 'Should have correct task ID');
    assert.strictEqual(
      firstTask.task.title,
      'Implement user authentication',
      'Should have correct title',
    );
    assert.strictEqual(
      firstTask.collapsibleState,
      vscode.TreeItemCollapsibleState.None,
      'Should not be collapsible',
    );
  });

  test('TaskItem should have no children', async () => {
    taskProvider.toggleGrouping(); // Disable grouping for flat list

    const children = (await taskProvider.getChildren()) as TaskItem[];
    const firstTask = children[0];

    const taskChildren = await taskProvider.getChildren(firstTask);
    assert.strictEqual(taskChildren.length, 0, 'TaskItem should have no children');
  });

  test('Refresh should fire change event', (done) => {
    taskProvider.onDidChangeTreeData(() => {
      done();
    });

    taskProvider.refresh();
  });

  test('Toggle grouping should fire change event', (done) => {
    taskProvider.onDidChangeTreeData(() => {
      done();
    });

    taskProvider.toggleGrouping();
  });

  test('Should match golden snapshot (grouped)', async () => {
    const result = await snapshotTest('tasks-grouped', taskProvider, {
      updateGoldens: process.env.UPDATE_GOLDENS === 'true',
    });

    if (!result.matches) {
      console.error('Snapshot differences:', result.differences);
    }

    assert.ok(result.matches, `Snapshot mismatch: ${result.differences.join(', ')}`);
  });

  test('Should match golden snapshot (flat)', async () => {
    taskProvider.toggleGrouping(); // Disable grouping

    const result = await snapshotTest('tasks-flat', taskProvider, {
      updateGoldens: process.env.UPDATE_GOLDENS === 'true',
    });

    if (!result.matches) {
      console.error('Snapshot differences:', result.differences);
    }

    assert.ok(result.matches, `Snapshot mismatch: ${result.differences.join(', ')}`);
  });
});
