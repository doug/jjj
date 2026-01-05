# JJJ Document Provider

Custom document provider for editing JJJ items (tasks, features, milestones, bugs) as YAML files.

## Overview

The document provider allows users to:

- Click items in the sidebar to open them as editable YAML files
- Edit the YAML content directly in the editor
- Save changes with automatic validation
- See changes reflected in the sidebar views

## Architecture

### URI Format

Items are represented as virtual URIs:

```
jjj://task/T-1.yaml
jjj://feature/F-1.yaml
jjj://milestone/M-1.yaml
jjj://bug/B-1.yaml
```

### URI Parsing

The `parseUri()` method extracts the type and ID from the URI:

```typescript
private parseUri(uri: vscode.Uri): { type: string; id: string } {
    // Path is like "/task/T-1.yaml"
    const parts = uri.path.split('/').filter(Boolean);

    if (parts.length < 2) {
        throw new Error(`Invalid URI format: ${uri.toString()}`);
    }

    const type = parts[0];      // 'task'
    let id = parts[1];          // 'T-1.yaml'

    // Remove .yaml extension if present
    if (id.endsWith('.yaml')) {
        id = id.slice(0, -5);   // 'T-1'
    }

    return { type, id };
}
```

**Important**: The `.yaml` extension is added for better VSCode integration (syntax highlighting, file icons), but must be stripped when parsing the actual item ID.

## Implementation Details

### TextDocumentContentProvider

Implements VSCode's `TextDocumentContentProvider` interface:

```typescript
provideTextDocumentContent(uri: vscode.Uri): string
```

- Returns cached content if available
- Otherwise returns a loading placeholder
- Actual content is loaded asynchronously via `loadContent()`

### Content Loading

```typescript
async loadContent(uri: vscode.Uri): Promise<string>
```

1. Parse URI to get type and ID
2. Fetch item from JJJ CLI
3. Format as YAML with comments
4. Return formatted content

### Content Formatting

Each item type has a format method:

- `formatTask(task: Task): string`
- `formatFeature(feature: Feature): string`
- `formatMilestone(milestone: Milestone): string`
- `formatBug(bug: Bug): string`

Format includes:

- Header comments with instructions
- YAML formatted data
- All editable fields
- Read-only fields marked in comments

### Validation

```typescript
async saveDocument(document: vscode.TextDocument): Promise<boolean>
```

1. Parse URI to get type and ID
2. Parse YAML content from document
3. Validate required fields
4. Validate field values (enums, formats)
5. Show error message if validation fails
6. Return success/failure status

### Caching

Content is cached in memory:

```typescript
private documents = new Map<string, string>();
```

- `update(uri, content)` - Add/update cache
- `refresh()` - Clear all cache
- Cache is used by `provideTextDocumentContent()`

## Testing

See [jjjDocumentProvider.test.ts](../test/suite/jjjDocumentProvider.test.ts) for comprehensive tests:

### URI Parsing Tests

- Validate correct parsing of all item types
- Ensure .yaml extension is stripped
- Handle URIs with and without extension

### YAML Formatting Tests

- Verify all required fields are present
- Check YAML structure is valid
- Validate header comments

### Validation Tests

- Required field validation
- Enum value validation
- Date format validation
- Invalid data rejection

### Integration Tests

- Load → Cache → Save workflow
- All item types can be loaded and saved
- Error handling for missing items

## Common Issues and Fixes

### Issue: "Unknown type: F-1.yaml"

**Cause**: The ID includes the `.yaml` extension when it shouldn't.

**Fix**: The `parseUri()` method now strips the extension:

```typescript
if (id.endsWith('.yaml')) {
  id = id.slice(0, -5);
}
```

### Issue: Validation fails on save

**Cause**: Missing required fields or invalid values

**Fix**: Check error message for specific validation failure. Common issues:

- Missing required fields (id, title, etc.)
- Invalid enum values (status, priority, severity)
- Incorrect date format (must be YYYY-MM-DD)

### Issue: Content not refreshing after save

**Cause**: Cache not invalidated

**Fix**: The save handler automatically refreshes providers after successful save. If issue persists, manually call `documentProvider.refresh()`.

## Extension Integration

### Registration

```typescript
// Create provider
documentProvider = new JJJDocumentProvider(jjj);

// Register with VSCode
context.subscriptions.push(
  vscode.workspace.registerTextDocumentContentProvider('jjj', documentProvider),
);
```

### Save Handler

```typescript
vscode.workspace.onWillSaveTextDocument(async (event) => {
  if (event.document.uri.scheme === 'jjj' && documentProvider) {
    event.waitUntil(
      documentProvider.saveDocument(event.document).then((success) => {
        if (success) {
          // Refresh views
          taskProvider?.refresh();
          featureProvider?.refresh();
          milestoneProvider?.refresh();
          bugProvider?.refresh();
        }
      }),
    );
  }
});
```

### Open Commands

```typescript
async function openJJJDocument(type: string, id: string, title: string) {
  const uri = vscode.Uri.parse(`jjj://${type}/${id}.yaml`);

  // Load content
  const content = await documentProvider.loadContent(uri);
  documentProvider.update(uri, content);

  // Open document
  const doc = await vscode.workspace.openTextDocument(uri);
  await vscode.window.showTextDocument(doc);

  // Set YAML language mode
  await vscode.languages.setTextDocumentLanguage(doc, 'yaml');
}
```

## Future Enhancements

Potential improvements:

- [ ] Diff view showing changes before save
- [ ] Undo/redo support
- [ ] Auto-complete for enum values
- [ ] Schema validation with JSON Schema
- [ ] Conflict detection for concurrent edits
- [ ] Rich formatting with syntax highlighting
- [ ] Field descriptions in hover tooltips
