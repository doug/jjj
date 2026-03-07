---
title: Code Review Workflow
description: How to request, perform, and complete code reviews using jjj
---

# Code Review Workflow

jjj provides built-in code review capabilities that work seamlessly with Jujutsu's change-based workflow.

## Why Code Review in jjj?

Traditional code review tools (GitHub PRs, GitLab MRs) are designed for Git's commit-based model. They struggle with:

- **History rewrites**: Comments get lost when you rebase
- **Force pushes**: Review state resets
- **Squashing**: All comments disappear

jjj solves this by leveraging **change IDs**, which remain stable across all history modifications.

## Core Concepts

### Change IDs

Every change in Jujutsu has a unique, stable **change ID**:

```bash
$ jj log -r @
@  kpqxywon alice@example.com 2025-11-23 15:30:00 my-feature kpqxywon
|  Add user authentication
~
```

The change ID `kpqxywon` stays the same even if you:
- Rebase onto a different parent
- Amend the change
- Squash with other changes
- Split into multiple changes

This stability makes it perfect for attaching review metadata.

### Unified Critique Model

In jjj, code review is attached to **solutions**, not directly to changes. A solution may have one or more jj changes associated with it. Review requests and critiques are unified into a single model: **critiques with a reviewer field**.

When you assign reviewers at solution creation (using `--reviewer`), jjj automatically creates review-type critiques for each reviewer. These critiques must be resolved (typically via `jjj solution lgtm`) before the solution can be approved.

### Comment Relocation

When code changes, jjj intelligently relocates comments using:

1. **Exact match** (fast path): Line number + content hash match
2. **Fuzzy match**: Similarity scoring finds new location (70% threshold)
3. **Orphaned**: Comment marked as unresolved if context disappears

This is powered by **context fingerprinting** using SHA-256 hashing.

## Basic Workflow

### 1. Assign Reviewers

You can assign reviewers when creating a solution, or add them later using critiques.

At creation:

```bash
jjj solution new "Use JWT tokens" --problem "authentication" --reviewer @alice --reviewer @bob
```

This creates review-type critiques for each reviewer. You can also add reviewers to an existing solution by creating review critiques:

```bash
# Add a reviewer via critique
jjj critique new "JWT tokens" "Review requested" --reviewer @alice
```

Output:
```
Created critique 01958a: Review requested (reviewer: @alice)
```

### 2. Reviewer: Examine the Solution

Alice receives a notification and wants to review the code.

#### Fetch and Checkout

```bash
# 1. Fetch latest changes from remote
jj git fetch

# 2. Check out the change to review
jj new kpqxywon
```

#### Review the Solution Context

Before looking at code, Alice can understand the solution's context:

```bash
# See what problem this solution addresses
jjj solution show "JWT tokens"

# See any existing critiques
jjj critique list --solution "JWT"
```

#### Run Tests

```bash
cargo test
npm start
```

### 3. Reviewer: Raise Critiques or Sign Off

After examining the solution, Alice has two paths.

#### If issues are found: raise a critique

Critiques are the formal mechanism for identifying problems with a solution. They must be resolved before the solution can be accepted.

```bash
# Design-level critique
jjj critique new "JWT tokens" "JWT tokens stored in localStorage are vulnerable to XSS" \
  --severity high

# Code-level critique with file location
jjj critique new "JWT" "Password comparison is not constant-time" \
  --severity critical \
  --file src/auth/password.rs \
  --line 42
```

See the [Critique Guidelines](critique-guidelines.md) for severity levels and how to write effective critiques.

#### If the implementation looks correct: sign off (LGTM)

When a reviewer approves the solution, they sign off with a single command:

```bash
jjj solution lgtm "JWT tokens"
```

This addresses the reviewer's open review critique and records the sign-off with a timestamp.

### 4. Author: Respond to Critiques

Check what critiques are open:

```bash
jjj critique list --solution "JWT" --status open
```

For each critique, address it, dismiss it, or validate it:

```bash
# Fix the issue and mark as addressed
jjj critique address "constant-time"

# Or dismiss with explanation
jjj critique reply "localStorage" "The token is stored in an httpOnly cookie, not localStorage. See the solution's approach section."
jjj critique dismiss "localStorage"
```

After addressing critiques, request re-review if needed by creating a new review critique:

```bash
jjj critique new "JWT" "Re-review requested after fixes" --reviewer @alice
```

### 5. Submit and Approve

Once your solution is ready for review, submit it:

```bash
jjj solution submit "JWT tokens"
```

This changes the solution status to `submitted`, signaling it is ready for critique.

Once all critiques are resolved and reviews are in, approve the solution:

```bash
jjj solution approve "JWT tokens"
```

`jjj solution approve` checks that all critiques are resolved (addressed, dismissed, or validated), including review critiques from assigned reviewers. If any check fails, it explains what is still needed. Use `--force` to bypass the gates in emergencies (this sets the `force_approved` flag on the solution).

## Unified Gate to Acceptance

jjj uses a unified critique model where all feedback -- including review requests -- are critiques:

