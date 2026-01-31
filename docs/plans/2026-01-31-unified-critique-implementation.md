# Unified Critique and Review Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Merge the separate review/sign-off system into the critique model, where review requests become "Awaiting review" critiques.

**Architecture:** Add `reviewer` field to Critique model, remove `reviewers`/`sign_offs` from Solution model. The `--reviewer` flag on `solution new` creates an auto-generated critique. Acceptance logic simplifies to just checking for open critiques.

**Tech Stack:** Rust, clap (CLI), serde (serialization), VS Code extension (TypeScript)

---

## Task 1: Add `reviewer` field to Critique model

**Files:**
- Modify: `src/models/critique.rs`
- Test: `src/models/critique.rs` (inline tests)

**Step 1: Write the failing test**

Add to the `tests` module in `src/models/critique.rs`:

```rust
#[test]
fn test_critique_with_reviewer() {
    let mut critique = Critique::new(
        "c1".to_string(),
        "Awaiting review from @bob".to_string(),
        "s1".to_string(),
    );
    critique.reviewer = Some("bob".to_string());
    assert_eq!(critique.reviewer, Some("bob".to_string()));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test test_critique_with_reviewer`
Expected: FAIL with "no field `reviewer` on type `Critique`"

**Step 3: Add reviewer field to Critique struct**

In `src/models/critique.rs`, add to the `Critique` struct:

```rust
    /// Who should address/review this critique
    pub reviewer: Option<String>,
```

And initialize it in `Critique::new()`:

```rust
    reviewer: None,
```

**Step 4: Run test to verify it passes**

Run: `cargo test test_critique_with_reviewer`
Expected: PASS

**Step 5: Commit**

```bash
git add src/models/critique.rs
git commit -m "feat(critique): add reviewer field to Critique model"
```

---

## Task 2: Update Critique serialization for reviewer field

**Files:**
- Modify: `src/storage.rs`

**Step 1: Write the failing test**

Add to storage tests:

```rust
#[test]
fn test_critique_frontmatter_with_reviewer() {
    let mut critique = Critique::new(
        "c1".to_string(),
        "Awaiting review".to_string(),
        "s1".to_string(),
    );
    critique.reviewer = Some("bob".to_string());

    let markdown = to_markdown(&critique);
    assert!(markdown.contains("reviewer: bob"));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test test_critique_frontmatter_with_reviewer`
Expected: FAIL (reviewer not in output)

**Step 3: Update CritiqueFrontmatter struct**

In `src/storage.rs`, add to `CritiqueFrontmatter`:

```rust
    pub reviewer: Option<String>,
```

Update `From<Critique> for CritiqueFrontmatter`:

```rust
    reviewer: critique.reviewer.clone(),
```

Update `From<(CritiqueFrontmatter, String, String)> for Critique`:

```rust
    reviewer: frontmatter.reviewer,
```

**Step 4: Run test to verify it passes**

Run: `cargo test test_critique_frontmatter_with_reviewer`
Expected: PASS

**Step 5: Commit**

```bash
git add src/storage.rs
git commit -m "feat(storage): serialize reviewer field in critique frontmatter"
```

---

## Task 3: Add --reviewer flag to critique new command

**Files:**
- Modify: `src/commands/critique.rs`

**Step 1: Write the failing test**

In `tests/integration_test.rs`, add:

```rust
#[test]
fn test_critique_new_with_reviewer() {
    let dir = setup_test_repo();

    run_jjj(&dir, &["problem", "new", "Test problem"]);
    run_jjj(&dir, &["solution", "new", "Test solution", "--problem", "p1"]);
    let output = run_jjj(&dir, &["critique", "new", "s1", "Review needed", "--reviewer", "bob"]);

    assert!(output.contains("c1"));

    let show_output = run_jjj(&dir, &["critique", "show", "c1", "--json"]);
    assert!(show_output.contains("\"reviewer\":\"bob\""));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test test_critique_new_with_reviewer`
Expected: FAIL (unknown flag --reviewer)

**Step 3: Add --reviewer argument to CritiqueNewArgs**

