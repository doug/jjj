# Workflow Commands

Workflow commands provide high-level operations that combine multiple steps. These are the primary interface for day-to-day work with jjj.

## `jjj init`

Initialize jjj in the current repository. Creates the `.jjj/` metadata directory.

```
jjj init
```

```bash,test
jjj init
```

## `jjj start`

Start working on a solution. Either creates a new solution or resumes an existing one.

```
jjj start <arg> [OPTIONS]
```

| Flag | Type | Description |
|------|------|-------------|
| `--problem` | string | Problem this solution addresses (required for new solutions) |

When `arg` matches an existing solution ID (e.g., `S-1`), resumes that solution by switching to its attached change. When `arg` is a title string, creates a new solution and a new jj change.

```bash
# Start a new solution
jjj start "Add caching layer" --problem P-1

# Resume an existing solution
jjj start S-1
```

## `jjj submit`

Submit current changes by rebasing onto main and squashing. Automatically accepts the solution and solves the problem when conditions are met.

```
jjj submit [OPTIONS]
```

| Flag | Type | Description |
|------|------|-------------|
| `--force` | bool | Bypass review and critique checks |

Submit performs these steps:

1. Checks for open critiques and pending reviews (unless `--force`).
2. Rebases the current change onto main and squashes.
3. Auto-accepts the solution if it was in testing or proposed status.
4. Auto-solves the problem if the accepted solution is the only active one and there are no open sub-problems.

```bash
jjj submit
jjj submit --force
```

## `jjj next`

Show next actions -- what should I work on?

```
jjj next [OPTIONS]
```

| Flag | Type | Description |
|------|------|-------------|
| `--all` | bool | Show all items (no limit) |
| `--mine` | bool | Only show authored work |
| `--limit` | integer | Show top N items (default: 5) |
| `--json` | bool | Output as JSON |

Items are sorted by priority. Categories include open critiques to address, solutions needing review, solutions in testing, and open problems without solutions.

```bash,test
jjj next
jjj next --all
jjj next --mine --limit 10
jjj next --json
```

## `jjj review`

Request review on the current change's solution. Shorthand for `jjj solution review`.

```
jjj review <reviewers...>
```

```bash
jjj review @alice @bob
```

## `jjj lgtm`

LGTM the current change's solution. Shorthand for `jjj solution lgtm`.

```
jjj lgtm
```

```bash
jjj lgtm
```

## `jjj board`

Display the board showing solutions organized by status.

```
jjj board [OPTIONS]
```

| Flag | Type | Description |
|------|------|-------------|
| `--json` | bool | Output in JSON format |

```bash,test
jjj board
jjj board --json
```

## `jjj dashboard`

Show dashboard with pending work across problems, solutions, and critiques.

```
jjj dashboard [OPTIONS]
```

| Flag | Type | Description |
|------|------|-------------|
| `--json` | bool | Output in JSON format |

```bash,test
jjj dashboard
jjj dashboard --json
```

## `jjj completion`

Generate shell completions.

```
jjj completion <shell>
```

Supported shells: `bash`, `zsh`, `fish`, `powershell`, `elvish`.

```bash
jjj completion bash >> ~/.bashrc
jjj completion zsh >> ~/.zshrc
jjj completion fish > ~/.config/fish/completions/jjj.fish
```
