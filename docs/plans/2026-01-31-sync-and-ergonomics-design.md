# Sync and CLI Ergonomics Design

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Reduce friction in multi-user sync and the create-to-accept workflow.

**Architecture:** Add `jjj push` and `jjj fetch` wrapper commands that sync code + metadata together. Unify solution creation with jj change descriptions. Add smart prompts to reduce manual state transitions.

**Tech Stack:** Rust, existing jjj/jj infrastructure, GitHub CLI (gh) for PR creation.

---

## Phase 1: Core Improvements

### 1. `jjj push` Command

**Purpose:** Replace the 4-step metadata sync with a single command.

**Current pain:**
```bash
git push origin jjj/meta --force
# User must remember this separately from code push
```

**New behavior:**
```bash
jjj push [BOOKMARKS...]
```

1. Run `jj git push` for specified bookmarks (or default behavior if none specified)
2. Always include `jjj/meta` bookmark automatically
3. Display summary: "Pushed main, jjj/meta (2 new critiques synced)"

**Smart prompts after push:**

If all critiques on the user's active solution are resolved:
```
All critiques on s1 "Fix JWT refresh" resolved. Accept solution? [Y/n]
```

If accepting completes the problem (no other active solutions):
```
Solution accepted. Problem p1 "Auth is broken" has no other solutions. Mark solved? [Y/n]
```

**Flags:**
- `--pr` - Create/update GitHub PR (see section 4)
- `--no-prompt` - Skip interactive prompts
- `--dry-run` - Show what would be pushed

### 2. `jjj fetch` Command

**Purpose:** Fetch code and metadata, update the jjj-meta workspace automatically.

**Current pain:**
```bash
git fetch origin jjj/meta:jjj/meta --force
jj git import
cd .jj/jjj-meta && jj new jjj/meta  # Arcane!
```

**New behavior:**
```bash
jjj fetch
```

1. Run `jj git fetch`
2. If jjj-meta workspace exists, update it to the new bookmark target
3. Display summary of changes:
   ```
   Fetched from origin:
     2 new critiques (c3, c4 on s1)
     1 solution updated (s2 now accepted)
   ```

**Flags:**
- `--remote <name>` - Fetch from specific remote (default: origin)

### 3. Solution + Change Description Unification

**Purpose:** Eliminate duplicate descriptions between solution title and jj change.

**Current pain:**
```bash
jjj solution new "Fix JWT refresh" --problem p1
# ... write code ...
jj describe -m "Fix JWT refresh"  # Same text again!
```

**New behavior:**

`jjj solution new` automatically sets the jj change description:

```bash
jjj solution new "Fix JWT refresh handling" --problem p1 --reviewer bob
```

Does three things:
1. Creates solution s1 in jjj metadata
2. Runs `jj describe` with formatted message:
   ```
   s1: Fix JWT refresh handling

   Problem: p1 - Authentication is broken
   ```
3. Attaches current change ID to the solution's `change_ids[]`

**Edge cases:**

- No uncommitted changes (empty working copy):
  ```
  Warning: No uncommitted changes in working copy.
  Solution created but no change attached.
  Start working with: jj new
  ```

- `jjj solution edit s1 --title "New title"`:
  - Updates solution title in metadata
  - Updates jj change description if change is still mutable

### 4. GitHub PR Integration (Basic)

**Purpose:** Create GitHub PRs without leaving the terminal.

**Behavior:**
```bash
jjj push --pr
```

1. Push code + metadata as normal
2. If active solution has no associated PR:
   - Create PR using `gh pr create`
   - Title: solution title
   - Body: solution approach + "Addresses: p1 - Problem title"
   - Mark as draft if open critiques exist
3. If PR exists:
   - Update with force-push
   - Convert from draft to ready if all critiques resolved

**Config (`.jjj/config.toml`):**
```toml
[github]
auto_pr = false        # If true, --pr is implied on every push
repo = "owner/repo"    # Optional, auto-detect from git remote
```

**Storing PR association:**

Add optional field to Solution model:
```rust
pub struct Solution {
    // ... existing fields ...
    pub pr_number: Option<u32>,
    pub pr_url: Option<String>,
}
```

---

## Phase 2: Full GitHub Sync (Future Work)

This section outlines the vision for complete bidirectional GitHub integration. **Not in scope for Phase 1.**

### 2.1 PR Comments → Critiques

When a GitHub PR receives review comments:
- Sync them as critiques with `source: github`
- Include file path and line number
- Map GitHub review states:
  - "Changes requested" → Open critique
  - "Approved" → Dismiss review critique (LGTM)

**Challenges:**
- Webhook or polling for real-time sync
- Deduplication (don't create duplicate critiques)
- Identity mapping (GitHub user → jjj user)

### 2.2 GitHub Issues → Problems

```bash
jjj problem import github#123
```

- Creates problem from GitHub issue
- Links back: `source: github, issue_number: 123`
- Optionally sync status changes bidirectionally

### 2.3 Critique → PR Comment

When a critique is created with `--file` and `--line`:
- Optionally post as PR review comment
- Keep them in sync (edit critique → edit comment)

### 2.4 Full Bidirectional Sync

**Config:**
```toml
[github]
sync_mode = "bidirectional"  # or "push_only", "pull_only", "off"
sync_issues = true
sync_pr_comments = true
```

**Sync triggers:**
- `jjj fetch` pulls GitHub state
- `jjj push` pushes jjj state to GitHub
- Optional: background daemon for real-time sync

---

## Implementation Notes

### Files to modify/create

**New files:**
- `src/commands/push.rs` - Push command implementation
- `src/commands/fetch.rs` - Fetch command implementation
- `src/github.rs` - GitHub API integration (Phase 1: just PR creation)

**Modified files:**
- `src/cli.rs` - Add push/fetch subcommands
- `src/commands/solution.rs` - Auto-describe jj change on solution new
- `src/models/solution.rs` - Add pr_number, pr_url fields
- `src/storage.rs` - Serialize new solution fields

### Dependencies

- `gh` CLI for GitHub operations (avoid adding GitHub API client dependency)
- No new Rust crates required for Phase 1

### Testing

- Update `multi-user-review-test.sh` to use `jjj push` / `jjj fetch`
- Add unit tests for prompt logic (when to offer accept/solve)
- Integration test for GitHub PR creation (requires gh auth)

---

## User Journey After Implementation

```bash
# Start work
jjj solution new "Fix JWT refresh" --problem p1 --reviewer bob
# → Creates s1, sets jj description "s1: Fix JWT refresh\n\nProblem: p1 - Auth broken"
# → Attaches current change to s1

# Write code...

# Share for review
jjj push
# → Pushes code + jjj/meta
# → "Pushed main, jjj/meta"

# Bob reviews (in his terminal)
jjj fetch
# → "Fetched: 1 solution awaiting your review (s1)"
jjj critique new s1 "Token expiry too long" --severity high --file src/auth.rs --line 42
jjj push

# You address feedback
jjj fetch
# → "Fetched: 1 new critique on s1"
jjj critique address c2
jjj push
# → "All critiques on s1 resolved. Accept solution? [Y/n]" → y
# → "Solution accepted. Mark p1 solved? [Y/n]" → y
# → Done!
```

**Command count:** 3 jjj commands for happy path (solution new, push, push) vs ~8-10 previously.
