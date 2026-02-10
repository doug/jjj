# Critique Commands

Critiques are criticisms of solutions. They track issues found during review, with severity levels and a lifecycle: open, addressed, valid, or dismissed. Critiques can reference specific files and lines.

## Entity Resolution

All commands that take a critique or solution reference support multiple resolution methods:

- **Fuzzy title match**: `"error handling"` or `"unused import"` -- matches against titles
- **Truncated prefix**: `01958b` -- minimum 6 hex characters from the UUID
- **Full UUID**: `01958b3c-d4e5-7f6a-7b8c-9d0e1f2a3b4c`

If multiple entities match, an interactive picker appears (TTY) or suggestions are shown (non-TTY).

## `jjj critique new`

Add a critique to a solution.

```
jjj critique new <solution> <title> [OPTIONS]
```

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--severity` | string | `medium` | Severity: low, medium, high, critical |
| `--file` | string | | File path for code-level critique |
| `--line` | integer | | Line number for code-level critique |
| `--reviewer` | string | | Mark as a review request from this reviewer (e.g., `@alice`) |

When `--reviewer` is specified, the critique is created as a review request. The reviewer field indicates which reviewer should address this critique. This unifies the concepts of review requests and critiques into a single model.

```bash,test
jjj init
jjj problem new "Fix login flow"
jjj solution new "Refactor auth handler" --problem "login flow"
jjj critique new "auth handler" "Missing error handling" --severity high
jjj critique new "auth handler" "Unused import" --severity low --file src/auth.rs --line 3
jjj critique list
```

## `jjj critique list`

List critiques.

```
jjj critique list [OPTIONS]
```

| Flag | Type | Description |
|------|------|-------------|
| `--solution` | string | Filter by solution (title, prefix, or UUID) |
| `--status` | string | Filter by status (open, addressed, valid, dismissed) |
| `--reviewer` | string | Filter by reviewer (e.g., `@alice`) |
| `--json` | bool | Output in JSON format |

```bash,test
jjj critique list --solution "auth handler"
jjj critique list --status open
jjj critique list --json
```

## `jjj critique show`

Show critique details.

```
jjj critique show <critique> [OPTIONS]
```

| Flag | Type | Description |
|------|------|-------------|
| `--json` | bool | Output in JSON format |

```bash,test
jjj critique show "error handling"
jjj critique show "error" --json
```

## `jjj critique edit`

Edit critique details.

```
jjj critique edit <critique> [OPTIONS]
```

| Flag | Type | Description |
|------|------|-------------|
| `--title` | string | New title |
| `--severity` | string | New severity (low, medium, high, critical) |
| `--status` | string | New status (open, addressed, valid, dismissed) |

```bash
jjj critique edit "error handling" --severity critical --title "Missing error handling in auth"
```

## `jjj critique address`

Mark a critique as addressed (the solution was modified to fix the issue).

```
jjj critique address <critique>
```

```bash,test
jjj critique address "error handling"
```

## `jjj critique validate`

Validate a critique (confirm it is correct; the solution should be refuted).

```
jjj critique validate <critique>
```

```bash
jjj critique validate "error handling"
```

## `jjj critique dismiss`

Dismiss a critique (incorrect or irrelevant).

```
jjj critique dismiss <critique>
```

```bash
jjj critique dismiss "unused import"
```

## `jjj critique reply`

Reply to a critique.

```
jjj critique reply <critique> <body>
```

```bash
jjj critique reply "error handling" "Fixed in latest change, added error handling for all auth paths"
```
