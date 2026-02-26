---
title: Larger Features Design — Feb 2026
description: Design notes for GitHub bidirectional sync and reviewer sign-off improvements
---

# Larger Features Design

This document covers two features deferred from the Feb 2025 ergonomics pass: **GitHub bidirectional sync** and **reviewer sign-off flow**. Both require more thought than a simple ergonomics fix but are high-impact.

---

## Feature 1: GitHub Bidirectional Sync

### Status

The `jjj github` command (formerly `sync`) already has a skeleton:

| Subcommand | Status |
|-----------|--------|
| `jjj github import #123` | Implemented — creates a Problem from a GitHub issue |
| `jjj github import --all` | Implemented |
| `jjj github pr` | Implemented — creates/updates a PR from active solution |
| `jjj github merge <sol>` | Implemented |
| `jjj github status` | Implemented |
| `jjj github close/reopen` | Implemented |

### What's Missing

**1. PR comments → jjj Critiques**

When a reviewer leaves a GitHub PR comment with "Request Changes", that feedback lives in GitHub only. There's no automated path to import it as a jjj Critique.

Proposed:
```bash
jjj github          # pulls PR review state; on "changes_requested", creates a critique
```

The bare `jjj github` (no subcommand) already refreshes review states. Extend it to:

1. For each linked solution, fetch the GitHub PR reviews via `gh pr view --json reviews`.
2. For each `CHANGES_REQUESTED` review not already tracked, create a critique:
   - `title`: `"GitHub review: @<reviewer>"` (or the first line of the review body)
   - `argument`: full review body
   - `reviewer`: the GitHub login
   - `source`: `github_pr` (new field for tracking origin)

**2. Issue labels → Problem priority**

GitHub issues often have priority labels (`P0`, `high`, `critical`). When importing:

```bash
jjj github import --all --label P0
```

Currently imports with default priority. Add label-to-priority mapping in `.jjj/config.toml`:

```toml
[github.label_priority]
"P0" = "critical"
"P1" = "high"
"P2" = "medium"
"P3" = "low"
```

**3. Problem status → Issue state**

When `jjj problem solve` runs, optionally close the linked GitHub issue. Currently `jjj github close` is a separate explicit step.

Option A: `problem solve` accepts `--github-close` flag.
Option B: config `github.auto_close_on_solve = true`.

Option A is simpler and keeps the behavior opt-in. Recommended.

### Implementation Sketch

```rust
// In src/commands/sync.rs — existing GithubAction::Refresh handler
fn refresh_reviews(ctx, dry_run) -> Result<()> {
    let solutions_with_prs = store.list_solutions()?.into_iter()
        .filter(|s| s.github_pr.is_some())
        .collect();

    for solution in &solutions_with_prs {
        let pr_num = solution.github_pr.unwrap();
        let reviews = github_client.get_pr_reviews(pr_num)?;

        for review in reviews.iter().filter(|r| r.state == "CHANGES_REQUESTED") {
            // Check if we already have a critique for this review
            let already_tracked = store.list_critiques()?.iter().any(|c| {
                c.solution_id == solution.id
                    && c.reviewer.as_deref() == Some(&review.login)
                    && c.source.as_deref() == Some("github_pr")
            });

            if !already_tracked {
                let critique = Critique::new(
                    store.next_critique_id()?,
                    solution.id.clone(),
                    format!("GitHub review: @{}", review.login),
                );
                // ...set reviewer, argument, source
                store.save_critique(&critique)?;
                println!("Imported GitHub review as critique: {}", critique.id);
            }
        }
    }
    Ok(())
}
```

New field needed in `Critique`:
```rust
pub source: Option<String>,  // "github_pr", None = local
```

### Open Questions

1. **Direction of truth**: If a GitHub review is imported as a critique and then addressed in jjj, should we post back to GitHub? Complexity increases significantly. Recommended: one-way for now (GitHub → jjj), explicit `jjj github pr` to push changes back.

