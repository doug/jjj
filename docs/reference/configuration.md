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

> **Deprecated:** `review.default_required` has been removed. Review is now per-solution: a solution requires review when it has assigned reviewers. Use `--reviewer` on `jjj solution new` to assign reviewers at creation, or create review critiques with `jjj critique new "solution" "Review requested" --reviewer @name`. Solutions without assigned reviewers do not require sign-offs.

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

Automation rules let you trigger actions when jjj events occur. Rules are defined in
`.jj/jjj-meta/automation.toml` using the `[[automation]]` array-of-tables syntax.

> **Why a separate file.** `automation.toml` is machine-local: push never copies it and
> fetch never writes it. `config.toml`, by contrast, syncs through the shared `jjj`
> bookmark — a `action = "shell"` rule placed there would execute whatever a collaborator
> pushed, on every clone. Rules found in `config.toml` are reported and ignored; run
> `jjj automation migrate --force` to move them.

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
| `problem_reopened` | A solved/dissolved problem is reopened |
| `solution_created` | A new solution is proposed |
| `solution_submitted` | A solution is submitted for review |
| `solution_approved` | A solution is approved |
| `solution_withdrawn` | A solution is withdrawn |
| `critique_raised` | A new critique is raised |
| `critique_addressed` | A critique is addressed |
| `critique_dismissed` | A critique is dismissed |
| `critique_validated` | A critique is validated |
| `critique_replied` | A reply is added to a critique |
| `milestone_created` | A new milestone is created |
| `milestone_completed` | A milestone is completed |

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

### Identity and sync

| Variable | Description |
|----------|-------------|
| `JJJ_USER` | Who the actor is. Takes precedence over the jj user name, so every event and claim is attributable. Set it per agent in a fleet; a blank or whitespace-only value is ignored rather than honoured. |
| `JJJ_POD` | Names this clone's push bookmark and event shard (`events/{user}.jsonl`). Setting only `JJJ_POD` still yields a stable identity. It does **not** override `last_synced_rev`, which always comes from `.sync_state.json`. |

### Diagnostics and measurement

| Variable | Description |
|----------|-------------|
| `JJJ_DEBUG` | When set to anything, echoes every `jj` and `gh` subprocess invocation to stderr before running it. |
| `JJJ_TELEMETRY` | `1` appends one JSON line per command (`{ts, command, subcommand, ms}`) to `.jj/jjj-meta/.telemetry.jsonl`. Opt-in; unset, the cost is a single `getenv`. It cannot see invocations clap rejects, because timing starts after argument parsing. |
| `JJJ_NO_DB` | Makes the store behave as if `.jj/jjj.db` had never been built, so reads fall through to the canonical markdown walk. **A read-path measurement switch, not a supported way to run**: commands that open the database directly and have no filesystem fallback — `db rebuild`, `search`, and the `--search` flags — are unaffected and still require the file, and a write taken with it set skips the cache upsert, leaving an existing cache stale. |

### Embeddings

These override the matching key in `[embeddings]` of `.jj/jjj.toml` (see
[Local configuration](#local-configuration)). Booleans accept `true` or `1`.

| Variable | Description |
|----------|-------------|
| `JJJ_EMBEDDINGS_ENABLED` | Turn embeddings on. When explicitly on, a connection failure warns instead of passing silently. |
| `JJJ_EMBEDDINGS_MODEL` | Model name label, e.g. `all-MiniLM-L6-v2`. |
| `JJJ_EMBEDDINGS_MODEL_PATH` | Directory holding the local model files (`config.json`, `tokenizer.json`, `model.safetensors`). Default: `~/.cache/jjj/models/all-MiniLM-L6-v2`. |
| `JJJ_EMBEDDINGS_DIMENSIONS` | Embedding dimensions. A value that does not parse as a number is ignored. |
| `JJJ_EMBEDDINGS_API_KEY` | API key for a remote embedding service. **Prefer this to the config file** — a key in `.jj/jjj.toml` is stored in plaintext and jjj warns about it on every load. |
| `JJJ_EMBEDDINGS_BASE_URL` | Base URL for a remote embedding API. Unused by the in-process `semantic` backend; kept so older config files still parse. |
| `JJJ_EMBEDDINGS_DUPLICATE_CHECK` | Check for near-duplicates when creating an entity. Default: on when embeddings are enabled. |
| `JJJ_EMBEDDINGS_DUPLICATE_THRESHOLD` | Similarity above which a duplicate is reported. Default: `0.85`. |

### Standard variables jjj honours

`EDITOR` and `VISUAL` choose the editor for `-E` / `problem edit --editor`;
`XDG_CONFIG_HOME` and `HOME` locate the user-level config; `PATH` is how the
`jj` and `gh` binaries are found.

## Local configuration

`.jj/jjj.toml` holds machine-local settings and is **never synced** — unlike
`.jj/jjj-meta/config.toml`, which travels through the shared `jjj` bookmark.

```toml
[embeddings]
enabled = true
model = "all-MiniLM-L6-v2"
model_path = "~/.cache/jjj/models/all-MiniLM-L6-v2"
dimensions = 384
duplicate_check_enabled = true
duplicate_threshold = 0.85
```

Environment variables override these per-key. Keys absent from the file fall
back to their defaults; a missing file is equivalent to an empty one.

## Storage Location

Metadata lives in `.jj/jjj-meta/` on an orphaned `jjj` bookmark. The working
copy is never touched by jjj operations.

**Synced** — published by `jjj push`, fetched by `jjj fetch`:

```
.jj/jjj-meta/
  config.toml         # Project configuration (shared)
  problems/           # Problem markdown files
  solutions/          # Solution markdown files
  critiques/          # Critique markdown files
  findings/           # Finding markdown files
  milestones/         # Milestone markdown files
  rankings/           # rankings/{milestone_id}/{user}.json — per-user ordering
  events.jsonl        # Append-only event log
  events/{user}.jsonl # Per-pod event shards, union-merged on fetch
```

**Machine-local** — deliberately not synced, and safe to delete:

```
.jj/jjj-meta/
  automation.toml       # Executable rules. Local by design: a rule in the
                        # synced config.toml would let any collaborator run
                        # shell commands on every clone.
  .sync_state.json      # This clone's last_synced_rev and pod name
  .events_offsets.json  # Offset index making a shard tail an O(delta) read
  .lamport              # This clone's logical clock
  .telemetry.jsonl      # Only written when JJJ_TELEMETRY=1
.jj/
  jjj.toml              # Local configuration (above)
  jjj.db                # SQLite cache. Disposable: rebuild with `jjj db
                        # rebuild`. Markdown is the source of truth, and
                        # `jjj push` validates the markdown without it.
```

To sync metadata with collaborators:

```bash
jj git push -b jjj    # Push metadata bookmark
jj git fetch          # Fetch collaborator changes
```
