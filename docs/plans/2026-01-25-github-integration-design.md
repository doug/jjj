# GitHub Integration Design

## Overview

Bidirectional sync between jjj and GitHub, allowing teams to use either interface. GitHub features are auto-detected but require explicit commands by default.

## Goals

1. **Meet teams where they are** - GitHub is the standard; jjj should integrate, not replace
2. **Offline-first** - jjj works perfectly without network; sync is explicit
3. **Predictable** - No magic; user controls when sync happens
4. **Optional convenience** - Opt-in auto-push for teams who want it

## Entity Mapping

| jjj | GitHub | Direction |
|-----|--------|-----------|
| Problem | Issue | Bidirectional |
| Solution | Branch + PR | jjj → GitHub |
| Critique | PR "Request Changes" review | GitHub → jjj |
| LGTM | PR "Approve" review | GitHub → jjj |

### Linking Fields

```rust
pub struct Problem {
    // ... existing fields ...
    pub github_issue: Option<u64>,      // Issue number, e.g., 123
}

pub struct Solution {
    // ... existing fields ...
    pub github_pr: Option<u64>,         // PR number
    pub github_branch: Option<String>,  // Branch name
}

pub struct Critique {
    // ... existing fields ...
    pub github_review_id: Option<u64>,  // Review ID if from GitHub
}
```

## Configuration

### Auto-Detection

GitHub integration is auto-detected from the git remote:

```rust
fn detect_github() -> Option<GitHubRepo> {
    let remote = jj_client.git_remote_url("origin")?;

    // Match github.com URLs (SSH or HTTPS)
    // git@github.com:owner/repo.git
    // https://github.com/owner/repo.git
    let (owner, repo) = parse_github_url(&remote)?;

    Some(GitHubRepo { owner, repo })
}
```

### Config File

```toml
# .jjj/config.toml

[github]
# enabled = false          # Disable GitHub integration entirely
# repo = "owner/repo"      # Override auto-detected repo

# Behavior
auto_push = false          # Default: explicit commands required
                           # If true: submit auto-creates PR,
                           #          problem new auto-creates issue,
                           #          accept auto-merges PR
                           # Reading from GitHub still requires explicit sync

# What to sync
sync_critiques = true      # Import "Request Changes" as critiques
sync_lgtm = true           # Import "Approve" as LGTM

# Labels
problem_label = "problem"  # Label added to synced issues
```

### Authentication

```bash
# Option 1: Use gh CLI (recommended, already configured for most devs)
gh auth login
gh auth status  # jjj checks this

# Option 2: Environment variable
export GITHUB_TOKEN=ghp_...
```

## CLI Commands

### New `sync` Command (Extensible for Multiple Sources)

The `sync` command uses `jjj github` pattern, enabling future sources (buganizer, jira, etc.):

```bash
# Sync state from GitHub
jjj github                    # Pull all changes from GitHub
jjj github --dry-run          # Show what would sync

# Import issues as problems
jjj github import #123        # Import specific issue
jjj github import --all       # Import all unlinked open issues
jjj github import --label bug # Import issues with label

# Create PR explicitly (when auto_push = false)
jjj github pr                 # Create PR for current solution
jjj github pr S-1             # Create PR for specific solution

# Status
jjj github status             # Show sync status and linked entities

# Future sources follow same pattern:
# jjj sync buganizer
# jjj sync buganizer import b/12345
# jjj sync jira import PROJ-123
```

### Modified Existing Commands

**When `auto_push = false` (default):**

```bash
jjj problem new "Title"            # Local only
jjj submit                         # Local only (pushes branch, no PR)
jjj solution accept S-1            # Local only (doesn't touch GitHub)
```

**When `auto_push = true`:**

```bash
jjj problem new "Title"            # Creates Problem + GitHub issue
jjj submit                         # Pushes branch + creates/updates PR
jjj solution accept S-1            # Accepts + merges linked PR
jjj problem solve P-1              # Solves + closes linked issue
```

**Always explicit (regardless of config):**

```bash
jjj github                    # Required to pull from GitHub
jjj github import #123        # Required to import issues
```

## Sync Behavior

### What `jjj github` Pulls

| GitHub State | jjj Action |
|--------------|------------|
| New issues (unlinked) | Listed in output, not auto-imported |
| PR "Request Changes" review | Creates Critique (open) |
| PR "Approve" review | Adds reviewer to `solution.reviewed_by` |
| PR merged externally | Marks solution accepted |
| PR closed without merge | Prompts user (refute solution?) |
| Issue closed externally | Prompts user (solve/dissolve problem?) |

### What Auto-Push Does (when enabled)

| jjj Action | GitHub Effect |
|------------|---------------|
| `jjj problem new` | Creates issue |
| `jjj submit` | Creates/updates PR |
| `jjj solution accept` | Merges PR |
| `jjj problem solve` | Closes issue |
| `jjj critique address` | Marks review thread resolved |

