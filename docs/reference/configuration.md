# Configuration

jjj uses a TOML configuration file stored in `.jjj/config.toml` within the repository.

## Config File Format

```toml
# Project name
name = "my-project"

# Default reviewers for all solutions
default_reviewers = ["alice", "bob"]

# Custom key-value settings
[settings]
key = "value"
```

## Fields

| Field | Type | Description |
|-------|------|-------------|
| `name` | string | Project name |
| `default_reviewers` | string[] | Reviewers added to all new solutions |
| `settings` | map | Arbitrary key-value pairs for custom settings |

> **Deprecated:** `review.default_required` has been removed. Review is now per-solution: a solution requires review when it has assigned reviewers. Use `--review` on `jjj solution new` or `jjj solution review` to assign reviewers. Solutions without assigned reviewers do not require sign-offs.

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
