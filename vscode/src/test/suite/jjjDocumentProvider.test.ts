import * as assert from 'assert';
import * as vscode from 'vscode';
import { JJJDocumentProvider } from '../../editors/jjjDocumentProvider';
import { MockJJJ } from '../mocks/mockJJJ';

suite('JJJDocumentProvider Test Suite', () => {
  let mockJJJ: MockJJJ;
  let documentProvider: JJJDocumentProvider;

  setup(() => {
    mockJJJ = new MockJJJ();
    documentProvider = new JJJDocumentProvider(mockJJJ);
  });

  suite('URI Parsing', () => {
    test('Should parse task URI correctly', async () => {
      const uri = vscode.Uri.parse('jjj://task/T-1.yaml');
      const content = await documentProvider.loadContent(uri);

      assert.ok(content, 'Should return content');
      assert.ok(content.includes('id: T-1'), 'Should include task ID');
      assert.ok(
        content.includes('title: Implement user authentication'),
        'Should include task title',
      );
    });

    test('Should parse feature URI correctly', async () => {
      const uri = vscode.Uri.parse('jjj://feature/F-1.yaml');
      const content = await documentProvider.loadContent(uri);

      assert.ok(content, 'Should return content');
      assert.ok(content.includes('id: F-1'), 'Should include feature ID');
      assert.ok(
        content.includes('title: User Authentication System'),
        'Should include feature title',
      );
    });

    test('Should parse milestone URI correctly', async () => {
      const uri = vscode.Uri.parse('jjj://milestone/M-1.yaml');
      const content = await documentProvider.loadContent(uri);

      assert.ok(content, 'Should return content');
      assert.ok(content.includes('id: M-1'), 'Should include milestone ID');
      assert.ok(content.includes('title: Version 1.0 Release'), 'Should include milestone title');
    });

    test('Should parse bug URI correctly', async () => {
      const uri = vscode.Uri.parse('jjj://bug/B-1.yaml');
      const content = await documentProvider.loadContent(uri);

      assert.ok(content, 'Should return content');
      assert.ok(content.includes('id: B-1'), 'Should include bug ID');
      assert.ok(content.includes('title: Login button not responding'), 'Should include bug title');
    });

    test('Should handle URI without .yaml extension', async () => {
      const uri = vscode.Uri.parse('jjj://task/T-1');
      const content = await documentProvider.loadContent(uri);

      assert.ok(content, 'Should return content');
      assert.ok(content.includes('id: T-1'), 'Should include task ID');
    });

    test('Should strip .yaml extension from ID', async () => {
      const uri = vscode.Uri.parse('jjj://feature/F-1.yaml');
      const content = await documentProvider.loadContent(uri);

      // Should not include .yaml in the ID
      assert.ok(!content.includes('id: F-1.yaml'), 'Should not include .yaml in ID');
      assert.ok(content.includes('id: F-1'), 'Should have correct ID without extension');
    });
  });

  suite('YAML Formatting', () => {
    test('Task should be formatted as valid YAML', async () => {
      const uri = vscode.Uri.parse('jjj://task/T-1.yaml');
      const content = await documentProvider.loadContent(uri);

      // Check for YAML structure
      assert.ok(content.includes('id:'), 'Should have id field');
      assert.ok(content.includes('title:'), 'Should have title field');
      assert.ok(content.includes('feature_id:'), 'Should have feature_id field');
      assert.ok(content.includes('column:'), 'Should have column field');
      assert.ok(content.includes('tags:'), 'Should have tags field');

      // Check for header comments
      assert.ok(content.includes('# JJJ Task'), 'Should have type header');
      assert.ok(content.includes('# Edit and save to update'), 'Should have instructions');
    });

    test('Feature should include all required fields', async () => {
      const uri = vscode.Uri.parse('jjj://feature/F-1.yaml');
      const content = await documentProvider.loadContent(uri);

      assert.ok(content.includes('status:'), 'Should have status field');
      assert.ok(content.includes('priority:'), 'Should have priority field');
      assert.ok(content.includes('story_points:'), 'Should have story_points field');
      assert.ok(content.includes('task_ids:'), 'Should have task_ids field');
    });

    test('Milestone should include date fields', async () => {
      const uri = vscode.Uri.parse('jjj://milestone/M-1.yaml');
      const content = await documentProvider.loadContent(uri);

      assert.ok(content.includes('target_date:'), 'Should have target_date field');
      assert.ok(content.includes('description:'), 'Should have description field');
      assert.ok(content.includes('version:'), 'Should have version field');
    });

    test('Bug should include severity and status', async () => {
      const uri = vscode.Uri.parse('jjj://bug/B-1.yaml');
      const content = await documentProvider.loadContent(uri);

      assert.ok(content.includes('severity:'), 'Should have severity field');
      assert.ok(content.includes('status:'), 'Should have status field');
      assert.ok(content.includes('repro_steps:'), 'Should have repro_steps field');
    });
  });

  suite('Content Caching', () => {
    test('Should cache document content', () => {
      const uri = vscode.Uri.parse('jjj://task/T-1.yaml');
      const content = 'cached content';

      documentProvider.update(uri, content);

      const retrieved = documentProvider.provideTextDocumentContent(uri);
      assert.strictEqual(retrieved, content, 'Should return cached content');
    });

    test('Should return placeholder if not cached', () => {
      const uri = vscode.Uri.parse('jjj://task/T-2.yaml');

      const content = documentProvider.provideTextDocumentContent(uri);
      assert.ok(content.includes('Loading'), 'Should return loading placeholder');
      assert.ok(content.includes('T-2'), 'Should include task ID in placeholder');
    });

    test('Refresh should clear cache', () => {
      const uri = vscode.Uri.parse('jjj://task/T-1.yaml');
      documentProvider.update(uri, 'cached content');

      documentProvider.refresh();

      const content = documentProvider.provideTextDocumentContent(uri);
      assert.ok(content.includes('Loading'), 'Should return loading placeholder after refresh');
    });
  });

  suite('Error Handling', () => {
    test('Should handle non-existent task', async () => {
      const uri = vscode.Uri.parse('jjj://task/T-999.yaml');

      try {
        await documentProvider.loadContent(uri);
        assert.fail('Should throw error for non-existent task');
      } catch (error) {
        assert.ok(error, 'Should throw error');
      }
    });

    test('Should handle invalid URI format', () => {
      const uri = vscode.Uri.parse('jjj://invalid');

      try {
        documentProvider.provideTextDocumentContent(uri);
        assert.fail('Should throw error for invalid URI');
      } catch (error) {
        assert.ok(error, 'Should throw error for invalid URI');
      }
    });

    test('Should handle unknown item type', async () => {
      const uri = vscode.Uri.parse('jjj://unknown/X-1.yaml');

      try {
        await documentProvider.loadContent(uri);
        assert.fail('Should throw error for unknown type');
      } catch (error) {
        assert.ok(error, 'Should throw error');
      }
    });
  });

  suite('Validation', () => {
    test('Should validate required fields for task', async () => {
      const uri = vscode.Uri.parse('jjj://task/T-1.yaml');
      const doc = {
        uri,
        getText: () => `
id: T-1
title: Test Task
# Missing feature_id and column
tags: []
                `,
      } as vscode.TextDocument;

      const result = await documentProvider.saveDocument(doc);
      assert.strictEqual(result, false, 'Should fail validation with missing fields');
    });

    test('Should validate column values', async () => {
      const uri = vscode.Uri.parse('jjj://task/T-1.yaml');
      const doc = {
        uri,
        getText: () => `
id: T-1
title: Test Task
feature_id: F-1
column: InvalidColumn
tags: []
change_ids: []
comment_count: 0
version: 1
                `,
      } as vscode.TextDocument;

      const result = await documentProvider.saveDocument(doc);
      assert.strictEqual(result, false, 'Should fail validation with invalid column');
    });

    test('Should validate feature status', async () => {
      const uri = vscode.Uri.parse('jjj://feature/F-1.yaml');
      const doc = {
        uri,
        getText: () => `
id: F-1
title: Test Feature
status: InvalidStatus
priority: High
task_ids: []
bug_ids: []
created_at: "2024-01-01"
updated_at: "2024-01-01"
                `,
      } as vscode.TextDocument;

      const result = await documentProvider.saveDocument(doc);
      assert.strictEqual(result, false, 'Should fail validation with invalid status');
    });

    test('Should validate milestone date format', async () => {
      const uri = vscode.Uri.parse('jjj://milestone/M-1.yaml');
      const doc = {
        uri,
        getText: () => `
id: M-1
title: Test Milestone
status: Active
target_date: "01/15/2024"
feature_ids: []
bug_ids: []
tags: []
created_at: "2024-01-01"
updated_at: "2024-01-01"
                `,
      } as vscode.TextDocument;

      const result = await documentProvider.saveDocument(doc);
      assert.strictEqual(result, false, 'Should fail validation with invalid date format');
    });

    test('Should accept valid task document', async () => {
      const uri = vscode.Uri.parse('jjj://task/T-1.yaml');
      const doc = {
        uri,
        getText: () => `
id: T-1
title: Valid Test Task
feature_id: F-1
column: Backlog
assignee: alice
tags:
  - test
change_ids: []
comment_count: 0
version: 1
                `,
      } as vscode.TextDocument;

      const result = await documentProvider.saveDocument(doc);
      assert.strictEqual(result, true, 'Should pass validation with valid data');
    });
  });

  suite('Integration', () => {
    test('Load and validate task workflow', async () => {
      // Load content
      const uri = vscode.Uri.parse('jjj://task/T-1.yaml');
      const content = await documentProvider.loadContent(uri);

      // Cache it
      documentProvider.update(uri, content);

      // Retrieve from cache
      const cached = documentProvider.provideTextDocumentContent(uri);
      assert.strictEqual(cached, content, 'Cached content should match loaded content');

      // Parse as document and validate
      const doc = {
        uri,
        getText: () => cached,
      } as vscode.TextDocument;

      const result = await documentProvider.saveDocument(doc);
      assert.strictEqual(result, true, 'Should be able to save loaded content');
    });

    test('Load all item types', async () => {
      const types = [
        { type: 'task', id: 'T-1' },
        { type: 'feature', id: 'F-1' },
        { type: 'milestone', id: 'M-1' },
        { type: 'bug', id: 'B-1' },
      ];

      for (const { type, id } of types) {
        const uri = vscode.Uri.parse(`jjj://${type}/${id}.yaml`);
        const content = await documentProvider.loadContent(uri);

        assert.ok(content, `Should load ${type}`);
        assert.ok(content.includes(`id: ${id}`), `Should have correct ${type} ID`);
        assert.ok(content.includes('# JJJ'), `Should have header for ${type}`);
      }
    });
  });
});