| Critique Type | What it represents | How to resolve |
|---------------|-------------------|----------------|
| **Regular critique** | A flaw or issue in the approach | Address, dismiss, or validate |
| **Review critique** (has `--reviewer`) | A review request from a specific person | Reviewer addresses it (LGTM) or raises issues |

All critiques must be resolved before a solution can be approved. Review critiques are resolved when the assigned reviewer signs off (via `jjj solution lgtm`).

This unified model means:
- A solution with any open critique (regular or review) cannot be approved
- Review requests and issue critiques follow the same lifecycle
- The `--reviewer` field distinguishes review requests from issue critiques

## Landing Changes

Since jjj decouples review from the forge (GitHub/GitLab), "merging" is updating the main branch.

### Direct Landing

```bash
# Rebase onto latest main
jj git fetch
jj rebase -r kpqxywon -d main

# Advance main bookmark
jj bookmark set main -r kpqxywon

# Push
jj git push -b main
jj git push -b jjj  # Push review metadata
```

### Hybrid Workflow (GitHub/GitLab)

If your team requires Pull Requests for CI gating or compliance:

1. Push your change as a bookmark:
   ```bash
   jj bookmark set my-solution -r kpqxywon
   jj git push -b my-solution
   ```

2. Open a PR on GitHub/GitLab targeting `main`.

3. Paste `jjj solution show "JWT"` output in the PR description to show it has been reviewed and critiques resolved.

4. Merge via the forge's UI.

In the hybrid flow, review and critique happen in jjj. The GitHub PR is used for CI checks and the merge button.

## Advanced Features

### Viewing Review Requests

```bash
# All solutions submitted for review
jjj solution list --status submitted

# Review critiques assigned to you
jjj critique list --reviewer @alice --status open

# Your actionable items
jjj status
```

## Comment Relocation Example

### Initial State

You request review for this code:

```rust
// src/auth/password.rs:40
pub fn hash_password(password: &str) -> Result<String> {
    let salt = generate_salt();
    let hash = sha256(password + &salt);  // Line 42
    Ok(hash)
}
```

Alice raises a critique at line 42:

```bash
jjj critique new "password hashing" "Use bcrypt instead of SHA-256" \
  --severity high \
  --file src/auth/password.rs \
  --line 42
```

### After Rebase

You rebase onto main, which added some imports:

```rust
// src/auth/password.rs:43 (was 40)
use bcrypt::{hash, DEFAULT_COST};  // New imports

pub fn hash_password(password: &str) -> Result<String> {
    let salt = generate_salt();
    let hash = sha256(password + &salt);  // Now line 45 (was 42)
    Ok(hash)
}
```

**Result**: The critique's location automatically relocates from line 42 to line 45.

### After Addressing

You address the critique:

```rust
pub fn hash_password(password: &str) -> Result<String> {
    hash(password, DEFAULT_COST)
        .map_err(|e| AuthError::HashingFailed(e))
}
```

The original code context is gone. jjj detects the content change and marks the critique's location as orphaned, signaling that the code was modified (likely in response to the critique).

## Best Practices

> **Review Early, Review Often**
>
> Request reviews for work-in-progress solutions to get feedback before you invest heavily in an approach.

> **Keep Solutions Focused**
>
> Smaller solutions are faster to review and easier to critique precisely. Aim for solutions that address one problem clearly.

> **Use Solution-Based Bookmarks**
>
> Name your jj bookmarks using a meaningful name to make changes easy to find:
>
>     jj bookmark set solution/jwt-auth

> **Respond to All Critiques**
>
> Every critique must be resolved before acceptance. Either:
> - Fix the issue and mark as addressed
> - Reply with your reasoning and dismiss
> - Acknowledge the flaw and validate (then withdraw the solution)

> **Write Clear Descriptions**
>
> Include context in your solution's approach field so reviewers understand the "why":
>
>     jj describe -m "Add bcrypt password hashing
>
>     Replaces SHA-256 with bcrypt for better security.
>     Uses DEFAULT_COST (12) for work factor.
>
>     Addresses: password security problem"

## Troubleshooting

### Critique Locations Not Relocating

If critique locations do not update after rebase:

1. **Check context**: Did you completely rewrite the section?
2. **View orphaned critiques**: `jjj critique list --solution "JWT"`
3. **Re-create if needed**: Raise a new critique at the correct location

### Approve Fails

If `jjj solution approve` reports unresolved critiques or missing sign-offs:

```bash
# Check what is blocking
jjj critique list --solution "JWT" --status open
jjj solution show "JWT"  # Shows reviewer and sign-off status
```

### Finding the Right Solution

If you forget which solution you are working on:

```bash
# List solutions for a problem
jjj solution list --problem "auth"

# The dashboard shows your assigned work
jjj status
```

## Next Steps

- [Critique Guidelines](critique-guidelines.md) -- How to write and respond to critiques
- [Problem Solving](problem-solving.md) -- The problem lifecycle
- [TUI and Status](board-dashboard.md) -- Visualize sign-off and critique status