In `src/commands/critique.rs`, add to `CritiqueNewArgs`:

```rust
    /// Assign a reviewer to address this critique
    #[arg(long)]
    pub reviewer: Option<String>,
```

In the `critique_new` function, after creating the critique:

```rust
    if let Some(reviewer) = &args.reviewer {
        critique.reviewer = Some(reviewer.trim_start_matches('@').to_string());
    }
```

**Step 4: Run test to verify it passes**

Run: `cargo test test_critique_new_with_reviewer`
Expected: PASS

**Step 5: Commit**

```bash
git add src/commands/critique.rs tests/integration_test.rs
git commit -m "feat(cli): add --reviewer flag to critique new command"
```

---

## Task 4: Add --reviewer filter to critique list command

**Files:**
- Modify: `src/commands/critique.rs`

**Step 1: Write the failing test**

In `tests/integration_test.rs`, add:

```rust
#[test]
fn test_critique_list_filter_by_reviewer() {
    let dir = setup_test_repo();

    run_jjj(&dir, &["problem", "new", "Test problem"]);
    run_jjj(&dir, &["solution", "new", "Test solution", "--problem", "p1"]);
    run_jjj(&dir, &["critique", "new", "s1", "For alice", "--reviewer", "alice"]);
    run_jjj(&dir, &["critique", "new", "s1", "For bob", "--reviewer", "bob"]);
    run_jjj(&dir, &["critique", "new", "s1", "No reviewer"]);

    let output = run_jjj(&dir, &["critique", "list", "--reviewer", "alice"]);
    assert!(output.contains("For alice"));
    assert!(!output.contains("For bob"));
    assert!(!output.contains("No reviewer"));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test test_critique_list_filter_by_reviewer`
Expected: FAIL (unknown flag --reviewer on list)

**Step 3: Add --reviewer argument to CritiqueListArgs**

In `src/commands/critique.rs`, add to `CritiqueListArgs`:

```rust
    /// Filter by assigned reviewer
    #[arg(long)]
    pub reviewer: Option<String>,
```

In the `critique_list` function, add filter logic:

```rust
    if let Some(reviewer) = &args.reviewer {
        let reviewer = reviewer.trim_start_matches('@');
        critiques.retain(|c| c.reviewer.as_deref() == Some(reviewer));
    }
```

**Step 4: Run test to verify it passes**

Run: `cargo test test_critique_list_filter_by_reviewer`
Expected: PASS

**Step 5: Commit**

```bash
git add src/commands/critique.rs tests/integration_test.rs
git commit -m "feat(cli): add --reviewer filter to critique list command"
```

---

## Task 5: Add --reviewer flag to solution new command

**Files:**
- Modify: `src/commands/solution.rs`

**Step 1: Write the failing test**

In `tests/integration_test.rs`, add:

```rust
#[test]
fn test_solution_new_with_reviewer() {
    let dir = setup_test_repo();

    run_jjj(&dir, &["problem", "new", "Test problem"]);
    let output = run_jjj(&dir, &["solution", "new", "Test solution", "--problem", "p1", "--reviewer", "bob"]);

    assert!(output.contains("s1"));

    // Should have created an awaiting review critique
    let critiques = run_jjj(&dir, &["critique", "list", "--json"]);
    assert!(critiques.contains("Awaiting review from @bob"));
    assert!(critiques.contains("\"reviewer\":\"bob\""));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test test_solution_new_with_reviewer`
Expected: FAIL (unknown flag --reviewer)

**Step 3: Add --reviewer argument to SolutionNewArgs**

In `src/commands/solution.rs`, add to `SolutionNewArgs`:

```rust
    /// Request review from specified people (creates awaiting review critiques)
    #[arg(long, value_name = "REVIEWER")]
    pub reviewer: Vec<String>,
```

In the `solution_new` function, after saving the solution, add:

