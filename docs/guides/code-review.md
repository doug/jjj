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
│  Add user authentication
~
```

The change ID `kpqxywon` stays the same even if you:
- Rebase onto a different parent
- Amend the change
- Squash with other changes
- Split into multiple changes

This stability makes it perfect for attaching review metadata!

### Review Manifest

When you request a review, jjj creates a **review manifest** that tracks:

- Change ID being reviewed
- Requested reviewers
- Review status (Pending, Approved, ChangesRequested)
- All comments and their locations
- Timestamps and metadata

### Comment Relocation

When code changes, jjj intelligently relocates comments using:

1. **Exact match** (fast path): Line number + content hash match
2. **Fuzzy match**: Similarity scoring finds new location (70% threshold)
3. **Orphaned**: Comment marked as unresolved if context disappears

This is powered by **context fingerprinting** using SHA-256 hashing.

## Basic Workflow

### 1. Request a Review

After making changes:

```bash
# Create a change
jj new -m "Add user authentication"
# ... make code changes ...

# Request review from teammates
jjj review request @alice @bob
```

Output:
```
Review requested for change kpqxywon
Reviewers: @alice, @bob
```

### 2. Reviewer: Start Reviewing

Alice receives a notification (or checks pending reviews):

```bash
# List pending reviews
jjj review list --pending

# Start reviewing the change
jjj review start kpqxywon

# Check out the change to review locally
jj edit kpqxywon
```

### 3. Reviewer: Add Comments

Alice can add two types of comments:

#### Inline Comments (File + Line)

```bash
jjj review comment kpqxywon \
  --file src/auth/password.rs \
  --line 42 \
  --body "Consider using bcrypt instead of SHA-256 for password hashing"
```

#### General Comments (No Location)

```bash
jjj review comment kpqxywon \
  --body "Overall looks good! Just a few security concerns to address."
```

### 4. Reviewer: Approve or Request Changes

After reviewing:

```bash
# If everything looks good
jjj review approve kpqxywon

# If changes needed
jjj review request-changes kpqxywon \
  --message "Please address the password hashing concern"
```

### 5. Author: Address Feedback

You see the review status:

```bash
jjj review status kpqxywon
```

Output:
```
Review Status for kpqxywon - Add user authentication

Status: ChangesRequested
Reviewers:
  @alice: ChangesRequested
  @bob: Pending

Comments (3):
  [src/auth/password.rs:42] @alice - Consider using bcrypt...
  [src/auth/login.rs:15] @alice - Add rate limiting...
  [general] @alice - Overall looks good!...
```

Make changes by amending:

```bash
# Edit the change
jj edit kpqxywon

# Make fixes
# ... update code ...

# Amend the change (change ID stays the same!)
jj commit --amend

# Request re-review
jjj review request @alice
```

### 6. Final Approval

Once Alice approves:

```bash
jjj review approve kpqxywon
```

Now both reviewers have approved, and the change can be merged.

## Advanced Features

### Stack Reviews

Review an entire stack of changes:

```bash
# Request review for the entire stack
jjj review request @alice --stack
```

This creates separate review manifests for each change in the stack.

### Mentions in Comments

Reference teammates and other entities:

```bash
jjj review comment kpqxywon \
  --file src/api/routes.rs \
  --line 28 \
  --body "@bob Should this use the same validation as in T-15?"
```

Mentions can reference:
- **Users**: `@alice`, `@bob`
- **Tasks**: `T-1`, `T-42`
- **Features**: `F-1`, `F-2`
- **Bugs**: `B-1`, `B-5`
- **Changes**: `kpqxywon`

### Viewing All Reviews

```bash
# All reviews
jjj review list

# Reviews you requested
jjj review list --mine

# Reviews waiting for your feedback
jjj review list --pending

# JSON for scripting
jjj review list --json
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

Alice comments:

```bash
jjj review comment kpqxywon \
  --file src/auth/password.rs \
  --line 42 \
  --body "Use bcrypt instead of SHA-256"
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

**Result**: Comment automatically relocates from line 42 → 45!

jjj detects:
1. Line 45 has the same content hash as original line 42
2. Surrounding context matches (fuzzy matching)
3. Comment stays attached to the correct line

### After Amending

You address Alice's feedback:

```rust
// src/auth/password.rs:43
use bcrypt::{hash, DEFAULT_COST};

pub fn hash_password(password: &str) -> Result<String> {
    hash(password, DEFAULT_COST)  // Line 45: Content changed!
        .map_err(|e| AuthError::HashingFailed(e))
}
```

**Result**: Comment marks as "resolved" context changed

jjj detects:
1. Line 45 content hash doesn't match original
2. Fuzzy match score < 70%
3. Comment marked as orphaned (context removed)

This signals to Alice that you've addressed her feedback by changing the implementation.

## Integration with Tasks

Link reviews to tasks for tracking:

```bash
# Create a task
jjj task new "Implement user authentication" --feature F-1

