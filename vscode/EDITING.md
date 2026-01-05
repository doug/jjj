# Editing JJJ Items in VSCode

The JJJ VSCode extension allows you to edit tasks, features, milestones, and bugs directly in the editor as YAML files with validation.

## How to Edit Items

### Opening an Item

Click on any item in the sidebar views:
- **Tasks** - Click a task to edit its properties
- **Features** - Click a feature to edit its details
- **Milestones** - Click a milestone to edit its information
- **Bugs** - Click a bug to edit its properties

The item will open in the main editor as a YAML file.

### Editing

The file opens with:
- **Header comments** - Instructions and field information
- **YAML content** - All editable fields for the item

Example Task:
```yaml
# JJJ Task
# Edit and save to update
# Fields marked with * are required

id: T-1
title: Implement user authentication
feature_id: F-1
column: In Progress
assignee: alice
tags:
  - backend
  - auth
change_ids:
  - abc123
comment_count: 2
version: 1
```

### Saving Changes

1. **Edit the YAML** - Modify any field values
2. **Save the file** - Press `Cmd+S` (Mac) or `Ctrl+S` (Windows/Linux)
3. **Validation runs automatically**:
   - Required fields are checked
   - Field values are validated (status, priority, severity, etc.)
   - Date formats are checked
   - If validation fails, an error message is shown
4. **On success**:
   - Changes are saved to jjj
   - Sidebar views refresh automatically
   - Status bar updates

## Validation Rules

### Tasks

**Required fields:**
- `id` - Task identifier (cannot be changed)
- `title` - Task title
- `feature_id` - Parent feature ID
- `column` - Current column

**Valid columns:**
- `Backlog`
- `In Progress`
- `Review`
- `Done`

**Other fields:**
- `assignee` - Username (optional)
- `tags` - List of tags
- `change_ids` - Associated change IDs (read-only)
- `comment_count` - Number of comments (read-only)
- `version` - Version number (read-only)

### Features

**Required fields:**
- `id` - Feature identifier (cannot be changed)
- `title` - Feature title
- `status` - Current status
- `priority` - Priority level

**Valid statuses:**
- `Backlog`
- `InProgress`
- `Review`
- `Done`
- `Blocked`

**Valid priorities:**
- `Low`
- `Medium`
- `High`
- `Critical`

**Other fields:**
- `milestone_id` - Parent milestone (optional)
- `assignee` - Username (optional)
- `story_points` - Estimated points (optional)
- `task_ids` - Child task IDs (read-only)
- `bug_ids` - Associated bug IDs (read-only)
- `created_at` - Creation timestamp (read-only)
- `updated_at` - Update timestamp (read-only)

### Milestones

**Required fields:**
- `id` - Milestone identifier (cannot be changed)
- `title` - Milestone title
- `status` - Current status

**Valid statuses:**
- `Planning`
- `Active`
- `Released`
- `Cancelled`

**Other fields:**
- `description` - Milestone description (optional)
- `target_date` - Target release date in `YYYY-MM-DD` format (optional)
- `version` - Version string (optional)
- `feature_ids` - Child feature IDs (read-only)
- `bug_ids` - Associated bug IDs (read-only)
- `tags` - List of tags
- `created_at` - Creation timestamp (read-only)
- `updated_at` - Update timestamp (read-only)

### Bugs

**Required fields:**
- `id` - Bug identifier (cannot be changed)
- `title` - Bug title
- `severity` - Severity level
- `status` - Current status

**Valid severities:**
- `Low`
- `Medium`
- `High`
- `Critical`

**Valid statuses:**
- `New`
- `Confirmed`
- `InProgress`
- `Fixed`
- `Closed`
- `WontFix`
- `Duplicate`

**Other fields:**
- `feature_id` - Parent feature (optional)
- `milestone_id` - Target milestone (optional)
- `assignee` - Assigned to (optional)
- `reporter` - Reported by (optional)
- `repro_steps` - Reproduction steps (optional)
- `created_at` - Creation timestamp (read-only)
- `updated_at` - Update timestamp (read-only)

## Tips

### YAML Syntax

- Use 2 spaces for indentation (not tabs)
- Lists use `-` prefix: `tags: [backend, auth]` or multiline:
  ```yaml
  tags:
    - backend
    - auth
  ```
- Strings with special characters need quotes: `title: "Fix: bug in auth"`
- `null` for empty optional fields

### Validation Errors

Common validation errors and fixes:

**"Missing required fields"**
- Ensure all required fields are present
- Check spelling of field names

**"Invalid status"**
- Use exact case: `InProgress` not `in-progress`
- Check valid values list above

**"Invalid date format"**
- Use `YYYY-MM-DD` format: `2024-03-15`
- Not `03/15/2024` or `15-03-2024`

**"YAML parse error"**
- Check indentation (use 2 spaces)
- Ensure proper YAML syntax
- Look for unclosed quotes

### Read-Only Fields

Some fields are read-only and cannot be modified:
- `id` - Item identifier
- `*_ids` fields - Lists of associated items (managed by jjj)
- `comment_count` - Calculated field
- `version` - Internal version number
- `created_at` / `updated_at` - Timestamps (auto-updated)

Changes to these fields will be ignored or may cause validation errors.

### Keyboard Shortcuts

- `Cmd+S` / `Ctrl+S` - Save and validate
- `Cmd+W` / `Ctrl+W` - Close editor
- `Cmd+Shift+P` / `Ctrl+Shift+P` - Command palette (search for "JJJ" commands)

## Troubleshooting

### Changes not saving

1. Check for validation errors in the error message
2. Ensure YAML is valid (proper indentation, no syntax errors)
3. Verify required fields are present
4. Check that jjj CLI is accessible

### Sidebar not refreshing

After saving, the sidebar should refresh automatically. If it doesn't:
1. Click the refresh button in the sidebar view
2. Or use command palette: `JJJ: Refresh All Views`

### File shows as modified but not saved

- The file uses a virtual URI (`jjj://type/id.yaml`)
- Save normally with `Cmd+S` / `Ctrl+S`
- VSCode may show unsaved indicator until validation completes

## Example Workflow

1. **View your tasks** - Open JJJ sidebar
2. **Click a task** - Opens in editor as YAML
3. **Update status** - Change `column: Backlog` to `column: In Progress`
4. **Assign yourself** - Add `assignee: yourname`
5. **Save** - Press `Cmd+S`
6. **Verify** - Check sidebar updates to show new status

## Advanced Usage

### Bulk Editing

To edit multiple items:
1. Open first item from sidebar
2. Make changes and save
3. Open next item (previous stays open in tab)
4. Repeat as needed
5. All tabs update independently

### Templates

Create YAML snippets in VSCode for common patterns:

```json
{
  "jjj-new-task": {
    "prefix": "jjj-task",
    "body": [
      "id: ${1:T-ID}",
      "title: ${2:Task Title}",
      "feature_id: ${3:F-1}",
      "column: ${4|Backlog,In Progress,Review,Done|}",
      "assignee: ${5:username}",
      "tags:",
      "  - ${6:tag}"
    ]
  }
}
```

Add to `.vscode/yaml.code-snippets` in your workspace.
