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

## `jjj submit`

Submit current changes by rebasing onto main and squashing. Automatically accepts the solution and solves the problem when conditions are met.

```
jjj submit [OPTIONS]
```

| Flag | Type | Description |
|------|------|-------------|
| `--force` | bool | Bypass critique and sign-off checks (sets `force_accepted` flag) |

Submit performs these steps:

1. Checks for open critiques (unless `--force`).
2. Checks that all assigned reviewers have signed off (unless `--force`).
3. Rebases the current change onto main and squashes.
4. Auto-accepts the solution if it was in testing or proposed status.
5. Auto-solves the problem if the accepted solution is the only active one and there are no open sub-problems.

Using `--force` bypasses the critique and sign-off gates and sets the `force_accepted` flag on the solution.

```bash
jjj submit
jjj submit --force
```

## `jjj status`

Show status and next actions -- what should I work on?

```
jjj status [OPTIONS]
```

| Flag | Type | Description |
|------|------|-------------|
| `--all` | bool | Show all items (no limit) |
| `--mine` | bool | Only show authored work |
| `--limit` | integer | Show top N items (default: 5) |
| `--json` | bool | Output as JSON |

Shows the active solution for the current jj change, next actions sorted by priority, and a summary of open problems, testing solutions, and open critiques.

```bash,test
jjj status
jjj status --all
jjj status --mine --limit 10
jjj status --json
```

## `jjj fetch`

Fetch code and metadata from remote.

```
jjj fetch [OPTIONS]
```

| Flag | Type | Description |
|------|------|-------------|
| `--remote` | string | Remote to fetch from (default: origin) |

```bash
jjj fetch
jjj fetch --remote upstream
```

## `jjj push`

Push code and metadata to remote.

```
jjj push [bookmarks...] [OPTIONS]
```

| Flag | Type | Description |
|------|------|-------------|
| `--remote` | string | Remote to push to (default: origin) |
| `--no-prompt` | bool | Skip interactive prompts |
| `--dry-run` | bool | Show what would be pushed without pushing |

```bash
jjj push
jjj push my-feature
jjj push --dry-run
```

## `jjj ui`

Launch the interactive TUI dashboard.

```
jjj ui
```

The TUI provides a visual interface for browsing problems, solutions, and their relationships.

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

## `jjj events`

Query the decision event log.

```
jjj events [OPTIONS]
```

| Flag | Type | Description |
|------|------|-------------|
| `--from` | string | Filter by start date (YYYY-MM-DD or YYYY-MM) |
| `--to` | string | Filter by end date |
| `--problem` | string | Filter by problem ID |
| `--solution` | string | Filter by solution ID |
| `--type` | string | Filter by event type |
| `--search` | string | Full-text search in rationales |
| `--json` | bool | Output as JSON |
| `--limit` | integer | Number of events (default: 20) |

```bash
jjj events
jjj events --from 2024-01 --to 2024-06
jjj events --problem p1
jjj events --type solution_accepted
jjj events --search "cache"
```

### `jjj events rebuild`

Rebuild events.jsonl from commit history.

### `jjj events validate`

Validate event log against entity states.

## `jjj timeline`

Show timeline for a problem and all related entities.

```
jjj timeline <problem_id> [OPTIONS]
```

| Flag | Type | Description |
|------|------|-------------|
| `--json` | bool | Output as JSON |

```bash
jjj timeline p1
```