```rust
    // Create awaiting review critiques for each reviewer
    for reviewer_spec in &args.reviewer {
        let (reviewer, severity) = parse_reviewer_spec(reviewer_spec);
        let critique_id = store.next_critique_id()?;
        let mut critique = Critique::new(
            critique_id.clone(),
            format!("Awaiting review from @{}", reviewer),
            solution.id.clone(),
        );
        critique.reviewer = Some(reviewer.clone());
        critique.severity = severity;
        critique.author = Some(get_current_user());
        store.save_critique(&critique)?;
        solution.critique_ids.push(critique_id);
    }

    // Re-save solution with critique IDs if we added any
    if !args.reviewer.is_empty() {
        store.save_solution(&solution)?;
    }
```

Add helper function:

```rust
fn parse_reviewer_spec(spec: &str) -> (String, CritiqueSeverity) {
    let spec = spec.trim_start_matches('@');
    if let Some((name, severity)) = spec.split_once(':') {
        let severity = severity.parse().unwrap_or(CritiqueSeverity::Low);
        (name.to_string(), severity)
    } else {
        (spec.to_string(), CritiqueSeverity::Low)
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test test_solution_new_with_reviewer`
Expected: PASS

**Step 5: Commit**

```bash
git add src/commands/solution.rs tests/integration_test.rs
git commit -m "feat(cli): add --reviewer flag to solution new command"
```

---

## Task 6: Remove reviewers and sign_offs from Solution model

**Files:**
- Modify: `src/models/solution.rs`
- Modify: `src/storage.rs`
- Modify: `src/commands/solution.rs`

**Step 1: Remove fields from Solution struct**

In `src/models/solution.rs`, remove:

```rust
    // DELETE these lines:
    pub reviewers: Vec<String>,
    pub sign_offs: Vec<SignOff>,
```

Remove the `SignOff` struct entirely.

Remove these methods:
- `add_reviewer`
- `add_sign_off`
- `all_reviewers_signed_off`
- `pending_reviewers`
- `requires_review`

**Step 2: Update Solution::new()**

Remove initialization of `reviewers` and `sign_offs` fields.

**Step 3: Update storage.rs**

In `SolutionFrontmatter`, remove:

```rust
    // DELETE:
    pub reviewers: Vec<String>,
    pub sign_offs: Vec<SignOffData>,
```

Remove `SignOffData` struct if it exists.

Update the `From` implementations to remove these fields.

**Step 4: Remove review-related CLI commands**

In `src/commands/solution.rs`, remove:
- `SolutionReviewArgs` struct
- `SolutionLgtmArgs` struct
- `solution_review` function
- `solution_lgtm` function

In `src/commands/mod.rs`, remove the review/lgtm command registrations.

**Step 5: Run tests to find remaining references**

Run: `cargo test`
Expected: Compilation errors showing remaining references

**Step 6: Fix remaining references**

Fix each compilation error by removing references to:
- `reviewers`
- `sign_offs`
- `SignOff`
- `requires_review`
- `all_reviewers_signed_off`
- `pending_reviewers`

**Step 7: Run tests to verify**

Run: `cargo test`
Expected: PASS (with some test failures from removed functionality)

**Step 8: Remove tests for deleted functionality**

Remove tests that test the old review/sign-off system.

**Step 9: Run all tests**

Run: `cargo test`
Expected: PASS

**Step 10: Commit**

```bash
git add -A
git commit -m "refactor: remove reviewers and sign_offs from Solution model"
```

---

## Task 7: Simplify acceptance logic

**Files:**
- Modify: `src/commands/solution.rs`
- Modify: `src/commands/workflow.rs`

**Step 1: Write the failing test**

In `tests/workflow_test.rs`, add:

```rust
#[test]
fn test_submit_blocked_by_awaiting_review() {
    let dir = setup_test_repo();

    run_jjj(&dir, &["problem", "new", "Test problem"]);
    run_jjj(&dir, &["solution", "new", "Test solution", "--problem", "p1", "--reviewer", "bob"]);

    // Submit should fail because awaiting review critique is open
    let output = run_jjj_expect_fail(&dir, &["submit"]);
    assert!(output.contains("open critique"));
    assert!(output.contains("Awaiting review from @bob"));
}
```

