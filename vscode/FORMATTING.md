# Code Formatting with Prettier

The JJJ VSCode extension uses [Prettier](https://prettier.io/) for automatic code formatting to ensure consistent code style across the project.

## Configuration

### Prettier Settings ([.prettierrc](vscode/.prettierrc))

```json
{
  "semi": true,
  "trailingComma": "es5",
  "singleQuote": true,
  "printWidth": 100,
  "tabWidth": 4,
  "useTabs": false,
  "arrowParens": "avoid",
  "endOfLine": "lf"
}
```

**Style Choices:**
- **Semicolons**: Always use semicolons
- **Quotes**: Single quotes for strings
- **Line Width**: 100 characters maximum
- **Indentation**: 4 spaces (no tabs)
- **Trailing Commas**: ES5 compatible (objects, arrays)
- **Arrow Functions**: Omit parens when possible `x => x`
- **Line Endings**: LF (Unix-style)

### Ignored Files ([.prettierignore](vscode/.prettierignore))

```
out/              # Build output
node_modules/     # Dependencies
*.vsix            # Extension packages
src/test/goldens/ # Test snapshots
*.js.map          # Source maps
*.log             # Log files
```

## Usage

### Format All Files

```bash
npm run format
```

Formats all TypeScript, JavaScript, JSON, and Markdown files in the `src/` directory.

### Check Formatting

```bash
npm run format:check
```

Checks if all files are formatted correctly without making changes. Useful for CI/CD pipelines.

### Format on Save (Automatic)

The workspace is configured to format files automatically when you save them in VSCode.

**Requirements:**
1. Install the [Prettier VSCode extension](https://marketplace.visualstudio.com/items?itemName=esbenp.prettier-vscode)
2. Open this directory in VSCode

The extension will be recommended automatically when you open the workspace.

## VSCode Integration

### Workspace Settings ([.vscode/settings.json](vscode/.vscode/settings.json))

```json
{
    "editor.formatOnSave": true,
    "editor.defaultFormatter": "esbenp.prettier-vscode",
    "[typescript]": {
        "editor.formatOnSave": true
    }
}
```

These settings enable:
- **Format on save** for all file types
- **Prettier as default formatter** for TypeScript, JavaScript, JSON, and Markdown
- **Automatic formatting** when you press Ctrl+S / Cmd+S

### Recommended Extensions ([.vscode/extensions.json](vscode/.vscode/extensions.json))

When you open this workspace, VSCode will suggest installing:
- **Prettier** - Code formatter
- **ESLint** - JavaScript/TypeScript linter

## Integration with Development Workflow

### Before Committing

Format your code before committing:

```bash
npm run format
```

### In CI/CD

Check formatting in your CI pipeline:

```bash
npm run format:check
```

This will fail the build if any files are not formatted correctly.

### Pre-commit Hook (Optional)

You can add a pre-commit hook to automatically format staged files:

```bash
# Install husky and lint-staged
npm install --save-dev husky lint-staged

# Add to package.json
"lint-staged": {
  "src/**/*.{ts,js,json,md}": "prettier --write"
}
```

## Manual Formatting

### Format a Specific File

```bash
npx prettier --write src/extension.ts
```

### Format a Directory

```bash
npx prettier --write src/views/
```

### Check a Specific File

```bash
npx prettier --check src/extension.ts
```

## Prettier vs ESLint

This project uses both:

- **Prettier** - Code formatting (indentation, line breaks, quotes)
- **ESLint** - Code quality (unused variables, best practices)

They work together:
1. Prettier formats the code style
2. ESLint checks for code quality issues

Run both before committing:

```bash
npm run format   # Format with Prettier
npm run lint     # Check with ESLint
```

Or run the pretest script which includes both:

```bash
npm run pretest
```

## Common Issues

### "All matched files use Prettier code style!"

✅ Perfect! All files are properly formatted.

### Warning: Files need formatting

Run `npm run format` to fix automatically.

### Prettier and ESLint Conflicts

The Prettier configuration is designed to work with ESLint. If you encounter conflicts:

1. Check that you have the latest versions
2. ESLint rules should not override Prettier formatting
3. Use `eslint-config-prettier` if needed (already compatible)

### Format on Save Not Working

1. Ensure Prettier extension is installed:
   ```bash
   code --install-extension esbenp.prettier-vscode
   ```

2. Check VSCode settings:
   - Open Command Palette (Cmd+Shift+P / Ctrl+Shift+P)
   - Type "Preferences: Open Workspace Settings (JSON)"
   - Verify `editor.formatOnSave` is true

3. Restart VSCode

## Best Practices

### 1. Format Before Committing

Always format your code before creating a commit:

```bash
npm run format && git add . && git commit -m "Your message"
```

### 2. Check Formatting in Pull Requests

Add a CI check to ensure all PRs have properly formatted code:

```yaml
# .github/workflows/ci.yml
- name: Check formatting
  run: npm run format:check
```

### 3. Use Editor Integration

Let Prettier format automatically as you code:
- Install the VSCode extension
- Enable format on save
- Focus on writing code, not formatting

### 4. Don't Fight the Formatter

Prettier is opinionated by design:
- Accept its formatting decisions
- Don't manually adjust what Prettier formats
- Consistency > personal preference

## Examples

### Before Prettier

```typescript
function   hello(  name:string ):   string{
return "Hello, "+name+'!'}
```

### After Prettier

```typescript
function hello(name: string): string {
    return 'Hello, ' + name + '!';
}
```

Prettier automatically:
- Fixed spacing
- Used consistent quotes
- Added proper indentation
- Made the code readable

## Additional Resources

- [Prettier Documentation](https://prettier.io/docs/en/)
- [Prettier Playground](https://prettier.io/playground/)
- [VSCode Prettier Extension](https://marketplace.visualstudio.com/items?itemName=esbenp.prettier-vscode)
- [Prettier Options](https://prettier.io/docs/en/options.html)