# Attach your change to the task
jjj task attach T-1

# Request review (implicitly linked to T-1)
jjj review request @alice

# Alice's approval = T-1 can move to Done
jjj task move T-1 "Done"
```

## Review Dashboard

See all review activity:

```bash
jjj dashboard
```

Output:
```
Dashboard

Pending Reviews:
  kpqxywon... - Add user auth (You requested - @alice, @bob)
    @alice: ChangesRequested
    @bob: Pending

  zxcvbnmq... - Fix login bug (You're reviewing - @charlie)
    Your status: Pending

Tasks:
  In Progress: 2 tasks
  Review: 1 task

Recent Activity:
  kpqxywon... - @alice requested changes
  T-1 moved to Review
```

## Workflow Patterns

### Pattern 1: Pre-Commit Review

Review before merging to main:

```bash
# Author
jj new -m "Add feature X"
# ... code ...
jjj review request @alice @bob
jjj task attach T-5

# Reviewer
jjj review approve kpqxywon

# Author (after approval)
jj bookmark set main
jjj task move T-5 "Done"
```

### Pattern 2: Post-Commit Review

Review after merging (for rapid iteration):

```bash
# Author
jj new -m "Quick fix"
# ... code ...
jj bookmark set main  # Merge immediately

# Request async review
jjj review request @alice

# Alice reviews later, comments on merged code
jjj review comment kpqxywon --body "Consider refactoring this"
```

### Pattern 3: Pair Programming

Real-time review via screen sharing:

```bash
# During pairing session
jjj review request @pair-buddy

# Add comments as discussion notes
jjj review comment kpqxywon \
  --body "Discussed: Should extract this to a helper function"

# Approve immediately after pairing
jjj review approve kpqxywon
```

## Review Checklist

Create consistent review standards:

```markdown
## Code Review Checklist

### Functionality
- [ ] Code does what it's supposed to
- [ ] Edge cases handled
- [ ] Error handling present

### Tests
- [ ] Unit tests added/updated
- [ ] Integration tests if needed
- [ ] Tests actually pass

### Code Quality
- [ ] Follows project conventions
- [ ] No obvious performance issues
- [ ] Comments where needed
- [ ] No debugging code left in

### Security
- [ ] Input validation present
- [ ] No SQL injection risks
- [ ] No XSS vulnerabilities
- [ ] Secrets not hardcoded

### Documentation
- [ ] API docs updated
- [ ] README updated if needed
- [ ] Breaking changes noted
```

Use in review comments:

```bash
jjj review comment kpqxywon \
  --body "$(cat review-checklist.md)"
```

## Best Practices

> **Review Early, Review Often**
>
> Request reviews for work-in-progress changes to get feedback early.
> **Keep Changes Small**
>
> Smaller changes = faster reviews = better feedback. Aim for < 400 lines changed.
> **Review Your Own Code First**
>
> Before requesting review, check your own diff:
>
>     jj show  # Review your own diff
>     # Look for debugging code, TODOs, etc.

> **Use Descriptive Change Descriptions**
>
> Write clear commit messages:
>
>     jj describe -m "Add bcrypt password hashing
>
>     Replaces SHA-256 with bcrypt for better security.
>     Uses DEFAULT_COST (12) for work factor.
>
>     Addresses: B-5 (password security)"

> **Respond to All Comments**
>
> Either:
> - Fix the issue and amend
> - Reply explaining why you disagree
> - Mark as "won't fix" with reason
> **Don't Squash Before Review Complete**
>
> Wait for approval before squashing commits. Comments may be lost!
## Troubleshooting

### Comments Not Relocating

If comments don't relocate after rebase:

1. **Check context**: Did you completely rewrite the section?
2. **Manual relocation**: View orphaned comments and manually update line numbers
3. **Re-comment**: If needed, ask reviewer to re-comment on new location

### Review Status Confusion

If reviewers appear out of sync:

```bash
# Check current status
jjj review status kpqxywon

# Re-sync metadata
jj git fetch
jj bookmark track jjj/meta@origin
```

### Can't Find Change ID

If you forget the change ID:

```bash
# List recent changes
jj log -r 'mine()' -r 'recent()'

# Find by description
jj log -r 'description(authentication)'

# Show change ID
jj log -r @ --no-graph -T 'change_id ++ "\n"'
```

## Next Steps

- [**Task Management Guide**](task-management.md) - Integrate reviews with tasks
- [**CLI Reference: Review Commands**](../reference/cli-review.md) - Complete command docs
- [**Architecture: Comment Relocation**](../architecture/comment-relocation.md) - Technical deep dive