**Step 2: Run test to verify behavior**

Run: `cargo test test_submit_blocked_by_awaiting_review`

The test may already pass if the old review check was removed. If not, proceed to fix.

**Step 3: Update accept_solution function**

In `src/commands/solution.rs`, simplify `accept_solution`:

Remove any checks for `requires_review()` or `all_reviewers_signed_off()`.

Keep only the check for open critiques:

```rust
let open_critiques: Vec<_> = critiques
    .iter()
    .filter(|c| c.status == CritiqueStatus::Open)
    .collect();

if !open_critiques.is_empty() && !args.force {
    println!("Error: Cannot accept {} - {} open critique(s):", solution.id, open_critiques.len());
    for c in &open_critiques {
        println!("  {}: {} [{}]", c.id, c.title, c.severity);
    }
    println!("\nResolve with: jjj critique address {}", open_critiques[0].id);
    println!("Or dismiss:   jjj critique dismiss {}", open_critiques[0].id);
    println!("Or force:     jjj solution accept {} --force", solution.id);
    return Ok(());
}
```

**Step 4: Update submit function**

In `src/commands/workflow.rs`, apply same simplification to `submit`.

**Step 5: Run tests**

Run: `cargo test`
Expected: PASS

**Step 6: Commit**

```bash
git add src/commands/solution.rs src/commands/workflow.rs tests/workflow_test.rs
git commit -m "refactor: simplify acceptance to only check open critiques"
```

---

## Task 8: Update status command for actionable items

**Files:**
- Modify: `src/commands/status.rs`

**Step 1: Write the failing test**

In `tests/integration_test.rs`, add:

```rust
#[test]
fn test_status_shows_review_needed() {
    let dir = setup_test_repo();

    // Set git user to bob for this test
    std::env::set_var("GIT_AUTHOR_NAME", "bob");

    run_jjj(&dir, &["problem", "new", "Test problem"]);
    run_jjj(&dir, &["solution", "new", "Test solution", "--problem", "p1", "--reviewer", "bob"]);

    let output = run_jjj(&dir, &["status"]);
    assert!(output.contains("[REVIEW]") || output.contains("review"));
    assert!(output.contains("Awaiting review from @bob"));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test test_status_shows_review_needed`
