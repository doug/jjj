---
title: Workflow Commands
description: CLI reference for init, status, next, fetch, push, sync, github, and other high-level jjj commands
---

# Workflow Commands

Workflow commands provide high-level operations that combine multiple steps. These are the primary interface for day-to-day work with jjj.

## `jjj init`

Initialize jjj in the current repository. Creates the `jjj` bookmark to store metadata.

```
jjj init
```

```bash,test
jjj init
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

Shows the active solution for the current jj change, next actions sorted by priority, and a summary of open problems, solutions in review, and open critiques.

```bash,test
jjj status
jjj status --all
jjj status --mine --limit 10
jjj status --json
```

## `jjj next`

Show the top next actions — a quick summary of what to work on.

```
jjj next [OPTIONS]
```

| Flag | Type | Description |
|------|------|-------------|
| `--top` | integer | Number of items to show (default: 1, 0 for all) |
| `--mine` | bool | Only show items assigned to you |
| `--json` | bool | Output as JSON |

```bash
jjj next
jjj next --top 5
jjj next --mine
jjj next --json
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

## `jjj sync`

Fetch from remote then push local changes back — shorthand for `jjj fetch` followed by `jjj push`.

```
jjj sync [OPTIONS]
```

| Flag | Type | Description |
|------|------|-------------|
| `--remote` | string | Remote to sync with (default: origin) |
| `--no-prompt` | bool | Skip interactive prompts |
| `--dry-run` | bool | Show what would happen without making any changes |

```bash
jjj sync
jjj sync --remote upstream
jjj sync --dry-run
```

## `jjj github`

Bridge jjj problems and solutions with GitHub Issues and Pull Requests.

```
jjj github [COMMAND] [OPTIONS]
```

| Flag | Type | Description |
|------|------|-------------|
| `--dry-run` | bool | Preview actions without making any changes |

### Subcommands

| Command | Description |
|---------|-------------|
| `import [issue] [--all] [--label]` | Import GitHub issue(s) as jjj problems |
| `pr [solution] [--base]` | Create or update a GitHub PR for a solution |
| `status` | Show sync status for all linked problems and solutions |
| `merge <solution>` | Squash-merge the linked GitHub PR for a solution |
| `close <problem>` | Close the linked GitHub issue for a problem |
| `reopen <problem>` | Reopen the linked GitHub issue for a problem |
| `push` | Refresh PR bodies and sync GitHub issue open/closed state |

```bash
jjj github                          # pull review states from GitHub
jjj github import #123              # import issue #123 as a problem
jjj github import --all             # import all unlinked open issues
jjj github import --all --label bug # import issues with a specific label
jjj github pr                       # create PR for current solution
jjj github pr "auth fix" --base dev # create PR against a specific base branch
jjj github status                   # show linked issue/PR status
jjj github merge "auth fix"         # squash-merge the linked PR
jjj github close "old problem"      # close the linked GitHub issue
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
jjj events --problem "auth"
jjj events --type solution_approved
jjj events --search "cache"
```

### `jjj events rebuild`

Rebuild events.jsonl from commit history.

### `jjj events validate`

Validate event log against entity states.

## `jjj timeline`

Show timeline for a problem and all related entities.

```
jjj timeline <problem> [OPTIONS]
```

| Flag | Type | Description |
|------|------|-------------|
| `--json` | bool | Output as JSON |

```bash
jjj timeline "auth bug"
```
