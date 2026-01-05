import * as assert from 'assert';
import * as vscode from 'vscode';
import { FeatureProvider, FeatureItem } from '../../views/featureProvider';
import { MockJJJ } from '../mocks/mockJJJ';
import { snapshotTest } from '../utils/snapshotUtils';

suite('FeatureProvider Test Suite', () => {
  let mockJJJ: MockJJJ;
  let featureProvider: FeatureProvider;

  setup(() => {
    mockJJJ = new MockJJJ();
    featureProvider = new FeatureProvider(mockJJJ);
  });

  test('Should load features from JJJ', async () => {
    const children = await featureProvider.getChildren();
    assert.ok(children, 'Should return children');
    assert.strictEqual(children.length, 4, 'Should have 4 features');
  });

  test('All children should be FeatureItems', async () => {
    const children = await featureProvider.getChildren();
    assert.ok(
      children.every((child) => child instanceof FeatureItem),
      'All children should be FeatureItems',
    );
  });

  test('FeatureItem should have correct properties', async () => {
    const children = (await featureProvider.getChildren()) as FeatureItem[];
    const authFeature = children.find((f) => f.feature.id === 'F-1');

    assert.ok(authFeature, 'Should find auth feature');
    assert.strictEqual(
      authFeature.feature.title,
      'User Authentication System',
      'Should have correct title',
    );
    assert.strictEqual(authFeature.feature.status, 'InProgress', 'Should have correct status');
    assert.strictEqual(authFeature.feature.priority, 'High', 'Should have correct priority');
    assert.strictEqual(
      authFeature.collapsibleState,
      vscode.TreeItemCollapsibleState.None,
      'Should not be collapsible',
    );
  });

  test('FeatureItem should display priority indicator', async () => {
    const children = (await featureProvider.getChildren()) as FeatureItem[];

    // Check critical priority
    const criticalFeature = children.find((f) => f.feature.priority === 'Critical');
    assert.ok(criticalFeature, 'Should have critical priority feature');
    assert.ok(
      (typeof criticalFeature.description === 'string' ? criticalFeature.description : '').includes(
        '🔴',
      ),
      'Critical feature should have red indicator',
    );

    // Check high priority
    const highFeature = children.find((f) => f.feature.priority === 'High');
    assert.ok(highFeature, 'Should have high priority feature');
    assert.ok(
      (typeof highFeature.description === 'string' ? highFeature.description : '').includes('🟠'),
      'High feature should have orange indicator',
    );
  });

  test('FeatureItem should display milestone', async () => {
    const children = (await featureProvider.getChildren()) as FeatureItem[];
    const featureWithMilestone = children.find((f) => f.feature.milestone_id);

    assert.ok(featureWithMilestone, 'Should have feature with milestone');
    assert.ok(
      (typeof featureWithMilestone.description === 'string'
        ? featureWithMilestone.description
        : ''
      ).includes('M:'),
      'Should display milestone ID',
    );
  });

  test('FeatureItem should display task count', async () => {
    const children = (await featureProvider.getChildren()) as FeatureItem[];
    const featureWithTasks = children.find((f) => f.feature.task_ids.length > 0);

    assert.ok(featureWithTasks, 'Should have feature with tasks');
    assert.ok(
      (typeof featureWithTasks.description === 'string'
        ? featureWithTasks.description
        : ''
      ).includes('tasks'),
      'Should display task count',
    );
  });

  test('FeatureItem should have correct icon based on status', async () => {
    const children = (await featureProvider.getChildren()) as FeatureItem[];

    const doneFeature = children.find((f) => f.feature.status === 'Done');
    assert.ok(doneFeature, 'Should have done feature');
    assert.ok(doneFeature.iconPath, 'Done feature should have icon');

    const inProgressFeature = children.find((f) => f.feature.status === 'InProgress');
    assert.ok(inProgressFeature, 'Should have in progress feature');
    assert.ok(inProgressFeature.iconPath, 'In progress feature should have icon');
  });

  test('FeatureItem should have no children', async () => {
    const children = (await featureProvider.getChildren()) as FeatureItem[];
    const firstFeature = children[0];

    const featureChildren = await featureProvider.getChildren(firstFeature);
    assert.strictEqual(featureChildren.length, 0, 'FeatureItem should have no children');
  });

  test('Refresh should fire change event', (done) => {
    featureProvider.onDidChangeTreeData(() => {
      done();
    });

    featureProvider.refresh();
  });

  test('Should match golden snapshot', async () => {
    const result = await snapshotTest('features', featureProvider, {
      updateGoldens: process.env.UPDATE_GOLDENS === 'true',
    });

    if (!result.matches) {
      console.error('Snapshot differences:', result.differences);
    }

    assert.ok(result.matches, `Snapshot mismatch: ${result.differences.join(', ')}`);
  });

  test('Should handle empty feature list', async () => {
    // Create a mock that returns empty list
    const emptyMock = new MockJJJ();
    emptyMock.listFeatures = async () => [];
    const emptyProvider = new FeatureProvider(emptyMock);

    const children = await emptyProvider.getChildren();
    assert.strictEqual(children.length, 0, 'Should return empty array');
  });
});
