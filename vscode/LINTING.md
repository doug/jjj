# ESLint and Prettier Integration

This project uses both ESLint and Prettier working together:
- **Prettier**: Code formatting (style)
- **ESLint**: Code quality (logic, best practices)

## Configuration

### Prettier ([.prettierrc](vscode/.prettierrc))

Handles all code **formatting**:
- 2 space indentation
- Single quotes
- Always semicolons
- Trailing commas everywhere
- 100 character line width
- Always include arrow function parentheses

### ESLint ([.eslintrc.json](vscode/.eslintrc.json))

Handles **code quality** only:
- ✅ All formatting rules **disabled** (Prettier handles these)
- ✅ Unused variables detection
- ✅ TypeScript best practices
- ✅ Code logic issues

## Key Changes Made

### Disabled ESLint Formatting Rules

All formatting-related rules are turned **off** to avoid conflicts:

```json
{
  "rules": {
    // Formatting rules - OFF (Prettier handles these)
    "semi": "off",
    "quotes": "off",
    "indent": "off",
    "comma-dangle": "off",
    "arrow-parens": "off",
    "max-len": "off",
    "no-trailing-spaces": "off",
    "no-multiple-empty-lines": "off",

    // TypeScript formatting rules - OFF
    "@typescript-eslint/semi": "off",
    "@typescript-eslint/quotes": "off",
    "@typescript-eslint/indent": "off",
    "@typescript-eslint/comma-dangle": "off"
  }
}
```

### Kept ESLint Quality Rules

Code quality rules remain **active**:

```json
{
  "rules": {
    // Code quality rules - ON
    "@typescript-eslint/no-unused-vars": "warn",
    "@typescript-eslint/no-explicit-any": "warn",
    "no-console": "off"
  }
}
```

## Workflow

### 1. Format Code (Prettier)

```bash
npm run format
```

Automatically fixes:
- Indentation
- Quotes
- Semicolons
- Line length
- Trailing commas
- Arrow function parentheses

### 2. Check Code Quality (ESLint)

```bash
npm run lint
```

Checks for:
- Unused variables
- Type safety issues
- Logic errors
- Best practice violations

### 3. Auto-fix ESLint Issues

```bash
npx eslint src --ext ts --fix
```

Some ESLint issues can be auto-fixed (like `let` → `const`).

## Verification

Both tools work together without conflicts:

```bash
# Check formatting
$ npm run format:check
✓ All matched files use Prettier code style!

# Check code quality
$ npm run lint
✖ 49 problems (10 errors, 39 warnings)
  8 errors and 0 warnings potentially fixable with the `--fix` option.
```

Notice: ESLint reports **code quality** issues, not formatting issues.

## Current Code Quality Issues

Running `npm run lint` shows:

### Errors (10)
- Empty case declarations (need braces)
- Variables that should be `const` instead of `let`
- Empty function `deactivate()`

### Warnings (39)
- Unused imports
- Use of `any` type (TypeScript)
- Non-null assertions (`!`)
- Unused function parameters

**None of these are formatting issues** - they're all legitimate code quality concerns.

## IDE Integration

### VSCode Settings

The workspace is configured to:
1. **Format on save** with Prettier
2. **Show ESLint warnings** inline
3. **Auto-fix** on save (optional)

### Format on Save

Already enabled in [.vscode/settings.json](vscode/.vscode/settings.json):

```json
{
  "editor.formatOnSave": true,
  "editor.defaultFormatter": "esbenp.prettier-vscode"
}
```

### ESLint Auto-fix on Save (Optional)

To enable ESLint auto-fix on save, add to `.vscode/settings.json`:

```json
{
  "editor.codeActionsOnSave": {
    "source.fixAll.eslint": true
  }
}
```

## Best Practices

### 1. Format First, Then Lint

```bash
npm run format    # Fix formatting
npm run lint      # Check code quality
```

### 2. Don't Fight Prettier

Let Prettier handle **all** formatting decisions:
- Don't manually adjust spacing
- Don't manually add/remove semicolons
- Don't manually format code

### 3. Fix ESLint Issues Manually

ESLint catches real code problems:
- Remove unused variables
- Replace `any` with proper types
- Fix logic errors
- Add missing return types

### 4. Before Committing

```bash
npm run format    # Format code
npm run lint      # Check quality
npm run compile   # Ensure it builds
```

Or just run pretest:

```bash
npm run pretest   # Runs: format + lint + compile
```

## No Conflicts

These configurations ensure **zero conflicts** between Prettier and ESLint:

✅ Prettier controls: semi, quotes, indent, commas, parens, line width
✅ ESLint controls: unused vars, type safety, logic errors, best practices
❌ No overlap: Both tools have clear, separate responsibilities

## Example Issues

### Prettier Issue (Formatting)

```typescript
// Before
const foo={bar:1,baz:2}

// After: npm run format
const foo = { bar: 1, baz: 2 };
```

### ESLint Issue (Code Quality)

```typescript
// Before: lint error "prefer-const"
let x = 5;
console.log(x);

// After: manual fix
const x = 5;
console.log(x);
```

## Summary

- ✅ **Prettier and ESLint work together** without conflicts
- ✅ **Formatting is automatic** (Prettier on save)
- ✅ **Quality issues are caught** (ESLint warnings/errors)
- ✅ **Clear separation** of concerns
- ✅ **No duplicate rules** or conflicts

This setup provides the best developer experience: automatic formatting with powerful code quality checks!
