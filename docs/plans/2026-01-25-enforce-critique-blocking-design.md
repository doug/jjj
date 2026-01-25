# Enforce Critique Blocking Design

## Overview

Open critiques must block solution acceptance. This enforces the Popperian principle that unaddressed criticism should prevent a conjecture from being accepted.

## Goals

1. **Philosophy enforcement** - Criticism must be resolved before acceptance
2. **Audit trail** - Forced acceptances are recorded for accountability
3. **Escape hatch** - `--force` allows pragmatic overrides when necessary

## Rule

**A solution cannot be accepted while it has any open critiques.**

- All severities block equally (low, medium, high, critical)
- Severity indicates priority for addressing, not whether it counts
- If a critique doesn't matter, dismiss it explicitly

## Data Model

```rust
// Add to Solution
pub struct Solution {
    // ... existing fields ...

    // Audit trail for forced acceptance
    pub force_accepted: Option<ForceAcceptRecord>,
}

pub struct ForceAcceptRecord {
    pub by: String,                     // Who forced it
    pub at: DateTime<Utc>,              // When
    pub skipped_critiques: Vec<String>, // Which critiques were open
}
```

## Implementation

```rust
fn accept_solution(solution_id: &str, force: bool) -> Result<()> {
    let mut solution = store.load_solution(solution_id)?;
    let open_critiques = store.list_critiques()?
        .into_iter()
        .filter(|c| c.solution_id == solution_id && c.status == CritiqueStatus::Open)
        .collect::<Vec<_>>();

    if !open_critiques.is_empty() {
        if !force {
            // Block with helpful message
            eprintln!("Error: Cannot accept {} - {} open critique(s):\n",
                solution_id, open_critiques.len());

            for c in &open_critiques {
                let location = c.file_path.as_ref()
                    .map(|f| format!(" - {}:{}", f, c.line_start.unwrap_or(0)))
                    .unwrap_or_default();
                eprintln!("  {}: {} [{}]{}", c.id, c.title, c.severity, location);
            }

            eprintln!();
            eprintln!("Resolve with: jjj critique address {}", open_critiques[0].id);
            eprintln!("Or dismiss:   jjj critique dismiss {}", open_critiques[0].id);
            eprintln!("Or force:     jjj solution accept {} --force", solution_id);

            return Err(JjjError::OpenCritiques(open_critiques.len()));
        }

        // Force-accept: record audit trail
        solution.force_accepted = Some(ForceAcceptRecord {
            by: jj_client.user_identity()?,
            at: Utc::now(),
            skipped_critiques: open_critiques.iter().map(|c| c.id.clone()).collect(),
        });

        eprintln!("Warning: Accepting with {} open critique(s):", open_critiques.len());
        for c in &open_critiques {
            eprintln!("  {}: {} [{}]", c.id, c.title, c.severity);
        }
    }

    solution.accept();
    store.save_solution(&solution)?;

    let status = if solution.force_accepted.is_some() {
        "accepted (forced)"
    } else {
        "accepted"
    };
    println!("✓ Solution {} {}", solution_id, status);

    Ok(())
}
```

## User Experience

### Normal Blocking

```
$ jjj solution accept S-1
Error: Cannot accept S-1 - 2 open critique(s):

  CQ-5: SQL injection risk [high] - src/db.rs:42
  CQ-6: Missing validation [medium]

Resolve with: jjj critique address CQ-5
Or dismiss:   jjj critique dismiss CQ-5
Or force:     jjj solution accept S-1 --force
```

### Force Accept

```
$ jjj solution accept S-1 --force
Warning: Accepting with 2 open critique(s):
  CQ-5: SQL injection risk [high]
  CQ-6: Missing validation [medium]
✓ Solution S-1 accepted (forced)
```

### Viewing Force-Accepted Solution

```
$ jjj solution show S-1

S-1: Fix authentication
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Problem:    P-2 - Token refresh fails
Status:     Accepted (forced)
Author:     doug@example.com
Created:    2026-01-20 10:00:00 UTC
Updated:    2026-01-25 14:30:00 UTC

Force-accepted by: doug@example.com
Force-accepted at: 2026-01-25 14:30:00 UTC
Skipped critiques: CQ-5, CQ-6

## Description

Implement JWT token refresh with explicit error handling...
```

## Storage Format

Add optional fields to solution frontmatter:

```yaml
---
id: S-1
title: Fix authentication
problem_id: P-2
status: accepted
# ... other fields ...

# NEW: Force-accept audit trail (optional)
force_accepted:
  by: doug@example.com
  at: 2026-01-25T14:30:00Z
  skipped_critiques:
    - CQ-5
    - CQ-6
---
```

## Files to Modify

- `src/models/solution.rs` - Add `ForceAcceptRecord` struct and field
- `src/commands/solution.rs` - Update accept logic with blocking and audit
- `src/storage.rs` - Handle force_accepted serialization

## Summary

| Aspect | Decision |
|--------|----------|
| Blocking scope | All open critiques, regardless of severity |
| Escape hatch | `--force` flag |
| Audit trail | Record who, when, which critiques skipped |
| Display | Shown in `jjj solution show` output |

This ensures the Popperian principle is enforced through tooling: criticism must be addressed, dismissed, or explicitly bypassed with accountability.
