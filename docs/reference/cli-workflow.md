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

## `jjj review`

Assign reviewers to the current change's solution. Shorthand for `jjj solution review`.

Assigned reviewers must sign off before the solution can be accepted.

```
jjj review <reviewers...>
```

```bash
jjj review @alice @bob
```

## `jjj lgtm`

Sign off on the current change's solution. Shorthand for `jjj solution lgtm`.

Records a structured sign-off with your name, timestamp, and optional comment.

```
jjj lgtm [OPTIONS]
```

| Flag | Type | Description |
|------|------|-------------|
| `--comment` | string | Optional comment to include with the sign-off |

```bash
jjj lgtm
jjj lgtm --comment "looks good"
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
