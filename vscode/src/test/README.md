# VSCode Extension Test Suite

This directory contains comprehensive tests for the JJJ VSCode extension, including UI tests and golden snapshot testing.

## Test Structure

```
test/
├── fixtures/          # Mock data for testing
│   └── mockData.ts    # Sample tasks, features, milestones, bugs, reviews
├── mocks/             # Mock implementations
│   └── mockJJJ.ts     # Mock JJJ client for testing
├── utils/             # Test utilities
│   └── snapshotUtils.ts  # Golden snapshot comparison utilities
├── goldens/           # Golden snapshot files (JSON)
│   ├── tasks-grouped.json
│   ├── tasks-flat.json
│   ├── features.json
│   ├── milestones.json
│   └── bugs.json
└── suite/             # Test suites
    ├── taskProvider.test.ts
    ├── featureProvider.test.ts
    ├── milestoneProvider.test.ts
    ├── bugProvider.test.ts
    └── generateGoldens.ts
```

## Running Tests

### Run all tests

```bash
npm test
```

### Run tests in watch mode

```bash
npm run watch
```

### Generate golden snapshots

```bash
npm run generate-goldens
```

This runs the tests with `UPDATE_GOLDENS=true`, which tells the tests to save their output as golden snapshots instead of comparing against existing ones.

**Note:** Golden generation requires running tests inside VSCode's extension host, so it runs the full test suite.

## Test Types

### Unit Tests

Each provider has comprehensive unit tests that verify:

- Data loading from mock JJJ client
- Tree item creation and properties
- Icon and description formatting
- Status indicators and priorities
- Parent-child relationships
- Event handling (refresh, toggle grouping)
- Empty state handling

### Golden Snapshot Tests

Golden snapshots capture the expected structure of tree views as JSON:

- Complete tree hierarchy
- Labels, descriptions, tooltips
- Icon types and colors
- Collapsible states
- Context values

When tests run, the actual tree structure is compared against the golden snapshot. Any differences are reported as test failures.

## Writing New Tests

### 1. Add Mock Data

Add new test data to `fixtures/mockData.ts`:

```typescript
export const mockNewFeature: Feature = {
  id: 'F-X',
  title: 'Test Feature',
  // ...
};
```

### 2. Create Test Suite

Create a new test file in `suite/`:

```typescript
import * as assert from 'assert';
import { MyProvider } from '../../views/myProvider';
import { MockJJJ } from '../mocks/mockJJJ';
import { snapshotTest } from '../utils/snapshotUtils';

suite('MyProvider Test Suite', () => {
  test('Should load data', async () => {
    const provider = new MyProvider(new MockJJJ());
    const children = await provider.getChildren();
    assert.ok(children.length > 0);
  });

  test('Should match golden snapshot', async () => {
    const provider = new MyProvider(new MockJJJ());
    const result = await snapshotTest('my-provider', provider);
    assert.ok(result.matches, `Mismatch: ${result.differences.join(', ')}`);
  });
});
```

### 3. Generate Golden Snapshot

```bash
UPDATE_GOLDENS=true npm test
```

This creates `goldens/my-provider.json` with the expected tree structure.

## Golden Snapshot Format

Golden snapshots are JSON files with the following structure:

```json
[
  {
    "label": "Item Label",
    "description": "Additional info",
    "tooltip": "Hover text",
    "contextValue": "itemType",
    "collapsibleState": "None",
    "iconPath": {
      "id": "icon-name",
      "color": "charts.blue"
    },
    "children": []
  }
]
```

## Updating Golden Snapshots

When you intentionally change the UI (labels, icons, structure):

1. Review the test failures to confirm they match your changes
2. Regenerate goldens: `npm run test:generate-goldens`
3. Review the diff in `goldens/*.json` files
4. Commit the updated golden files

## Continuous Integration

In CI, tests run without updating goldens:

```bash
npm test
```

Any UI changes that don't match golden snapshots will fail the build, ensuring intentional changes are reviewed.

## Troubleshooting

### Tests fail with "Golden snapshot not found"

Run `npm run test:generate-goldens` to create initial golden files.

### Tests fail with snapshot mismatch

- Check if you intentionally changed the UI
- If yes: regenerate goldens with `UPDATE_GOLDENS=true npm test`
- If no: fix the code to match the expected behavior

### Mock data doesn't match expectations

- Update `fixtures/mockData.ts` with correct data
- Regenerate goldens after updating fixtures

## Best Practices

1. **Keep mock data realistic** - Use data that represents actual usage
2. **Test edge cases** - Empty lists, missing fields, error states
3. **Update goldens deliberately** - Review diffs before committing
4. **Run tests before commits** - Catch regressions early
5. **Add tests for new features** - Maintain high coverage
