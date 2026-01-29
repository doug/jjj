# Critique Commands

Critiques are criticisms of solutions. They track issues found during review, with severity levels and a lifecycle: open, addressed, valid, or dismissed. Critiques can reference specific files and lines.

## `jjj critique new`

Add a critique to a solution.

```
jjj critique new <solution_id> <title> [OPTIONS]
```

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--severity` | string | `medium` | Severity: low, medium, high, critical |
| `--file` | string | | File path for code-level critique |
| `--line` | integer | | Line number for code-level critique |

```bash,test
jjj init
jjj problem new "Fix login flow"
jjj solution new "Refactor auth handler" --problem p1
jjj critique new s1 "Missing error handling" --severity high
jjj critique new s1 "Unused import" --severity low --file src/auth.rs --line 3
jjj critique list
```

## `jjj critique list`

List critiques.

```
jjj critique list [OPTIONS]
```

| Flag | Type | Description |
|------|------|-------------|
| `--solution` | string | Filter by solution |
| `--status` | string | Filter by status (open, addressed, valid, dismissed) |
| `--json` | bool | Output in JSON format |

```bash,test
jjj critique list --solution s1
jjj critique list --status open
jjj critique list --json
```

## `jjj critique show`

Show critique details.

```
jjj critique show <critique_id> [OPTIONS]
```

| Flag | Type | Description |
|------|------|-------------|
| `--json` | bool | Output in JSON format |

```bash,test
jjj critique show c1
jjj critique show c1 --json
```

## `jjj critique edit`

Edit critique details.

```
jjj critique edit <critique_id> [OPTIONS]
```

| Flag | Type | Description |
|------|------|-------------|
| `--title` | string | New title |
| `--severity` | string | New severity (low, medium, high, critical) |
| `--status` | string | New status (open, addressed, valid, dismissed) |

```bash
jjj critique edit c1 --severity critical --title "Missing error handling in auth"
```

## `jjj critique address`

Mark a critique as addressed (the solution was modified to fix the issue).

```
jjj critique address <critique_id>
```

```bash,test
jjj critique address c1
```

## `jjj critique validate`

Validate a critique (confirm it is correct; the solution should be refuted).

```
jjj critique validate <critique_id>
```

```bash
jjj critique validate c1
```

## `jjj critique dismiss`

Dismiss a critique (incorrect or irrelevant).

```
jjj critique dismiss <critique_id>
```

```bash
jjj critique dismiss c2
```

## `jjj critique reply`

Reply to a critique.

```
jjj critique reply <critique_id> <body>
```

```bash
jjj critique reply c1 "Fixed in latest change, added error handling for all auth paths"
```
