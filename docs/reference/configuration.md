---
title: Configuration
description: jjj configuration file format, environment variables, and storage location
---

# Configuration

jjj uses a TOML configuration file stored at `.jj/jjj-meta/config.toml` within the repository.

## Config File Format

```toml
# Project name
name = "my-project"

# Default reviewers for all solutions
default_reviewers = ["alice", "bob"]

# Custom key-value settings
[settings]
key = "value"

[github]
enabled = true
repo = "owner/my-project"
auto_push = false
sync_critiques = true
sync_lgtm = true
auto_close_on_solve = true
problem_label = "jjj"

[github.label_priority]
"P0" = "critical"
"P1" = "high"
"P2" = "medium"
"P3" = "low"
```

## Fields

| Field | Type | Description |
|-------|------|-------------|
| `name` | string | Project name |
| `default_reviewers` | string[] | Reviewers added to all new solutions |
| `settings` | map | Arbitrary key-value pairs for custom settings |

> **Deprecated:** `review.default_required` has been removed. Review is now per-solution: a solution requires review when it has assigned reviewers. Use `--review` on `jjj solution new` or `jjj solution review` to assign reviewers. Solutions without assigned reviewers do not require sign-offs.

## `[github]` Section

Controls GitHub integration. All fields are optional and have sensible defaults.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | bool \| null | auto-detect | Enable GitHub integration. When omitted, jjj checks for the `gh` CLI. Set to `false` to disable explicitly. |
| `repo` | string \| null | auto-detect | Override the repository as `"owner/repo"`. Useful when jjj cannot infer the repo from the remote. |
| `auto_push` | bool | `false` | Automatically create issues/PRs on local operations (e.g., `problem new`, `solution new`). |
| `sync_critiques` | bool | `true` | Import "Request Changes" PR reviews as critiques when syncing. |
| `sync_lgtm` | bool | `true` | Import "Approve" PR reviews (LGTM) when syncing. |
| `auto_close_on_solve` | bool | `false` | Close the linked GitHub issue automatically when `jjj problem solve` runs. |
| `problem_label` | string | `"jjj"` | Label applied to GitHub issues created for problems. |
| `label_priority` | map | `{}` | Map custom GitHub label names to jjj priorities. See below. |

### `[github.label_priority]`

Standard priority labels (`p0`, `critical`, `high`, `p1`, `medium`, `p2`, `low`, `p3`) are recognized automatically. Use `label_priority` only when your team uses non-standard label naming conventions.

```toml
[github.label_priority]
"P0 — Critical" = "critical"
"P1 — High"     = "high"
"P2 — Medium"   = "medium"
"P3 — Low"      = "low"
```

When jjj imports an issue with one of these labels, it maps the label to the corresponding jjj priority.

## Automation Rules

Automation rules let you trigger actions when jjj events occur. Rules are defined in `config.toml` using the `[[automation]]` array-of-tables syntax.

```toml
[[automation]]
on = "solution_submitted"
action = "github_pr"

[[automation]]
on = "problem_created"
action = "github_issue"

[[automation]]
on = "solution_approved"
action = "shell"
command = "echo 'Solution {{title}} approved by {{user}}'"

[[automation]]
on = "solution_withdrawn"
action = "shell"
command = "notify-team --msg '{{title}} was withdrawn'"
enabled = false
```

### Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `on` | string | yes | Event type that triggers this rule |
| `action` | string | yes | Action to execute (`shell` or a built-in action) |
| `command` | string | for `shell` | Shell command to run (supports template variables) |
| `enabled` | bool | no | Whether the rule is active (default: `true`) |

### Event Types

| Event | Fires when |
|-------|-----------|
| `problem_created` | A new problem is created |
| `problem_solved` | A problem is marked solved |
| `problem_dissolved` | A problem is dissolved |
| `solution_submitted` | A solution is submitted for review |
| `solution_approved` | A solution is approved |
| `solution_withdrawn` | A solution is withdrawn |
| `critique_created` | A new critique is raised |

### Built-in Actions

| Action | Description |
|--------|-------------|
| `github_issue` | Create a GitHub issue for the problem |
| `github_pr` | Create or update a GitHub PR for the solution |
| `github_merge` | Squash-merge the linked GitHub PR |
| `github_close` | Close the linked GitHub issue |
| `github_sync` | Sync state with GitHub |
| `shell` | Run a custom shell command |

### Template Variables

Shell commands support `{{var}}` template expansion:

| Variable | Description |
|----------|-------------|
| `{{id}}` | Entity UUID |
| `{{title}}` | Entity title |
| `{{user}}` | Current user |
| `{{problem.title}}` | Title of the linked problem |
| `{{pr_number}}` | GitHub PR number (if linked) |

## VS Code Extension

The VS Code extension uses the `jjj.path` setting to locate the jjj binary:

```json
{
  "jjj.path": "/usr/local/bin/jjj"
}
```

If not set, the extension looks for `jjj` on `PATH`.

## Environment Variables

| Variable | Description |
|----------|-------------|
| `JJJ_DIR` | Override the metadata directory (default: `.jj/jjj-meta/`) |
| `JJ_PATH` | Path to the jj binary |

## Storage Location

All metadata is stored in `.jj/jjj-meta/` inside an orphaned `jjj` bookmark. The working copy is never touched by jjj operations.

```
.jj/jjj-meta/
  config.toml       # Project configuration
  problems/         # Problem markdown files
  solutions/        # Solution markdown files
  critiques/        # Critique markdown files
  milestones/       # Milestone markdown files
  events.jsonl      # Append-only event log
```

To sync metadata with collaborators:

```bash
jj git push -b jjj    # Push metadata bookmark
jj git fetch          # Fetch collaborator changes
```
