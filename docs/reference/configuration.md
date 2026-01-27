# Configuration

jjj uses a TOML configuration file stored in `.jjj/config.toml` within the repository.

## Config File Format

```toml
# Project name
name = "my-project"

# Default reviewers for all solutions
default_reviewers = ["alice", "bob"]

# Review settings
[review]
default_required = false

# Custom key-value settings
[settings]
key = "value"

# Tag definitions
[[tags]]
id = "tag-1"
name = "backend"
description = "Backend-related work"
color = "blue"

[[tags]]
id = "tag-2"
name = "urgent"
description = "Needs immediate attention"
color = "red"
```

## Fields

| Field | Type | Description |
|-------|------|-------------|
| `name` | string | Project name |
| `default_reviewers` | string[] | Reviewers added to all new solutions |
| `review.default_required` | bool | Whether solutions require LGTM by default |
| `settings` | map | Arbitrary key-value pairs for custom settings |
| `tags` | array | Tag definitions with id, name, description, color |

## Tag Management

Tags are managed through the `jjj tag` command:

```bash,test
jjj init
jjj tag new "backend" --desc "Backend work" --color blue
jjj tag new "frontend" --desc "Frontend work" --color green
jjj tag list
```

Tags can be edited or deleted:

```bash
jjj tag edit tag-1 --name "server" --color purple
jjj tag delete tag-2
```

Tags are applied to problems, solutions, and milestones using `--tag` on creation or `--add-tag`/`--remove-tag` on edit.

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
| `JJJ_DIR` | Override the metadata directory (default: `.jjj/`) |
| `JJ_PATH` | Path to the jj binary |

## Storage Location

All metadata is stored in `.jjj/` at the repository root:

```
.jjj/
  config.toml       # Project configuration
  problems/         # Problem TOML files
  solutions/        # Solution TOML files
  critiques/        # Critique TOML files
  milestones/       # Milestone TOML files
```