Expected: FAIL (status doesn't show review items with new format)

**Step 3: Update status command**

In `src/commands/status.rs`, update the status output to categorize items:

```rust
// Actionable: critiques assigned to current user
let my_reviews: Vec<_> = critiques
    .iter()
    .filter(|c| c.reviewer.as_deref() == Some(&current_user) && c.status == CritiqueStatus::Open)
    .collect();

// Actionable: my critiques that were addressed (need verification)
let needs_verify: Vec<_> = critiques
    .iter()
    .filter(|c| c.author.as_deref() == Some(&current_user) && c.status == CritiqueStatus::Addressed)
    .collect();

// Waiting: my solutions with open critiques from others
let waiting_on_others: Vec<_> = solutions
    .iter()
    .filter(|s| {
        s.assignee.as_deref() == Some(&current_user) &&
        critiques.iter().any(|c| c.solution_id == s.id && c.status == CritiqueStatus::Open)
    })
    .collect();

// Display actionable first
if !my_reviews.is_empty() || !needs_verify.is_empty() {
    println!("Actionable:");
    for c in &my_reviews {
        println!("  [REVIEW] {} - {}", c.solution_id, c.title);
    }
    for c in &needs_verify {
        println!("  [VERIFY] {} - {} was addressed", c.solution_id, c.title);
    }
}

// Display waiting second
if !waiting_on_others.is_empty() {
    println!("\nWaiting on others:");
    for s in &waiting_on_others {
        println!("  [WAITING] {} - {}", s.id, s.title);
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test test_status_shows_review_needed`
Expected: PASS

**Step 5: Commit**

```bash
git add src/commands/status.rs tests/integration_test.rs
git commit -m "feat(status): show actionable review items first"
```

---

## Task 9: Update VS Code extension types

**Files:**
- Modify: `vscode/src/cli.ts`

**Step 1: Update Solution type**

Remove from `Solution` interface:

```typescript
// DELETE:
reviewers: string[];
sign_offs: SignOff[];
```

Delete `SignOff` interface.

**Step 2: Update Critique type**

Add to `Critique` interface:

```typescript
reviewer?: string;
```

**Step 3: Remove review-related methods**

Delete from `JjjCli` class:
- `requestReview` method
- `lgtm` method

**Step 4: Update newSolution method**

Add optional `reviewers` parameter:

```typescript
async newSolution(title: string, problemId: string, reviewers?: string[]): Promise<string> {
    const args = ["solution", "new", title, "--problem", problemId];
    for (const reviewer of reviewers ?? []) {
        args.push("--reviewer", reviewer);
    }
    return this.exec(args);
}
```

**Step 5: Update newCritique method**

Add optional `reviewer` parameter:

```typescript
async newCritique(
    solutionId: string,
    title: string,
    severity: string,
    filePath?: string,
    line?: number,
    reviewer?: string
): Promise<string> {
    const args = ["critique", "new", solutionId, title, "--severity", severity];
    if (filePath) {
        args.push("--file", filePath);
        if (line) args.push("--line", String(line));
    }
    if (reviewer) {
        args.push("--reviewer", reviewer);
    }
    return this.exec(args);
}
```

**Step 6: Commit**

```bash
git add vscode/src/cli.ts
git commit -m "refactor(vscode): update types for unified critique model"
```

---

## Task 10: Update VS Code extension tests

**Files:**
- Modify: `vscode/test/unit/cli.test.ts`
- Modify: `vscode/test/unit/cache.test.ts`
- Modify: `vscode/test/unit/entityDocument.test.ts`

**Step 1: Remove review/lgtm tests**

In `cli.test.ts`, remove tests for:
- `requestReview`
- `lgtm` / `signOff`

**Step 2: Update solution fixtures**

In all test files, remove `reviewers` and `sign_offs` from solution fixtures:

```typescript
// Before:
{ id: "s1", ..., reviewers: ["alice"], sign_offs: [...] }

// After:
{ id: "s1", ... }
```

**Step 3: Add reviewer to critique fixtures**

```typescript
{ id: "c1", ..., reviewer: "bob" }
```

**Step 4: Run VS Code tests**

Run: `cd vscode && npm test`
Expected: PASS

**Step 5: Commit**

```bash
git add vscode/test/unit/
git commit -m "test(vscode): update tests for unified critique model"
```

---

## Task 11: Update documentation

**Files:**
- Modify: `docs/reference/cli-solution.md`
- Modify: `docs/reference/cli-critique.md`
- Modify: `docs/guides/code-review.md`

**Step 1: Update cli-solution.md**

Remove documentation for:
- `jjj solution review` command
- `jjj solution lgtm` command
- `--review` flag

Add documentation for:
- `--reviewer` flag on `solution new`

**Step 2: Update cli-critique.md**

Add documentation for:
- `--reviewer` flag on `critique new`
- `--reviewer` filter on `critique list`

**Step 3: Update code-review.md**

Rewrite the guide to explain the new unified model where review requests are critiques.

**Step 4: Commit**

```bash
git add docs/
git commit -m "docs: update documentation for unified critique model"
```

---

## Task 12: Final verification

**Step 1: Run all Rust tests**

```bash
cargo test
```

Expected: All tests pass

**Step 2: Run clippy**

```bash
cargo clippy -- -D warnings
```

Expected: No warnings

**Step 3: Run VS Code tests**

```bash
cd vscode && npm test
```

Expected: All tests pass

**Step 4: Manual smoke test**

```bash
jjj init
jjj problem new "Test problem"
jjj solution new "Test solution" --problem p1 --reviewer bob
jjj critique list
# Should show: c1 "Awaiting review from @bob" [low]
jjj critique dismiss c1 --reason "No concerns"
jjj solution accept s1
# Should succeed
```

**Step 5: Commit any fixes**

```bash
git add -A
git commit -m "fix: address issues found in final verification"
```
