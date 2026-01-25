# Unified Critique System Design

## Overview

Unify the Review and Critique systems into a single criticism model. All feedback—conceptual or code-level—becomes a Critique that targets a Solution.

## Goals

1. **Philosophical alignment** - All criticism is the same: attempts to find flaws in conjectures
2. **Simpler model** - One entity instead of three (Review, Comment, Critique)
3. **Blocking by default** - Open critiques prevent solution acceptance
4. **Flexible review gates** - LGTM required per-project/per-solution configuration

## Data Model

### Critique (Modified)

```rust
pub struct Critique {
    pub id: String,                        // "CQ-1"
    pub title: String,                     // Summary of criticism
    pub solution_id: String,               // What this critiques
    pub status: CritiqueStatus,            // Open/Addressed/Valid/Dismissed
    pub severity: CritiqueSeverity,        // Low/Medium/High/Critical
    pub author: String,

    // Content
    pub argument: String,                  // The criticism (markdown)
    pub evidence: String,                  // Supporting evidence

    // Optional code location (NEW)
    pub file_path: Option<String>,         // e.g., "src/auth.rs"
    pub line_start: Option<usize>,         // e.g., 42
    pub line_end: Option<usize>,           // e.g., 45
    pub code_context: Option<Vec<String>>, // Surrounding lines for display

    // Discussion thread (NEW)
    pub replies: Vec<Reply>,

    // Timestamps
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct Reply {
    pub id: String,                        // "CQ-1-R1"
    pub author: String,
    pub body: String,
    pub created_at: DateTime<Utc>,
}
```

### Solution (Modified)

```rust
pub struct Solution {
    // Existing fields...
    pub id: String,
    pub title: String,
    pub problem_id: String,
    pub status: SolutionStatus,
    pub change_ids: Vec<String>,
    pub tags: HashSet<String>,
    // ...

    // NEW: Review tracking
    pub requested_reviewers: Vec<String>,  // People asked to review
    pub reviewed_by: Vec<String>,          // People who've LGTM'd
    pub requires_review: Option<bool>,     // Override project default
}
```

### Project Config (Modified)

```toml
# .jjj/config.toml
[review]
default_required = true   # Solutions require LGTM by default
```

## Removed Entities

- `ReviewManifest` - functionality absorbed by Solution
- `ReviewStatus` - no longer needed
- `Comment` - replaced by Critique with location
- `CommentLocation` - fields moved to Critique

## Status Flow

### Critique Status (Unchanged)

```
Open ────┬──► Addressed (solution modified to address)
         │
         ├──► Valid (critique correct, solution should be refuted)
         │
         └──► Dismissed (critique incorrect or irrelevant)
```

### Acceptance Rules

A solution can be accepted when:

1. **All critiques resolved** - No critiques with `status == Open`
2. **Review requirement satisfied** (if applicable):
   - `solution.requires_review` overrides project default
   - If review required: at least one person in `reviewed_by` must also be in `requested_reviewers`

```rust
fn can_accept(solution: &Solution, critiques: &[Critique], config: &Config) -> Result<(), AcceptError> {
    // 1. Check all critiques resolved
    let open_critiques: Vec<_> = critiques
        .iter()
        .filter(|c| c.solution_id == solution.id && c.status == CritiqueStatus::Open)
        .collect();

    if !open_critiques.is_empty() {
        return Err(AcceptError::OpenCritiques(open_critiques));
    }

    // 2. Check review requirement
    let requires_review = solution.requires_review
        .unwrap_or(config.review.default_required);

    if requires_review {
        let has_valid_lgtm = solution.reviewed_by
            .iter()
            .any(|r| solution.requested_reviewers.contains(r));

        if solution.requested_reviewers.is_empty() {
            return Err(AcceptError::NoReviewersRequested);
        }
        if !has_valid_lgtm {
            return Err(AcceptError::NoLgtmFromRequestedReviewer);
        }
    }

    Ok(())
}
```

## CLI Commands

### Removed

- `jjj review request`
- `jjj review list`
- `jjj review start`
- `jjj review comment`
- `jjj review status`
- `jjj review approve`
- `jjj review request-changes`

### New/Modified