## PR Description Template

When creating a PR, jjj generates:

```markdown
## S-3: Fix authentication token refresh

**Problem:** #45 - Token refresh fails silently

### Approach

[Solution description from jjj]

### Critiques

- [ ] CQ-1: Consider rate limiting (medium)
- [x] CQ-2: Add retry logic (addressed)

---
*Managed by [jjj](https://github.com/dougfritz/jjj)*
```

## Example Workflows

### Workflow A: Explicit (default)

```bash
# Create problem locally
jjj problem new "Search is slow"     # P-1 created

# Optionally push to GitHub
jjj github issue P-1            # Creates issue #50

# Work on solution
jjj start "Add search index" --problem P-1
# ... write code ...

# Push branch, create PR explicitly
jjj submit                           # Pushes branch
jjj github pr                   # Creates PR #51

# Teammate reviews on GitHub
jjj github                      # Imports critique CQ-1

# Address and update
jjj critique address CQ-1
jjj submit
jjj github pr                   # Updates PR

# Complete
jjj github                      # Imports LGTM
jjj solution accept S-1              # Local accept
jjj github merge S-1            # Merges PR
jjj problem solve P-1
jjj github close P-1            # Closes issue
```

### Workflow B: Auto-Push Enabled

```toml
# .jjj/config.toml
[github]
auto_push = true
```

```bash
# Create problem - auto-creates issue
jjj problem new "Search is slow"     # P-1 + issue #50

# Work on solution
jjj start "Add search index" --problem P-1
# ... write code ...

# Submit - auto-creates PR
jjj submit                           # PR #51 created

# Teammate reviews on GitHub
jjj github                      # Imports critique CQ-1

# Address and re-submit - auto-updates PR
jjj critique address CQ-1
jjj submit                           # PR updated

# Complete - auto-merges and closes
jjj github                      # Imports LGTM
jjj solution accept S-1              # Merges PR #51
jjj problem solve P-1                # Closes issue #50
```

### Workflow C: Starting from GitHub Issue

```bash
# Someone creates issue #60 on GitHub

# Import it
jjj github import #60           # Creates P-2 linked to #60

# Work normally
jjj start "Fix the bug" --problem P-2
# ... continues as above ...
```

## Error Handling

### Network Failures

```bash
$ jjj submit  # with auto_push = true, GitHub unreachable
Warning: GitHub unreachable, PR not created
✓ Solution S-1 submitted locally
  Branch pushed to origin/s-1-fix-auth

Run 'jjj github pr' when online to create PR.
```

### Conflict Detection

```bash
$ jjj github
Warning: Conflict detected for P-1
  Local: status = open
  GitHub #50: status = closed

Resolve with:
  jjj problem solve P-1        # Accept GitHub state
  jjj github reopen P-1   # Push local state
```

### Auth Failures

```bash
$ jjj github
Error: GitHub authentication failed

Run 'gh auth login' or set GITHUB_TOKEN environment variable.
```

## Files to Create/Modify

### New Files

- `src/commands/sync.rs` - Sync command dispatcher
- `src/sync/mod.rs` - Sync trait and common logic
- `src/sync/github/mod.rs` - GitHub sync implementation
- `src/sync/github/api.rs` - GitHub API client wrapper
- `src/sync/github/mapping.rs` - Entity mapping helpers

### Modified Files

- `src/cli.rs` - Add `SyncAction` enum with source subcommands
- `src/commands/mod.rs` - Add sync module
- `src/models/problem.rs` - Add `github_issue` field
- `src/models/solution.rs` - Add `github_pr`, `github_branch` fields
- `src/models/critique.rs` - Add `github_review_id` field
- `src/models/config.rs` - Add `[github]` config section
- `src/commands/workflow.rs` - Hook auto_push behavior into submit

### Dependencies

```toml
# Cargo.toml
[dependencies]
octocrab = "0.32"  # GitHub API client
# Or use gh CLI via subprocess for simpler auth
```

## Summary

| Aspect | Decision |
|--------|----------|
| Command pattern | `jjj github` - extensible for future sources |
| Sync direction | Bidirectional peer sync |
| Entity mapping | Problem=Issue, Solution=Branch+PR |
| Critique source | PR "Request Changes" only |
| Sync trigger | Explicit `jjj github` for reading |
| Auto-push | Opt-in via `auto_push = true` config |
| Detection | Auto-detect from git remote, opt-out available |
| Auth | Use `gh` CLI or `GITHUB_TOKEN` env var |
| Import | Manual `jjj github import #N` or `--all` |

GitHub integration extends jjj to teams already using GitHub, without compromising offline-first local workflows. The `jjj github` pattern enables future integrations (buganizer, jira, etc.) with consistent UX.
