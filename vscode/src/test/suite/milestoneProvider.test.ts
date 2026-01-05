import * as assert from 'assert';
import * as vscode from 'vscode';
import { MilestoneProvider, MilestoneItem } from '../../views/milestoneProvider';
import { MockJJJ } from '../mocks/mockJJJ';
import { snapshotTest } from '../utils/snapshotUtils';

suite('MilestoneProvider Test Suite', () => {
  let mockJJJ: MockJJJ;
  let milestoneProvider: MilestoneProvider;

  setup(() => {
    mockJJJ = new MockJJJ();
    milestoneProvider = new MilestoneProvider(mockJJJ);
  });

  test('Should load milestones from JJJ', async () => {
    const children = await milestoneProvider.getChildren();
    assert.ok(children, 'Should return children');
    assert.strictEqual(children.length, 3, 'Should have 3 milestones');
  });

  test('All children should be MilestoneItems', async () => {
    const children = await milestoneProvider.getChildren();
    assert.ok(
      children.every((child) => child instanceof MilestoneItem),
      'All children should be MilestoneItems',
    );
  });

  test('MilestoneItem should have correct properties', async () => {
    const children = (await milestoneProvider.getChildren()) as MilestoneItem[];
    const v1Milestone = children.find((m) => m.milestone.id === 'M-1');

    assert.ok(v1Milestone, 'Should find v1 milestone');
    assert.strictEqual(
      v1Milestone.milestone.title,
      'Version 1.0 Release',
      'Should have correct title',
    );
    assert.strictEqual(v1Milestone.milestone.status, 'Active', 'Should have correct status');
    assert.strictEqual(v1Milestone.milestone.version, '1.0.0', 'Should have correct version');
    assert.strictEqual(
      v1Milestone.collapsibleState,
      vscode.TreeItemCollapsibleState.None,
      'Should not be collapsible',
    );
  });

  test('MilestoneItem should display target date', async () => {
    const children = (await milestoneProvider.getChildren()) as MilestoneItem[];
    const milestoneWithDate = children.find((m) => m.milestone.target_date);

    assert.ok(milestoneWithDate, 'Should have milestone with target date');
    const desc =
      typeof milestoneWithDate.description === 'string' ? milestoneWithDate.description : '';
    assert.ok(
      desc.includes('📅') || desc.includes('⏰') || desc.includes('⚠️'),
      'Should display date indicator',
    );
  });

  test('MilestoneItem should display feature count', async () => {
    const children = (await milestoneProvider.getChildren()) as MilestoneItem[];
    const milestoneWithFeatures = children.find((m) => m.milestone.feature_ids.length > 0);

    assert.ok(milestoneWithFeatures, 'Should have milestone with features');
    assert.ok(
      (typeof milestoneWithFeatures.description === 'string'
        ? milestoneWithFeatures.description
        : ''
      ).includes('F'),
      'Should display feature count',
    );
  });

  test('MilestoneItem tooltip should include description', async () => {
    const children = (await milestoneProvider.getChildren()) as MilestoneItem[];
    const v1Milestone = children.find((m) => m.milestone.id === 'M-1');

    assert.ok(v1Milestone, 'Should find v1 milestone');
    assert.ok(v1Milestone.tooltip, 'Should have tooltip');
    assert.ok(
      typeof v1Milestone.tooltip === 'string' &&
        v1Milestone.tooltip.includes('First major release'),
      'Tooltip should include description',
    );
  });

  test('MilestoneItem should have correct icon based on status', async () => {
    const children = (await milestoneProvider.getChildren()) as MilestoneItem[];

    const activeMilestone = children.find((m) => m.milestone.status === 'active');
    assert.ok(activeMilestone, 'Should have active milestone');
    assert.ok(activeMilestone.iconPath, 'Active milestone should have icon');

    const releasedMilestone = children.find((m) => m.milestone.status === 'released');
    assert.ok(releasedMilestone, 'Should have released milestone');
    assert.ok(releasedMilestone.iconPath, 'Released milestone should have icon');

    const planningMilestone = children.find((m) => m.milestone.status === 'planning');
    assert.ok(planningMilestone, 'Should have planning milestone');
    assert.ok(planningMilestone.iconPath, 'Planning milestone should have icon');
  });

  test('MilestoneItem should have no children', async () => {
    const children = (await milestoneProvider.getChildren()) as MilestoneItem[];
    const firstMilestone = children[0];

    const milestoneChildren = await milestoneProvider.getChildren(firstMilestone);
    assert.strictEqual(milestoneChildren.length, 0, 'MilestoneItem should have no children');
  });

  test('Refresh should fire change event', (done) => {
    milestoneProvider.onDidChangeTreeData(() => {
      done();
    });

    milestoneProvider.refresh();
  });

  test('Should match golden snapshot', async () => {
    const result = await snapshotTest('milestones', milestoneProvider, {
      updateGoldens: process.env.UPDATE_GOLDENS === 'true',
    });

    if (!result.matches) {
      console.error('Snapshot differences:', result.differences);
    }

    assert.ok(result.matches, `Snapshot mismatch: ${result.differences.join(', ')}`);
  });

  test('Should handle empty milestone list', async () => {
    const emptyMock = new MockJJJ();
    emptyMock.listMilestones = async () => [];
    const emptyProvider = new MilestoneProvider(emptyMock);

    const children = await emptyProvider.getChildren();
    assert.strictEqual(children.length, 0, 'Should return empty array');
  });
});