```bash
# Request review on a solution
jjj solution review S-1 @alice @bob
jjj review @alice @bob              # Shorthand: uses current change's solution

# LGTM a solution (only requested reviewers can do this)
jjj solution lgtm S-1
jjj lgtm                            # Shorthand: current change's solution

# Create critique with optional code location
jjj critique new S-1 "SQL injection risk" --severity high
jjj critique new S-1 "Unsafe query" --file src/db.rs --line 42 --severity critical

# Reply to a critique
jjj critique reply CQ-1 "Good point, will fix"

# Create solution with review override
jjj solution new "Quick fix" --problem P-1 --no-review-required
jjj solution new "Security patch" --problem P-1 --requires-review
```

### Shorthand Resolution

`jjj review @alice` and `jjj lgtm` look up which solution the current jj change belongs to by searching for solutions where `change_ids` contains the current change.

## Participation Rules

- **Anyone** can create critiques on any solution
- **Only requested reviewers** can LGTM a solution
- This enables open criticism (Popperian) while gating acknowledgment to assigned reviewers

## User Experience

### Requesting Review

```bash
$ jjj solution review S-1 @alice @bob
✓ Review requested for S-1: Fix authentication
  Reviewers: @alice, @bob
```

### Adding Critique

```bash
$ jjj critique new S-1 "Query is vulnerable to injection" --file src/db.rs --line 42 --severity high
✓ Created critique CQ-3
  Solution: S-1 - Fix authentication
  Location: src/db.rs:42
  Severity: high
```

### LGTM

```bash
$ jjj lgtm
✓ LGTM recorded for S-1: Fix authentication
  Reviewed by: @alice
```

### Acceptance Blocked

```bash
$ jjj solution accept S-1
Error: Cannot accept S-1

  2 open critiques:
    CQ-3: Query is vulnerable to injection [high] - src/db.rs:42
    CQ-5: Missing rate limiting [medium]

  Resolve with: jjj critique address CQ-3
  Or dismiss:   jjj critique dismiss CQ-3
  Or force:     jjj solution accept S-1 --force
```

### Force Accept

```bash
$ jjj solution accept S-1 --force
Warning: Accepting with 2 open critiques
✓ Solution S-1 accepted
```

## Storage Format

Critique files remain markdown with YAML frontmatter. New optional fields added:

```yaml
---
id: CQ-3
title: Query is vulnerable to injection
solution_id: S-1
status: open
severity: high
file_path: src/db.rs
line_start: 42
line_end: 45
author: alice
created_at: 2026-01-25T10:00:00Z
updated_at: 2026-01-25T10:00:00Z
---

The query on line 42 concatenates user input directly into the SQL string.
This allows attackers to inject arbitrary SQL.

## Evidence

```rust
let query = format!("SELECT * FROM users WHERE id = {}", user_input);
```

## Replies

### bob @ 2026-01-25T10:30:00Z

Good catch, I'll use parameterized queries.

### alice @ 2026-01-25T10:45:00Z

Thanks! Marking as addressed once I see the fix.
```

## Migration

### Files to Remove

- `src/models/review.rs`
- `src/commands/review.rs`

### Files to Modify

- `src/models/critique.rs` - add location fields, Reply struct
- `src/models/solution.rs` - add review tracking fields
- `src/models/config.rs` - add review config section
- `src/models/mod.rs` - remove review exports
- `src/cli.rs` - remove ReviewAction, add new subcommands
- `src/commands/solution.rs` - add `review`, `lgtm` subcommands
- `src/commands/critique.rs` - add `reply` subcommand, `--file`/`--line` flags
- `src/commands/mod.rs` - remove review module
- `src/storage.rs` - remove review/comment storage functions

### Data Migration

- **Existing Reviews** - No automatic migration (transient workflow state)
- **Existing Critiques** - Compatible (new fields are optional)
- **Existing Comments** - Lost unless manually converted to critiques

## Summary

| Before | After |
|--------|-------|
| Review + Comment + Critique | Critique only |
| Review tracks change | Critique tracks solution |
| Approval = Review status | Approval = no open critiques + LGTM |
| Comments inline | Critique with optional file/line |
| No threading | Replies on critiques |
| Anyone can approve | Only requested reviewers can LGTM |
