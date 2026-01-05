import * as assert from 'assert';
import * as vscode from 'vscode';
import { BugProvider, BugItem } from '../../views/bugProvider';
import { MockJJJ } from '../mocks/mockJJJ';
import { snapshotTest } from '../utils/snapshotUtils';

suite('BugProvider Test Suite', () => {
  let mockJJJ: MockJJJ;
  let bugProvider: BugProvider;

  setup(() => {
    mockJJJ = new MockJJJ();
    bugProvider = new BugProvider(mockJJJ);
  });

  test('Should load bugs from JJJ', async () => {
    const children = await bugProvider.getChildren();
    assert.ok(children, 'Should return children');
    // Provider filters for open bugs only
    assert.ok(children.length >= 3, 'Should have at least 3 open bugs');
  });

  test('All children should be BugItems', async () => {
    const children = await bugProvider.getChildren();
    assert.ok(
      children.every((child) => child instanceof BugItem),
      'All children should be BugItems',
    );
  });

  test('BugItem should have correct properties', async () => {
    const children = (await bugProvider.getChildren()) as BugItem[];
    const loginBug = children.find((b) => b.bug.id === 'B-1');

    assert.ok(loginBug, 'Should find login bug');
    assert.strictEqual(
      loginBug.bug.title,
      'Login button not responding',
      'Should have correct title',
    );
    assert.strictEqual(loginBug.bug.severity, 'Critical', 'Should have correct severity');
    assert.strictEqual(loginBug.bug.status, 'InProgress', 'Should have correct status');
    assert.strictEqual(
      loginBug.collapsibleState,
      vscode.TreeItemCollapsibleState.None,
      'Should not be collapsible',
    );
  });

  test('BugItem should display severity indicator', async () => {
    const children = (await bugProvider.getChildren()) as BugItem[];

    // Check critical severity
    const criticalBug = children.find((b) => b.bug.severity === 'critical');
    assert.ok(criticalBug, 'Should have critical severity bug');
    const criticalDesc = typeof criticalBug.description === 'string' ? criticalBug.description : '';
    assert.ok(criticalDesc.includes('🔴'), 'Critical bug should have red indicator');
    assert.ok(criticalDesc.includes('Critical'), 'Critical bug should display severity text');

    // Check high severity
    const highBug = children.find((b) => b.bug.severity === 'high');
    assert.ok(highBug, 'Should have high severity bug');
    const highDesc = typeof highBug.description === 'string' ? highBug.description : '';
    assert.ok(highDesc.includes('🟠'), 'High bug should have orange indicator');

    // Check medium severity
    const mediumBug = children.find((b) => b.bug.severity === 'medium');
    assert.ok(mediumBug, 'Should have medium severity bug');
    const mediumDesc = typeof mediumBug.description === 'string' ? mediumBug.description : '';
    assert.ok(mediumDesc.includes('🟡'), 'Medium bug should have yellow indicator');
  });

  test('BugItem should display assignee', async () => {
    const children = (await bugProvider.getChildren()) as BugItem[];
    const bugWithAssignee = children.find((b) => b.bug.assignee);

    assert.ok(bugWithAssignee, 'Should have bug with assignee');
    const desc = typeof bugWithAssignee.description === 'string' ? bugWithAssignee.description : '';
    assert.ok(desc.includes('@'), 'Should display assignee with @ symbol');
  });

  test('BugItem should display feature ID', async () => {
    const children = (await bugProvider.getChildren()) as BugItem[];
    const bugWithFeature = children.find((b) => b.bug.feature_id);

    assert.ok(bugWithFeature, 'Should have bug with feature');
    const desc = typeof bugWithFeature.description === 'string' ? bugWithFeature.description : '';
    assert.ok(desc.includes('F:'), 'Should display feature ID');
  });

  test('BugItem tooltip should include repro steps', async () => {
    const children = (await bugProvider.getChildren()) as BugItem[];
    const bugWithRepro = children.find((b) => b.bug.repro_steps);

    assert.ok(bugWithRepro, 'Should have bug with repro steps');
    assert.ok(bugWithRepro.tooltip, 'Should have tooltip');
    assert.ok(
      typeof bugWithRepro.tooltip === 'string' && bugWithRepro.tooltip.includes('Repro Steps'),
      'Tooltip should include repro steps section',
    );
  });

  test('BugItem should have correct icon based on status', async () => {
    const children = (await bugProvider.getChildren()) as BugItem[];

    const inProgressBug = children.find((b) => b.bug.status === 'inprogress');
    assert.ok(inProgressBug, 'Should have in progress bug');
    assert.ok(inProgressBug.iconPath, 'In progress bug should have icon');

    const confirmedBug = children.find((b) => b.bug.status === 'confirmed');
    assert.ok(confirmedBug, 'Should have confirmed bug');
    assert.ok(confirmedBug.iconPath, 'Confirmed bug should have icon');

    const newBug = children.find((b) => b.bug.status === 'new');
    assert.ok(newBug, 'Should have new bug');
    assert.ok(newBug.iconPath, 'New bug should have icon');
  });

  test('BugItem should have no children', async () => {
    const children = (await bugProvider.getChildren()) as BugItem[];
    const firstBug = children[0];

    const bugChildren = await bugProvider.getChildren(firstBug);
    assert.strictEqual(bugChildren.length, 0, 'BugItem should have no children');
  });

  test('Refresh should fire change event', (done) => {
    bugProvider.onDidChangeTreeData(() => {
      done();
    });

    bugProvider.refresh();
  });

  test('Should match golden snapshot', async () => {
    const result = await snapshotTest('bugs', bugProvider, {
      updateGoldens: process.env.UPDATE_GOLDENS === 'true',
    });

    if (!result.matches) {
      console.error('Snapshot differences:', result.differences);
    }

    assert.ok(result.matches, `Snapshot mismatch: ${result.differences.join(', ')}`);
  });

  test('Should handle empty bug list', async () => {
    const emptyMock = new MockJJJ();
    emptyMock.listBugs = async () => [];
    const emptyProvider = new BugProvider(emptyMock);

    const children = await emptyProvider.getChildren();
    assert.strictEqual(children.length, 0, 'Should return empty array');
  });

  test('Should only show open bugs', async () => {
    const children = (await bugProvider.getChildren()) as BugItem[];

    // All bugs should be in open states (New, Confirmed, InProgress)
    const openStates = ['New', 'Confirmed', 'InProgress'];
    const allOpen = children.every((bug) => openStates.includes(bug.bug.status));

    assert.ok(allOpen, 'All bugs should be in open state');
  });
});
