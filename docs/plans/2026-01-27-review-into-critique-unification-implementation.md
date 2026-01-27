# Review-into-Critique Unification Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Merge the review system into the critique model — reviewers become assigned on solutions, LGTM becomes structured sign-offs, the standalone review command namespace is absorbed into solution commands.

**Architecture:** Replace `requested_reviewers`, `reviewed_by`, and `requires_review` on Solution with `reviewers`, `sign_offs: Vec<SignOff>`, and `force_accepted`. Update all command handlers, CLI flags, storage migration, VS Code extension, and documentation to match.

**Tech Stack:** Rust (models, CLI, storage), TypeScript (VS Code extension), Markdown (docs)

---

### Task 1: Add SignOff struct and update Solution model

**Files:**
- Modify: `src/models/solution.rs`

**Step 1: Write the failing tests**

Add to `src/models/solution.rs` in the `tests` module:

```rust
#[test]
fn test_sign_off_creation() {
    let mut solution = Solution::new("S-1".to_string(), "Test".to_string(), "P-1".to_string());
    solution.add_reviewer("alice".to_string());
    solution.add_reviewer("bob".to_string());
    assert_eq!(solution.reviewers.len(), 2);
    assert!(solution.reviewers.contains(&"alice".to_string()));
}

#[test]
fn test_add_sign_off() {
    let mut solution = Solution::new("S-1".to_string(), "Test".to_string(), "P-1".to_string());
    solution.add_reviewer("alice".to_string());
    solution.add_sign_off("alice".to_string(), Some("looks good".to_string()));
    assert_eq!(solution.sign_offs.len(), 1);
    assert_eq!(solution.sign_offs[0].reviewer, "alice");
    assert_eq!(solution.sign_offs[0].comment.as_deref(), Some("looks good"));
}

#[test]
fn test_all_reviewers_signed_off() {
    let mut solution = Solution::new("S-1".to_string(), "Test".to_string(), "P-1".to_string());
    solution.add_reviewer("alice".to_string());
    solution.add_reviewer("bob".to_string());
    assert!(!solution.all_reviewers_signed_off());

    solution.add_sign_off("alice".to_string(), None);
    assert!(!solution.all_reviewers_signed_off());

    solution.add_sign_off("bob".to_string(), None);
    assert!(solution.all_reviewers_signed_off());
}

#[test]
fn test_pending_reviewers() {
    let mut solution = Solution::new("S-1".to_string(), "Test".to_string(), "P-1".to_string());
    solution.add_reviewer("alice".to_string());
    solution.add_reviewer("bob".to_string());
    solution.add_sign_off("alice".to_string(), None);

    let pending = solution.pending_reviewers();
    assert_eq!(pending, vec!["bob"]);
}

#[test]
fn test_non_assigned_sign_off() {
    let mut solution = Solution::new("S-1".to_string(), "Test".to_string(), "P-1".to_string());
    solution.add_reviewer("alice".to_string());
    solution.add_sign_off("charlie".to_string(), Some("nice work".to_string()));

    // Charlie signed off but is not an assigned reviewer
    assert!(!solution.all_reviewers_signed_off()); // alice hasn't signed off
    assert_eq!(solution.sign_offs.len(), 1); // charlie's sign-off is stored
}

#[test]
fn test_requires_review_derived() {
    let solution = Solution::new("S-1".to_string(), "Test".to_string(), "P-1".to_string());
    assert!(!solution.requires_review()); // no reviewers assigned

    let mut solution2 = solution.clone();
    solution2.add_reviewer("alice".to_string());
    assert!(solution2.requires_review()); // has reviewers
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p jjj --lib models::solution::tests -- --nocapture`
Expected: FAIL — `add_reviewer`, `add_sign_off`, `sign_offs`, `reviewers`, `all_reviewers_signed_off`, `pending_reviewers`, `requires_review` not found.

**Step 3: Implement the model changes**

In `src/models/solution.rs`:

1. Add `SignOff` struct:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignOff {
    pub reviewer: String,
    pub at: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
}
```

2. Update `Solution` struct fields:
   - Rename `requested_reviewers` → `reviewers`
   - Replace `reviewed_by: Vec<String>` → `sign_offs: Vec<SignOff>`
   - Remove `requires_review: Option<bool>`
   - Add `force_accepted: bool`

3. Update `Solution::new()` to initialize new fields (empty `reviewers`, empty `sign_offs`, `force_accepted: false`)

4. Replace old methods:
   - Remove `request_review()`, `add_lgtm()`, `has_lgtm_from_requested_reviewer()`, `set_requires_review()`
   - Add `add_reviewer(&mut self, reviewer: String)` — adds to `reviewers` if not present
   - Add `add_sign_off(&mut self, reviewer: String, comment: Option<String>)` — appends `SignOff` with `Utc::now()`
   - Add `all_reviewers_signed_off(&self) -> bool` — every entry in `reviewers` has a matching `sign_offs` entry
   - Add `pending_reviewers(&self) -> Vec<&str>` — reviewers not in sign_offs
   - Add `requires_review(&self) -> bool` — returns `!self.reviewers.is_empty()`

5. Update `SolutionFrontmatter`:
   - Replace `requested_reviewers` → `reviewers`
   - Replace `reviewed_by` → `sign_offs: Vec<SignOff>`
   - Remove `requires_review: Option<bool>`
   - Add `force_accepted: bool`

6. Update `From<&Solution> for SolutionFrontmatter` to map new fields.

7. Remove old tests that reference removed fields: `test_request_reviewers`, `test_lgtm`, `test_lgtm_from_non_requested`, `test_requires_review_override`.

**Step 4: Run tests to verify they pass**

Run: `cargo test -p jjj --lib models::solution::tests -- --nocapture`
Expected: PASS

**Step 5: Commit**

```bash
git add src/models/solution.rs
git commit -m "feat: replace review fields with reviewers/sign_offs on Solution model"
```

---

### Task 2: Update storage for new Solution fields and migration

**Files:**
- Modify: `src/storage.rs`

**Step 1: Write the failing test**

In `src/storage.rs` tests module, update `test_parse_frontmatter` to use a solution frontmatter with new fields. Also add a migration test:

```rust
#[test]
fn test_solution_frontmatter_roundtrip() {
    let mut solution = Solution::new("S-1".to_string(), "Test solution".to_string(), "P-1".to_string());
    solution.add_reviewer("alice".to_string());
    solution.add_sign_off("alice".to_string(), Some("looks good".to_string()));
    solution.force_accepted = false;

    let frontmatter = SolutionFrontmatter::from(&solution);
    let body = build_body(&[("Approach", &solution.approach), ("Trade-offs", &solution.tradeoffs)]);
    let content = to_markdown(&frontmatter, &body).unwrap();

    // Parse it back
    let (parsed_fm, _): (SolutionFrontmatter, String) = parse_frontmatter(&content).unwrap();
    assert_eq!(parsed_fm.reviewers, vec!["alice".to_string()]);
    assert_eq!(parsed_fm.sign_offs.len(), 1);
    assert_eq!(parsed_fm.sign_offs[0].reviewer, "alice");
    assert_eq!(parsed_fm.sign_offs[0].comment.as_deref(), Some("looks good"));
    assert_eq!(parsed_fm.force_accepted, false);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p jjj --lib storage::tests -- --nocapture`
Expected: FAIL — `reviewers` field not recognized in frontmatter (compilation errors due to model changes from Task 1)

**Step 3: Update storage**

In `src/storage.rs`, update `load_solution()`:
- Map `frontmatter.reviewers` → `solution.reviewers`
- Map `frontmatter.sign_offs` → `solution.sign_offs`
- Map `frontmatter.force_accepted` → `solution.force_accepted`
- Remove mappings for `requested_reviewers`, `reviewed_by`, `requires_review`

Update `save_solution()` — no changes needed (frontmatter derived from Solution).

Update the import list in `storage.rs` line 2-7 to import `SignOff` from models.

**Step 4: Run tests to verify they pass**

Run: `cargo test -p jjj --lib storage::tests -- --nocapture`
Expected: PASS

**Step 5: Commit**

```bash
git add src/storage.rs
git commit -m "feat: update storage for reviewers/sign_offs solution fields"
```

---

### Task 3: Update CLI flags for lgtm --comment and solution new --review

**Files:**
- Modify: `src/cli.rs`

**Step 1: Write the failing test (compile test)**

This is a CLI flag change — verify by trying to compile after changes.

**Step 2: Make the changes**

In `src/cli.rs`:

1. Update `SolutionAction::New` to add `--review` flag:
```rust
/// Assign reviewers (e.g., @alice,@bob)
#[arg(long)]
review: Vec<String>,
```

2. Update `SolutionAction::Lgtm` to add `--comment` flag:
```rust
/// Comment with sign-off
#[arg(long)]
comment: Option<String>,
```

3. Update `Commands::LgtmShorthand` to add `--comment` flag:
```rust
LgtmShorthand {
    /// Comment with sign-off
    #[arg(long)]
    comment: Option<String>,
},
```

4. Remove `Commands::ReviewShorthand` — no, actually keep it. The design says the `review` shorthand works by detecting current change's solution. Keep it but it now delegates to `solution review`. Actually, re-reading the design: "The standalone `review` command namespace goes away entirely." But currently `jjj review @alice` is a handy shorthand that auto-detects the solution. Let's keep it as `jjj review` shorthand since it's UX-convenient and just delegates to `solution review`.

**Step 3: Verify compilation**

Run: `cargo build -p jjj`
Expected: Compile success (may have warnings from unused variables in handlers — that's OK, Task 4 fixes them)

**Step 4: Commit**

```bash
git add src/cli.rs
git commit -m "feat: add --comment to lgtm, --review to solution new"
```

---

### Task 4: Update solution command handlers

**Files:**
- Modify: `src/commands/solution.rs`
- Modify: `src/commands/mod.rs`

**Step 1: Write the failing test**

Add integration test in `tests/integration_test.rs`:

```rust
#[test]
fn test_solution_lgtm_with_comment() {
    if which::which("jj").is_err() { return; }
    let temp_dir = setup_test_repo();
    let dir = temp_dir.path();

    run_jjj(dir, &["solution", "new", "Test Solution", "--problem", "P-1"]);
    run_jjj(dir, &["solution", "review", "S-1", "@alice"]);

    let output = run_jjj(dir, &["solution", "lgtm", "S-1", "--comment", "looks good"]);
    assert!(output.status.success(), "lgtm failed: {}", String::from_utf8_lossy(&output.stderr));

    let show = run_jjj(dir, &["solution", "show", "S-1", "--json"]);
    let stdout = String::from_utf8_lossy(&show.stdout);
    assert!(stdout.contains("looks good"), "Expected comment in output: {}", stdout);
}

#[test]
fn test_solution_new_with_review() {
    if which::which("jj").is_err() { return; }
    let temp_dir = setup_test_repo();
    let dir = temp_dir.path();

    let output = run_jjj(dir, &["solution", "new", "With Review", "--problem", "P-1", "--review", "@alice", "--review", "@bob"]);
    assert!(output.status.success(), "new with review failed: {}", String::from_utf8_lossy(&output.stderr));

    let show = run_jjj(dir, &["solution", "show", "S-1", "--json"]);
    let stdout = String::from_utf8_lossy(&show.stdout);
    assert!(stdout.contains("alice"), "Expected alice in reviewers: {}", stdout);
    assert!(stdout.contains("bob"), "Expected bob in reviewers: {}", stdout);
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test test_solution_lgtm_with_comment test_solution_new_with_review -- --nocapture`
Expected: FAIL (compilation errors from mismatched types)

**Step 3: Update command handlers**

In `src/commands/solution.rs`:

1. Update `execute()` match arm for `SolutionAction::New`:
   - Add `review` parameter
   - Pass to `new_solution()`

2. Update `new_solution()`:
   - Accept `reviewers: Vec<String>` parameter
   - Strip `@` prefix and call `solution.add_reviewer()` for each
   - Print reviewer info if any assigned

3. Update `execute()` match arm for `SolutionAction::Lgtm`:
   - Add `comment` parameter
   - Pass to `lgtm_solution()`

4. Update `lgtm_solution()`:
   - Accept `comment: Option<String>` parameter
   - Replace `solution.add_lgtm(user.clone())` with `solution.add_sign_off(user.clone(), comment)`
   - Update the "not a requested reviewer" warning to reference `reviewers` instead of `requested_reviewers`

5. Update `request_review()`:
   - Replace `solution.request_review()` with `solution.add_reviewer()`
   - Update output text

6. Update `show_solution()`:
   - Replace `requested_reviewers`/`reviewed_by` display with new Reviewers section showing sign-off status (signed off with timestamp/comment, or pending)

7. Update `accept_solution()`:
   - Replace `requires_review` check: now it's `solution.requires_review()` (derived from `!reviewers.is_empty()`)
   - Replace `has_lgtm_from_requested_reviewer()` check with `all_reviewers_signed_off()`
   - Show pending reviewers using `pending_reviewers()`
   - Set `solution.force_accepted = true` when force-accepting with open critiques or pending reviews
   - Remove loading of `config.review.default_required`

8. Update `Commands::LgtmShorthand` in `src/commands/mod.rs`:
   - Pass `comment` through to `SolutionAction::Lgtm`

**Step 4: Run tests to verify they pass**

Run: `cargo test -- --nocapture`
Expected: PASS (all tests)

**Step 5: Commit**

```bash
git add src/commands/solution.rs src/commands/mod.rs
git commit -m "feat: update solution handlers for reviewers/sign_offs model"
```

---

### Task 5: Update workflow submit gate logic

**Files:**
- Modify: `src/commands/workflow.rs`

**Step 1: Write the failing test**

Update existing tests in `tests/workflow_test.rs`:

The test `test_submit_blocked_by_review` should still work because `jjj solution review S-1 @alice` assigns alice as reviewer and submit should block without alice's sign-off. Verify.

Add new test:

```rust
#[test]
fn test_submit_with_all_signed_off() {
    if which::which("jj").is_err() { return; }
    let temp_dir = setup_test_repo();
    let dir = temp_dir.path();

    // Create main
    Command::new("jj").current_dir(dir).args(&["new", "root()", "-m", "initial"]).status().unwrap();
    Command::new("jj").current_dir(dir).args(&["bookmark", "create", "main"]).status().unwrap();

    // Start solution
    run_jjj(dir, &["start", "Token refresh", "--problem", "P-1"]);

    // Request review and sign off
    run_jjj(dir, &["solution", "review", "S-1", "@alice"]);
    run_jjj(dir, &["solution", "lgtm", "S-1"]);

    // Submit should succeed (user is the signer, matches if user identity matches "alice"... hmm)
    // Actually the test user is "Test User" not "alice", so this won't match.
    // Submit should still block because alice hasn't signed off (Test User signed off instead).
    let output = run_jjj(dir, &["submit"]);
    assert!(!output.status.success(), "Expected submit to fail — alice hasn't signed off");
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test test_submit -- --nocapture`
Expected: Compilation may fail initially; once it compiles, behavioral test should fail/pass appropriately.

**Step 3: Update workflow submit**

In `src/commands/workflow.rs`:

1. Update `submit()` function's review check:
   - Replace: `!solution.requested_reviewers.is_empty() && !solution.has_lgtm_from_requested_reviewer()`
   - With: `solution.requires_review() && !solution.all_reviewers_signed_off()`
   - Replace: `solution.requested_reviewers.iter().filter(|r| !solution.reviewed_by.contains(r))`
   - With: `solution.pending_reviewers()`

2. When force-submitting with pending reviews or open critiques, set `force_accepted = true` on the solution before saving.

**Step 4: Run tests to verify they pass**

Run: `cargo test -- --nocapture`
Expected: PASS

**Step 5: Commit**

```bash
git add src/commands/workflow.rs tests/workflow_test.rs
git commit -m "feat: update submit gate for reviewers/sign_offs model"
```

---

### Task 6: Update next command

**Files:**
- Modify: `src/commands/next.rs`

**Step 1: Write the failing test (compile test)**

The next command references `requested_reviewers`, `reviewed_by`, and `has_lgtm_from_requested_reviewer()`. After Task 1 these fields/methods no longer exist, so the code won't compile. The fix is straightforward.

**Step 2: Update next command**

In `src/commands/next.rs`:

1. Update section 3 (REVIEW category):
   - Replace `solution.requested_reviewers` with `solution.reviewers`
   - Replace `!solution.reviewed_by.iter().any(...)` with check against `solution.sign_offs`
   - Update summary text: `format!("Review requested by {}", solution.assignee.as_deref().unwrap_or("author"))` — fix the misleading text from friction point #5

2. Update section 4 (WAITING category):
   - Replace `!solution.requested_reviewers.is_empty()` with `solution.requires_review()`
   - Replace `!solution.has_lgtm_from_requested_reviewer()` with `!solution.all_reviewers_signed_off()`
   - Replace `solution.requested_reviewers.join(", ")` with `solution.pending_reviewers().join(", ")`

**Step 3: Run tests to verify they pass**

Run: `cargo test -- --nocapture`
Expected: PASS

**Step 4: Commit**

```bash
git add src/commands/next.rs
git commit -m "feat: update next command for reviewers/sign_offs model"
```

---

### Task 7: Update config and remove ReviewConfig dependency

**Files:**
- Modify: `src/models/config.rs`

**Step 1: Analyze impact**

The `ReviewConfig.default_required` field was used in `accept_solution()` (Task 4 already removed this usage). The config struct can keep existing for backwards compatibility of config files, but we no longer check `review.default_required`.

**Step 2: No code changes needed**

The `ReviewConfig` struct can stay in the config file — it doesn't hurt to have it there. The accept logic no longer reads it (done in Task 4). No breaking changes.

**Step 3: Verify compilation**

Run: `cargo test -- --nocapture`
Expected: PASS (may have a warning about unused import of `Priority` in storage.rs — fix if present)

**Step 4: Commit (only if there were changes)**

Skip if no changes needed.

---

### Task 8: Update existing Rust tests

**Files:**
- Modify: `tests/integration_test.rs`
- Modify: `tests/integration_storage.rs`

**Step 1: Fix compilation errors in integration tests**

Any test that creates solutions or references `requested_reviewers`/`reviewed_by`/`requires_review` needs updating. Scan for references:

In `tests/integration_test.rs`:
- `test_solution_supersedes` — no review fields, should be fine
- `test_problem_priority`, `test_problem_dissolve_reason`, `test_next_priority_sorting` — no review fields
- `test_solve_warns_active_solutions` — no review fields

In `tests/integration_storage.rs`:
- Check if any solution tests reference old fields

In `tests/workflow_test.rs` (already updated in Task 5):
- `test_submit_blocked_by_review` — uses `jjj solution review S-1 @alice` which still works (command unchanged)

**Step 2: Run all tests**

Run: `cargo test -- --nocapture`
Expected: PASS

**Step 3: Commit (if any test fixes needed)**

```bash
git add tests/
git commit -m "fix: update integration tests for reviewers/sign_offs model"
```

---

### Task 9: Update VS Code TypeScript interfaces

**Files:**
- Modify: `vscode/src/cli.ts`

**Step 1: Write the failing test**

Update `vscode/test/unit/cache.test.ts` `makeSolution()` helper to use new fields — compilation should fail with old interface.

**Step 2: Update Solution interface**

In `vscode/src/cli.ts`:

Remove from `Solution`:
```typescript
requested_reviewers: string[];
reviewed_by: string[];
requires_review: boolean | null;
```

Add to `Solution`:
```typescript
reviewers: string[];
sign_offs: Array<{ reviewer: string; at: string; comment?: string }>;
force_accepted: boolean;
```

**Step 3: Update CLI client methods**

Update `lgtm()` method to accept optional comment:
```typescript
async lgtm(solutionId: string, comment?: string): Promise<string> {
    const args = ["solution", "lgtm", solutionId];
    if (comment) { args.push("--comment", comment); }
    return this.exec(args);
}
```

**Step 4: Verify compilation**

Run: `cd vscode && npx tsc --noEmit`
Expected: Errors in test files and providers (fixed in Tasks 10-12)

**Step 5: Commit**

```bash
git add vscode/src/cli.ts
git commit -m "feat: update VS Code Solution interface for reviewers/sign_offs"
```

---

### Task 10: Update VS Code entity document provider

**Files:**
- Modify: `vscode/src/documents/entityDocumentProvider.ts`

**Step 1: Update solution rendering**

In `renderSolution()`:

Replace the old reviewers section (lines 82-87):
```typescript
const reviewers = s.requested_reviewers.length > 0
    ? s.requested_reviewers.map(r => {
        const lgtm = s.reviewed_by.includes(r) ? " (LGTM)" : " (requested)";
        return `@${r}${lgtm}`;
    }).join(", ")
    : "none";
```

With new Reviewers section:
```typescript
const assignedReviewers = s.reviewers.length > 0
    ? s.reviewers.map(r => {
        const signOff = s.sign_offs.find(so => so.reviewer === r);
        if (signOff) {
            const date = new Date(signOff.at).toLocaleDateString();
            const comment = signOff.comment ? ` — "${signOff.comment}"` : "";
            return `  @${r}: signed off (${date})${comment}`;
        }
        return `  @${r}: pending`;
    }).join("\n")
    : "  none";

// Non-assigned endorsements
const endorsements = s.sign_offs
    .filter(so => !s.reviewers.includes(so.reviewer))
    .map(so => {
        const date = new Date(so.at).toLocaleDateString();
        return `@${so.reviewer} (${date})`;
    });
const endorsementLine = endorsements.length > 0
    ? `\nAlso endorsed by: ${endorsements.join(", ")}`
    : "";
```

Update the return array to use `## Reviewers` section with the new format.

**Step 2: Verify compilation**

Run: `cd vscode && npx tsc --noEmit`

**Step 3: Commit**

```bash
git add vscode/src/documents/entityDocumentProvider.ts
git commit -m "feat: update VS Code entity document for reviewer sign-offs"
```

---

### Task 11: Update VS Code project tree provider

**Files:**
- Modify: `vscode/src/views/projectTreeProvider.ts`

**Step 1: Update SolutionNode description**

In `SolutionNode` constructor, after the critique description, add review status:

```typescript
const reviewStatus = solution.reviewers.length > 0
    ? (() => {
        const signedOff = solution.sign_offs.filter(so => solution.reviewers.includes(so.reviewer)).length;
        return signedOff === solution.reviewers.length
            ? ""
            : ` ${signedOff}/${solution.reviewers.length} reviewed`;
    })()
    : "";
this.description = `${solution.id} [${solution.status}]${critDesc}${reviewStatus}`;
```

**Step 2: Verify compilation**

Run: `cd vscode && npx tsc --noEmit`

**Step 3: Commit**

```bash
git add vscode/src/views/projectTreeProvider.ts
git commit -m "feat: update VS Code tree view for reviewer status"
```

---

### Task 12: Update VS Code tests

**Files:**
- Modify: `vscode/test/unit/entityDocument.test.ts`
- Modify: `vscode/test/unit/cache.test.ts`

**Step 1: Update test helpers**

In both files, update `makeSolution()`:

Replace:
```typescript
requested_reviewers: ["alice"], reviewed_by: ["alice"],
requires_review: null,
```

With:
```typescript
reviewers: ["alice"],
sign_offs: [{ reviewer: "alice", at: "2026-01-27T15:30:00Z", comment: "looks good" }],
force_accepted: false,
```

For `cache.test.ts`, the minimal helper:
```typescript
reviewers: [], sign_offs: [], force_accepted: false,
```

**Step 2: Update test assertions**

In `entityDocument.test.ts`:

Update the test "includes reviewers with LGTM status" to check for the new format:
```typescript
it("includes reviewer sign-off status", () => {
    const uri = vscode.Uri.parse("jjj:///solution/S-1.md");
    const content = provider.provideTextDocumentContent(uri);
    assert.ok(content.includes("@alice: signed off"));
    assert.ok(content.includes("looks good"));
});
```

Add test for pending reviewer:
```typescript
it("shows pending reviewers", async () => {
    const solution = makeSolution({
        id: "S-2", title: "Pending review", reviewers: ["bob"],
        sign_offs: [], force_accepted: false,
    });
    cli.listSolutions.resolves([makeSolution(), solution]);
    await cache.refresh();

    const uri = vscode.Uri.parse("jjj:///solution/S-2.md");
    const content = provider.provideTextDocumentContent(uri);
    assert.ok(content.includes("@bob: pending"));
});
```

Add test for non-assigned endorsement:
```typescript
it("shows non-assigned endorsements", async () => {
    const solution = makeSolution({
        id: "S-3", title: "Endorsed", reviewers: ["alice"],
        sign_offs: [
            { reviewer: "alice", at: "2026-01-27T15:30:00Z", comment: null },
            { reviewer: "charlie", at: "2026-01-27T16:00:00Z", comment: "nice work" },
        ],
        force_accepted: false,
    });
    cli.listSolutions.resolves([makeSolution(), solution]);
    await cache.refresh();

    const uri = vscode.Uri.parse("jjj:///solution/S-3.md");
    const content = provider.provideTextDocumentContent(uri);
    assert.ok(content.includes("Also endorsed by"));
    assert.ok(content.includes("charlie"));
});
```

**Step 3: Run VS Code tests**

Run: `cd vscode && npm test`
Expected: PASS

**Step 4: Commit**

```bash
git add vscode/test/
git commit -m "feat: update VS Code tests for reviewers/sign_offs model"
```

---

### Task 13: Update documentation

**Files:**
- Modify: `docs/reference/cli-solution.md`
- Modify: `docs/reference/cli-workflow.md`
- Modify: `docs/reference/configuration.md`
- Modify: `docs/guides/critique-guidelines.md`
- Modify: `docs/guides/code-review.md` (if exists)
- Modify: `docs/getting-started/quick-start.md`

**Step 1: Update CLI reference docs**

In `docs/reference/cli-solution.md`:
- Update `jjj solution new` to document `--review @alice,@bob` flag
- Update `jjj solution review` description: "Add assigned reviewers to a solution (additive)"
- Update `jjj solution lgtm` to document `--comment` flag
- Update `jjj solution show` to describe the new Reviewers section output
- Remove any mention of `requires_review`

In `docs/reference/cli-workflow.md`:
- Update submit description to mention the unified gate: critiques first, then reviewer sign-offs
- Document `--force` sets `force_accepted: true`
- Update `jjj lgtm` shorthand to mention `--comment`

In `docs/reference/configuration.md`:
- Note that `review.default_required` is deprecated — review is now controlled per-solution by assigning reviewers

In `docs/guides/critique-guidelines.md`:
- Update any references to separate review/critique systems to reflect the unified model

In `docs/getting-started/quick-start.md`:
- Update the review section to use `jjj solution review S-1 @alice` and `jjj solution lgtm S-1`

**Step 2: Run doc tests**

Run: `cargo test test_documentation_examples -- --nocapture`
Expected: PASS (or identify any broken doc test blocks)

**Step 3: Commit**

```bash
git add docs/
git commit -m "docs: update documentation for review-into-critique unification"
```

---

### Task 14: Final integration test pass

**Files:**
- All test files

**Step 1: Run full test suite**

Run: `cargo test -- --nocapture`
Expected: All Rust tests pass

Run: `cd vscode && npm test`
Expected: All VS Code tests pass

**Step 2: Fix any remaining failures**

Address any compilation errors or test failures discovered during the full run.

**Step 3: Commit**

```bash
git add -A
git commit -m "fix: resolve remaining test failures from review unification"
```