2. **PR descriptions**: Currently `jjj github pr` creates a PR but the description is minimal. Should it auto-include the solution's `approach` and open critiques? Recommended: yes, include approach + critique summary.

3. **Authentication**: `jjj github` uses `gh` CLI for auth. This is correct and we should keep it that way rather than managing tokens ourselves.

---

## Feature 2: Reviewer Sign-off Flow

### Status

The current flow (per `docs/guides/code-review.md`):

1. Reviewer is assigned via `--reviewer @alice` on `solution new`, which creates a review-type critique.
2. Alice addresses her critique to sign off (LGTM).
3. All critiques (including review ones) must be resolved before `solution accept`.

This works correctly. The friction is that the sign-off command is indirect: `jjj critique address <critique-id>`. Users often forget the critique ID.

### Proposed: `jjj solution lgtm`

A dedicated shorthand for "I've reviewed this and it's good":

```bash
jjj solution lgtm "JWT tokens"
# → Finds the open review-critique assigned to the current user
# → Addresses it
# → Prints: "Signed off on 'JWT tokens' as @alice"
```

**Implementation:**

```rust
// SolutionAction::Lgtm { solution_id }
fn lgtm_solution(ctx: &CommandContext, solution_input: String) -> Result<()> {
    let solution_id = ctx.resolve_solution(&solution_input)?;
    let store = &ctx.store;
    let current_user = store.get_current_user()?;

    // Find the review critique assigned to this user
    let critiques = store.get_critiques_for_solution(&solution_id)?;
    let my_review = critiques.iter().find(|c| {
        c.status == CritiqueStatus::Open
            && c.reviewer.as_ref().map(|r| r.contains(&current_user)).unwrap_or(false)
    });

    match my_review {
        Some(critique) => {
            // Address it
            store.with_metadata(&format!("LGTM on solution {}", solution_id), || {
                let mut c = store.load_critique(&critique.id)?;
                c.address();
                store.save_critique(&c)?;
                println!("Signed off on '{}' as @{}", solution.title, current_user);
                Ok(())
            })
        }
        None => {
            Err(JjjError::Validation(format!(
                "No open review critique assigned to you on this solution.\n\
                 To assign yourself as reviewer: jjj critique new \"{}\" \"Review\" --reviewer @{}",
                solution_id, current_user
            )))
        }
    }
}
```

**CLI addition in `cli.rs`:**

```rust
/// Sign off on a solution as a reviewer (LGTM — addresses your review critique)
Lgtm {
    solution_id: String,
},
```

### Considerations

- `solution lgtm` is intentionally symmetric with `solution review` (move to review) — both are solution-level state commands about the review process.
- It does not auto-accept the solution; that still requires an explicit `solution accept`.
- If the solution has **no** open review critique for the current user, the command fails with a helpful message. This prevents accidental LGTMs.
- Multi-reviewer: each reviewer calls `solution lgtm` independently. When all review critiques are addressed and no other open critiques remain, `solution accept` will succeed.

### VS Code Integration

Add `jjj.lgtmSolution` command in the extension that:
1. Gets active solution from cache
2. Calls `cli.lgtmSolution(solution.id)`
3. Shows: `"You've signed off on '${title}'. All critiques resolved? Run 'Accept Solution'."`

This is a quick-win since the VS Code status bar already shows the active solution.

---

## Summary and Prioritization

| Feature | Value | Complexity | Recommended Order |
|---------|-------|------------|-------------------|
| `solution lgtm` command | Medium | Low | **Do first** — small, clean |
| GitHub PR → Critique import | High | Medium | Second |
| Label → Priority mapping | Low | Low | Bundle with GitHub PR import |
| Auto-close issue on solve | Medium | Low | Bundle with GitHub PR import |
| PR description from approach | Medium | Low | Bundle with `jjj github pr` |

The `solution lgtm` shorthand is the clearest win: it removes the most common friction point in the review flow and is a natural companion to `solution review`. The GitHub PR import is valuable for teams that use both tools but requires careful design around the "direction of truth" question.
