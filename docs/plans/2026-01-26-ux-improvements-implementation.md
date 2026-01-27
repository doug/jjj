# UX Improvements Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Improve jjj's UX across model enrichment (priority, dissolved reasons, solution lineage), automation (smarter submit), documentation (rewrite for Popperian model), and testable docs (CI-verified examples).

**Architecture:** Model changes first (Priority enum, dissolved_reason, supersedes fields), then workflow automation (smarter submit with auto-accept/auto-solve), then full documentation rewrite (index, quick-start, reference, guides), then a Rust integration test harness that extracts and runs markdown code blocks.

**Tech Stack:** Rust (clap, serde, chrono), mdBook markdown, Rust integration tests

**Worktree:** `/Users/dougfritz/src/jjj/.worktrees/ux-improvements` on branch `feature/ux-improvements`

---

## Phase 1: Model Changes

### Task 1: Add Priority enum and field to Problem model

**Files:**
- Modify: `src/models/problem.rs`
- Test: `src/models/problem.rs` (inline tests)

**Context:** Problem currently has no priority. We need a `Priority` enum with values that map to both names (low/medium/high/critical) and P-levels (P3/P2/P1/P0). The enum must implement `FromStr` (for CLI parsing), `Display` (for output), `Serialize`/`Deserialize` (for storage), `Ord` (for sorting in `jjj next`).

**Step 1: Write the failing test**

Add to the `#[cfg(test)]` module at the bottom of `src/models/problem.rs`:

```rust
#[test]
fn test_priority_from_str() {
    assert_eq!("P0".parse::<Priority>().unwrap(), Priority::Critical);
    assert_eq!("critical".parse::<Priority>().unwrap(), Priority::Critical);
    assert_eq!("P1".parse::<Priority>().unwrap(), Priority::High);
    assert_eq!("high".parse::<Priority>().unwrap(), Priority::High);
    assert_eq!("P2".parse::<Priority>().unwrap(), Priority::Medium);
    assert_eq!("medium".parse::<Priority>().unwrap(), Priority::Medium);
    assert_eq!("P3".parse::<Priority>().unwrap(), Priority::Low);
    assert_eq!("low".parse::<Priority>().unwrap(), Priority::Low);
}

#[test]
fn test_priority_display() {
    assert_eq!(format!("{}", Priority::Critical), "P0/critical");
    assert_eq!(format!("{}", Priority::High), "P1/high");
    assert_eq!(format!("{}", Priority::Medium), "P2/medium");
    assert_eq!(format!("{}", Priority::Low), "P3/low");
}

#[test]
fn test_priority_ordering() {
    assert!(Priority::Critical > Priority::High);
    assert!(Priority::High > Priority::Medium);
    assert!(Priority::Medium > Priority::Low);
}

#[test]
fn test_problem_priority_default() {
    let p = Problem::new("P-1".to_string(), "Test".to_string());
    assert_eq!(p.priority, Priority::Medium);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --lib problem::tests::test_priority`
Expected: FAIL — `Priority` type doesn't exist yet.

**Step 3: Write minimal implementation**

Add this above `ProblemStatus` in `src/models/problem.rs`:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, PartialOrd, Ord)]
pub enum Priority {
    Low,
    Medium,
    High,
    Critical,
}

impl std::fmt::Display for Priority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Priority::Low => write!(f, "P3/low"),
            Priority::Medium => write!(f, "P2/medium"),
            Priority::High => write!(f, "P1/high"),
            Priority::Critical => write!(f, "P0/critical"),
        }
    }
}

impl std::str::FromStr for Priority {
    type Err = String;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "p0" | "critical" => Ok(Priority::Critical),
            "p1" | "high" => Ok(Priority::High),
            "p2" | "medium" => Ok(Priority::Medium),
            "p3" | "low" => Ok(Priority::Low),
            _ => Err(format!("Invalid priority: {}. Use P0/critical, P1/high, P2/medium, or P3/low", s)),
        }
    }
}

impl Default for Priority {
    fn default() -> Self {
        Priority::Medium
    }
}
```

Add `priority` field to the `Problem` struct:

```rust
pub priority: Priority,
```

Add to `Problem::new()`:

```rust
priority: Priority::default(),
```

Add to `ProblemFrontmatter`:

```rust
pub priority: Priority,
```

Update frontmatter conversion methods to include `priority`.

**Step 4: Run test to verify it passes**

Run: `cargo test --lib problem::tests::test_priority`
Expected: PASS (4 tests)

**Step 5: Commit**

```bash
git add src/models/problem.rs
git commit -m "feat(model): add Priority enum with P0-P3 mapping to Problem"
```

---

### Task 2: Add dissolved_reason field to Problem model

**Files:**
- Modify: `src/models/problem.rs`
- Test: `src/models/problem.rs` (inline tests)

**Context:** When a problem is dissolved (based on false premises), we want to capture why. Add `dissolved_reason: Option<String>` to Problem and a method to set it.

**Step 1: Write the failing test**

```rust
#[test]
fn test_dissolved_reason() {
    let mut p = Problem::new("P-1".to_string(), "Test".to_string());
    assert_eq!(p.dissolved_reason, None);
    p.dissolve("The data was correct; our test was wrong".to_string());
    assert_eq!(p.status, ProblemStatus::Dissolved);
    assert_eq!(p.dissolved_reason.as_deref(), Some("The data was correct; our test was wrong"));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --lib problem::tests::test_dissolved_reason`
Expected: FAIL — field and method don't exist.

**Step 3: Write minimal implementation**

Add to `Problem` struct:

```rust
pub dissolved_reason: Option<String>,
```

Initialize as `None` in `Problem::new()`.

Add method:

```rust
pub fn dissolve(&mut self, reason: String) {
    self.status = ProblemStatus::Dissolved;
    self.dissolved_reason = Some(reason);
    self.updated_at = Utc::now();
}
```

Add to `ProblemFrontmatter` and conversion methods. The `dissolved_reason` field should be a `## Dissolved Reason` body section (like `description` and `context`), not frontmatter, since it could be long prose.

**Step 4: Run test to verify it passes**

Run: `cargo test --lib problem::tests::test_dissolved_reason`
Expected: PASS

**Step 5: Commit**

```bash
git add src/models/problem.rs
git commit -m "feat(model): add dissolved_reason to Problem"
```

---

### Task 3: Add supersedes field to Solution model

**Files:**
- Modify: `src/models/solution.rs`
- Test: `src/models/solution.rs` (inline tests)

**Context:** When a solution is refuted and a new one proposed, the new solution can reference what it supersedes. Add `supersedes: Option<String>` to Solution.

**Step 1: Write the failing test**

```rust
#[test]
fn test_solution_supersedes() {
    let s = Solution::new(
        "S-2".to_string(),
        "Better approach".to_string(),
        "P-1".to_string(),
    );
    assert_eq!(s.supersedes, None);

    let mut s2 = s.clone();
    s2.supersedes = Some("S-1".to_string());
    assert_eq!(s2.supersedes.as_deref(), Some("S-1"));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --lib solution::tests::test_solution_supersedes`
Expected: FAIL — field doesn't exist.

**Step 3: Write minimal implementation**

Add to `Solution` struct:

```rust
pub supersedes: Option<String>,
```

Initialize as `None` in `Solution::new()`.

Add to `SolutionFrontmatter` and conversion methods.

**Step 4: Run test to verify it passes**

Run: `cargo test --lib solution::tests::test_solution_supersedes`
Expected: PASS

**Step 5: Commit**

```bash
git add src/models/solution.rs
git commit -m "feat(model): add supersedes field to Solution for lineage tracking"
```

---

### Task 4: Add CLI flags for new model fields

**Files:**
- Modify: `src/cli.rs`

**Context:** Add `--priority` to `ProblemAction::New` and `ProblemAction::Edit`. Add `--reason` to `ProblemAction::Dissolve`. Add `--supersedes` to `SolutionAction::New`. No tests for CLI parsing — clap handles this and integration tests will cover it.

**Step 1: Modify ProblemAction::New**

Add to `ProblemAction::New`:

```rust
/// Priority (P0/critical, P1/high, P2/medium, P3/low)
#[arg(long, default_value = "medium")]
priority: String,
```

**Step 2: Modify ProblemAction::Edit**

Add to `ProblemAction::Edit`:

```rust
/// Set priority (P0/critical, P1/high, P2/medium, P3/low)
#[arg(long)]
priority: Option<String>,
```

**Step 3: Modify ProblemAction::Dissolve**

Add to `ProblemAction::Dissolve`:

```rust
/// Reason for dissolving (why the problem was based on false premises)
#[arg(long)]
reason: Option<String>,
```

**Step 4: Modify SolutionAction::New**

Add to `SolutionAction::New`:

```rust
/// Solution this supersedes (e.g., S-1)
#[arg(long)]
supersedes: Option<String>,
```

**Step 5: Verify it compiles**

Run: `cargo check`
Expected: Compiles (with warnings about unused fields — commands don't use them yet).

**Step 6: Commit**

```bash
git add src/cli.rs
git commit -m "feat(cli): add --priority, --reason, --supersedes flags"
```

---

### Task 5: Wire CLI flags to commands

**Files:**
- Modify: `src/commands/problem.rs`
- Modify: `src/commands/solution.rs`
- Modify: `src/commands/mod.rs` (if dispatch signatures change)
- Test: `tests/integration_test.rs`

**Context:** Connect the new CLI flags to the model. Priority is parsed via `FromStr`, dissolved_reason is passed to `dissolve()`, supersedes is stored on creation.

**Step 1: Write the failing integration test**

Add to `tests/integration_test.rs`:

```rust
#[test]
fn test_problem_priority() {
    let dir = setup_test_repo();

    // Create with P0 priority
    let output = run_jjj(&dir, &["problem", "new", "Critical bug", "--priority", "P0"]);
    assert!(output.status.success());

    // Verify in show output
    let output = run_jjj(&dir, &["problem", "show", "P-1", "--json"]);
    let json: serde_json::Value = serde_json::from_str(&String::from_utf8_lossy(&output.stdout)).unwrap();
    assert_eq!(json["priority"], "Critical");

    // Create with default priority
    let output = run_jjj(&dir, &["problem", "new", "Normal bug"]);
    assert!(output.status.success());
    let output = run_jjj(&dir, &["problem", "show", "P-2", "--json"]);
    let json: serde_json::Value = serde_json::from_str(&String::from_utf8_lossy(&output.stdout)).unwrap();
    assert_eq!(json["priority"], "Medium");
}

#[test]
fn test_problem_dissolve_reason() {
    let dir = setup_test_repo();

    let output = run_jjj(&dir, &["problem", "new", "Ghost bug"]);
    assert!(output.status.success());

    let output = run_jjj(&dir, &["problem", "dissolve", "P-1", "--reason", "Test data was stale"]);
    assert!(output.status.success());

    let output = run_jjj(&dir, &["problem", "show", "P-1", "--json"]);
    let json: serde_json::Value = serde_json::from_str(&String::from_utf8_lossy(&output.stdout)).unwrap();
    assert_eq!(json["status"], "dissolved");
    assert_eq!(json["dissolved_reason"], "Test data was stale");
}

#[test]
fn test_solution_supersedes() {
    let dir = setup_test_repo();

    run_jjj(&dir, &["problem", "new", "Slow queries"]);
    run_jjj(&dir, &["solution", "new", "Add index", "--problem", "P-1"]);
    run_jjj(&dir, &["solution", "refute", "S-1"]);

    let output = run_jjj(&dir, &["solution", "new", "Use connection pool", "--problem", "P-1", "--supersedes", "S-1"]);
    assert!(output.status.success());

    let output = run_jjj(&dir, &["solution", "show", "S-2", "--json"]);
    let json: serde_json::Value = serde_json::from_str(&String::from_utf8_lossy(&output.stdout)).unwrap();
    assert_eq!(json["supersedes"], "S-1");
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test test_problem_priority test_problem_dissolve_reason test_solution_supersedes`
Expected: FAIL — commands don't pass new fields yet.

**Step 3: Wire the commands**

In `src/commands/problem.rs`:

- `new_problem()`: Parse `priority` string via `Priority::from_str()`, set on problem before saving.
- `edit_problem()`: If `--priority` provided, parse and update.
- `dissolve_problem()`: Accept `reason: Option<String>` parameter, call `problem.dissolve(reason)` if provided, otherwise just set status.
- `show_problem()`: Include `priority` and `dissolved_reason` in both text and JSON output.

In `src/commands/solution.rs`:

- `new_solution()`: Accept `supersedes: Option<String>`, set on solution before saving.
- `show_solution()`: Include `supersedes` in output. If set, show "Supersedes: S-1".

Update the `execute()` dispatch in both files to pass the new CLI fields through.

Update `src/commands/mod.rs` if the dispatch match arms need new fields.

**Step 4: Run tests to verify they pass**

Run: `cargo test test_problem_priority test_problem_dissolve_reason test_solution_supersedes`
Expected: PASS (3 tests)

Also run: `cargo test`
Expected: All existing tests still pass.

**Step 5: Commit**

```bash
git add src/commands/problem.rs src/commands/solution.rs src/commands/mod.rs tests/integration_test.rs
git commit -m "feat: wire priority, dissolved_reason, supersedes to commands"
```

---

### Task 6: Priority-aware sorting in `jjj next`

**Files:**
- Modify: `src/commands/next.rs`
- Test: `tests/integration_test.rs`

**Context:** The `jjj next` command currently lists items in category order (BLOCKED > READY > REVIEW > WAITING > TODO) but within each category, items appear in arbitrary order. Sort by priority within categories: critical items first.

**Step 1: Write the failing integration test**

Add to `tests/integration_test.rs`:

```rust
#[test]
fn test_next_priority_sorting() {
    let dir = setup_test_repo();

    // Create problems with different priorities
    run_jjj(&dir, &["problem", "new", "Low priority", "--priority", "P3"]);
    run_jjj(&dir, &["problem", "new", "Critical priority", "--priority", "P0"]);
    run_jjj(&dir, &["problem", "new", "High priority", "--priority", "P1"]);

    // All should appear as TODO (no solutions)
    let output = run_jjj(&dir, &["next", "--json"]);
    let json: serde_json::Value = serde_json::from_str(&String::from_utf8_lossy(&output.stdout)).unwrap();
    let items = json["items"].as_array().unwrap();

    // Critical should be first, then high, then low
    assert_eq!(items[0]["entity_id"], "P-2"); // Critical
    assert_eq!(items[1]["entity_id"], "P-3"); // High
    assert_eq!(items[2]["entity_id"], "P-1"); // Low
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test test_next_priority_sorting`
Expected: FAIL — items not sorted by priority.

**Step 3: Implement priority sorting**

In `src/commands/next.rs`, after building all items in section 5 (TODO), add a sort step before the limit is applied. Add a `priority` field to the JSON output for each item, and sort within each category group by priority descending.

The simplest approach: add a numeric `priority_sort` field to each JSON item (Critical=3, High=2, Medium=1, Low=0), then stable-sort the items array by `(category_order, -priority_sort)` before truncating.

For BLOCKED items, use the highest critique severity as a proxy for priority. For TODO items, use the problem's priority directly.

**Step 4: Run test to verify it passes**

Run: `cargo test test_next_priority_sorting`
Expected: PASS

Run: `cargo test`
Expected: All tests pass.

**Step 5: Commit**

```bash
git add src/commands/next.rs tests/integration_test.rs
git commit -m "feat(next): sort items by priority within each category"
```

---

## Phase 2: UX Automation

### Task 7: Smarter `jjj submit` with auto-accept and auto-solve

**Files:**
- Modify: `src/commands/workflow.rs`
- Test: `tests/workflow_test.rs`

**Context:** Currently `submit` squashes and optionally accepts. It should:
1. Find the solution for the current change
2. Check critiques (all resolved?) and reviews (all LGTM'd?)
3. If both pass → auto-accept the solution
4. If the solution is the only active one for its problem, and no open sub-problems → auto-solve the problem
5. If blocked → print exactly what's blocking with remediation commands
6. `--force` bypasses all checks

**Step 1: Write the failing tests**

Add to `tests/workflow_test.rs`:

```rust
#[test]
fn test_submit_auto_accept_and_solve() {
    let dir = setup_test_repo();

    // Create problem and start solution
    run_jjj(&dir, &["problem", "new", "Fix auth"]);
    run_jjj(&dir, &["start", "Token refresh", "--problem", "P-1"]);

    // Submit should auto-accept (no critiques, no reviewers required)
    let output = run_jjj(&dir, &["submit"]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("accepted") || stdout.contains("Accepted"));

    // Problem should be auto-solved (only solution, no sub-problems)
    let output = run_jjj(&dir, &["problem", "show", "P-1", "--json"]);
    let json: serde_json::Value = serde_json::from_str(&String::from_utf8_lossy(&output.stdout)).unwrap();
    assert_eq!(json["status"], "solved");
}

#[test]
fn test_submit_blocked_by_critiques() {
    let dir = setup_test_repo();

    run_jjj(&dir, &["problem", "new", "Fix auth"]);
    run_jjj(&dir, &["start", "Token refresh", "--problem", "P-1"]);
    run_jjj(&dir, &["critique", "new", "S-1", "Not thread safe", "--severity", "high"]);

    // Submit without --force should fail
    let output = run_jjj(&dir, &["submit"]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("open critique") || stderr.contains("CQ-1"));
}

#[test]
fn test_submit_blocked_by_review() {
    let dir = setup_test_repo();

    run_jjj(&dir, &["problem", "new", "Fix auth"]);
    run_jjj(&dir, &["start", "Token refresh", "--problem", "P-1"]);
    run_jjj(&dir, &["solution", "review", "S-1", "@alice"]);

    // Submit without LGTM should fail
    let output = run_jjj(&dir, &["submit"]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("review") || stderr.contains("alice"));
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test test_submit_auto_accept test_submit_blocked_by_critiques test_submit_blocked_by_review`
Expected: FAIL — current submit doesn't auto-accept or block properly.

**Step 3: Implement smarter submit**

Rewrite `submit()` in `src/commands/workflow.rs`:

```rust
pub fn submit(force: bool) -> Result<()> {
    let jj_client = JjClient::new()?;
    let mut store = MetadataStore::new(jj_client)?;
    let change_id = store.jj_client.current_change_id()?;

    // Find solution for current change
    let solutions = store.list_solutions()?;
    let solution = solutions.iter()
        .find(|s| s.change_ids.contains(&change_id))
        .ok_or_else(|| JjjError::Other(
            format!("No solution found for current change {}", change_id)
        ))?;
    let solution_id = solution.id.clone();

    if !force {
        let mut blocked = false;

        // Check critiques
        let critiques = store.list_critiques()?;
        let open_critiques: Vec<_> = critiques.iter()
            .filter(|c| c.solution_id == solution_id && c.status == CritiqueStatus::Open)
            .collect();
        if !open_critiques.is_empty() {
            eprintln!("\n  ✗ {} open critique(s):", open_critiques.len());
            for c in &open_critiques {
                eprintln!("    {}: {} [{}]", c.id, c.title, c.severity);
                eprintln!("    → jjj critique address {}", c.id);
            }
            blocked = true;
        }

        // Check reviews
        if !solution.requested_reviewers.is_empty()
            && !solution.has_lgtm_from_requested_reviewer()
        {
            let pending: Vec<_> = solution.requested_reviewers.iter()
                .filter(|r| !solution.reviewed_by.contains(r))
                .collect();
            eprintln!("\n  ✗ Review pending from {}",
                pending.iter().map(|r| r.as_str()).collect::<Vec<_>>().join(", "));
            eprintln!("    → waiting for LGTM");
            blocked = true;
        }

        if blocked {
            eprintln!("\nCannot auto-accept. Use --force to submit without acceptance.");
            return Err(JjjError::CannotAcceptSolution(
                "Unresolved critiques or pending reviews".to_string()
            ));
        }
    }

    // Squash changes
    println!("Submitting {}: {}", solution.id, solution.title);
    store.jj_client.rebase_onto_main()?;
    store.jj_client.squash()?;

    // Auto-accept
    let mut solution = store.load_solution(&solution_id)?;
    solution.accept();
    store.save_solution(&solution)?;
    println!("  ✓ Solution {} accepted", solution_id);

    // Auto-solve problem if conditions met
    let problem_id = solution.problem_id.clone();
    let problem = store.load_problem(&problem_id)?;
    let all_solutions = store.list_solutions()?;
    let active_solutions: Vec<_> = all_solutions.iter()
        .filter(|s| s.problem_id == problem_id && s.is_active() && s.id != solution_id)
        .collect();
    let sub_problems = store.get_subproblems(&problem_id)?;
    let open_sub_problems: Vec<_> = sub_problems.iter()
        .filter(|p| p.is_open() || p.is_in_progress())
        .collect();

    if active_solutions.is_empty() && open_sub_problems.is_empty() && problem.is_open() {
        let mut problem = problem;
        problem.set_status(ProblemStatus::Solved);
        store.save_problem(&problem)?;
        println!("  ✓ Problem {} solved (only solution, no open sub-problems)", problem_id);
    }

    Ok(())
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test test_submit_auto_accept test_submit_blocked_by_critiques test_submit_blocked_by_review`
Expected: PASS (3 tests)

Run: `cargo test`
Expected: All tests pass (including existing submit tests, adjust `test_workflow_submit_force` if needed).

**Step 5: Commit**

```bash
git add src/commands/workflow.rs tests/workflow_test.rs
git commit -m "feat(workflow): smarter submit with auto-accept and auto-solve"
```

---

### Task 8: Premature solve warning

**Files:**
- Modify: `src/commands/problem.rs`
- Test: `tests/integration_test.rs`

**Context:** `jjj problem solve P-1` should warn if there are active solutions still in testing. Don't block — just warn.

**Step 1: Write the failing test**

Add to `tests/integration_test.rs`:

```rust
#[test]
fn test_solve_warns_active_solutions() {
    let dir = setup_test_repo();

    run_jjj(&dir, &["problem", "new", "Fix auth"]);
    run_jjj(&dir, &["solution", "new", "Approach A", "--problem", "P-1"]);
    run_jjj(&dir, &["solution", "test", "S-1"]);

    // Solving with active testing solution should still succeed but warn
    let output = run_jjj(&dir, &["problem", "solve", "P-1"]);
    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("testing") || stderr.contains("active"));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test test_solve_warns_active_solutions`
Expected: FAIL — no warning emitted currently.

**Step 3: Implement the warning**

In `src/commands/problem.rs`, in `solve_problem()`, before setting status to Solved:

```rust
let solutions = store.list_solutions()?;
let active: Vec<_> = solutions.iter()
    .filter(|s| s.problem_id == problem_id && s.is_active())
    .collect();
if !active.is_empty() {
    eprintln!("Warning: {} active solution(s) still in progress:", active.len());
    for s in &active {
        eprintln!("  {}: {} [{}]", s.id, s.title, s.status);
    }
    eprintln!("Proceeding with solve anyway.");
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test test_solve_warns_active_solutions`
Expected: PASS

**Step 5: Commit**

```bash
git add src/commands/problem.rs tests/integration_test.rs
git commit -m "feat(problem): warn when solving with active solutions in testing"
```

---

## Phase 3: Documentation Rewrite

### Task 9: Rewrite index.md

**Files:**
- Modify: `docs/index.md`

**Context:** The current index.md describes a Feature/Task/Bug model. Rewrite entirely for the Popperian Problem/Solution/Critique model. No tests needed — this is pure documentation. The testable docs harness (Task 14) will validate the code examples later.

**Step 1: Rewrite docs/index.md**

Replace the entire file with content that:
- Explains jjj as "distributed project management for Jujutsu, built on Popperian epistemology"
- Describes the core model: Problems (things to solve), Solutions (conjectures), Critiques (error elimination)
- Shows a quick example using `problem new`, `solution new`, `start`, `critique new`, `critique address`, `submit`
- Lists key features: Offline-first, Change ID stability, Critique-driven review, `jjj next` guided workflow
- Links to Getting Started, Reference, and Architecture sections

Mark testable code blocks with `` ```bash,test `` for future validation.

**Step 2: Verify mdBook builds**

Run: `mdbook build`
Expected: Book builds without errors.

**Step 3: Commit**

```bash
git add docs/index.md
git commit -m "docs: rewrite index.md for Popperian model"
```

---

### Task 10: Rewrite quick-start.md

**Files:**
- Modify: `docs/getting-started/quick-start.md`

**Context:** The current quick-start describes Feature/Task/Bug commands that don't exist. Rewrite as a complete walkthrough of the actual workflow. Every code block should be marked `` ```bash,test `` and produce exit code 0.

**Step 1: Rewrite docs/getting-started/quick-start.md**

Replace the entire file. Sections:

1. **Initialize** — `jjj init`
2. **Create a Problem** — `jjj problem new "Search is slow" --priority P1`
3. **Propose a Solution** — `jjj solution new "Add search index" --problem P-1`
4. **Start Working** — `jjj start S-1` (explain what this does: creates change, attaches, moves to testing)
5. **Add a Critique** — `jjj critique new S-1 "Missing error handling" --severity medium`
6. **Address the Critique** — `jjj critique address CQ-1`
7. **Request Review** — `jjj solution review S-1 @teammate` (explain this requires LGTM before submit)
8. **Submit** — `jjj submit` (explain auto-accept and auto-solve)
9. **Check What's Next** — `jjj next` (explain the priority categories)
10. **View the Board** — `jjj board`
11. **Key Concepts** — Change IDs, Shadow Graph, Critique Blocking
12. **Common Commands** — Updated table with all actual commands

**Step 2: Verify mdBook builds**

Run: `mdbook build`

**Step 3: Commit**

```bash
git add docs/getting-started/quick-start.md
git commit -m "docs: rewrite quick-start for Problem/Solution/Critique workflow"
```

---

### Task 11: Rewrite SUMMARY.md and create reference pages

**Files:**
- Modify: `docs/SUMMARY.md`
- Create: `docs/reference/cli-problem.md`
- Create: `docs/reference/cli-solution.md`
- Create: `docs/reference/cli-critique.md`
- Create: `docs/reference/cli-milestone.md`
- Create: `docs/reference/cli-workflow.md`
- Create: `docs/reference/configuration.md`
- Delete: `docs/reference/cli-task.md`, `docs/reference/cli-feature.md`, `docs/reference/cli-bug.md`, `docs/reference/cli-review.md` (if they exist)

**Context:** SUMMARY.md is the table of contents for the mdBook. It currently references Feature/Task/Bug pages. Update to reference the new pages and create stub reference docs for each command group.

**Step 1: Rewrite docs/SUMMARY.md**

```markdown
# Summary

[Introduction](index.md)

# Getting Started

- [Installation](getting-started/installation.md)
- [Quick Start](getting-started/quick-start.md)

# User Guides

- [Problem Solving](guides/problem-solving.md)
- [Critique Guidelines](guides/critique-guidelines.md)
- [Code Review Workflow](guides/code-review.md)
- [Board & Dashboard](guides/board-dashboard.md)
- [Jujutsu Integration](guides/jujutsu-integration.md)
- [VS Code Extension](guides/vscode-extension.md)

# CLI Reference

- [Problem Commands](reference/cli-problem.md)
- [Solution Commands](reference/cli-solution.md)
- [Critique Commands](reference/cli-critique.md)
- [Milestone Commands](reference/cli-milestone.md)
- [Workflow Commands](reference/cli-workflow.md)
- [Configuration](reference/configuration.md)

# Architecture

- [Design Philosophy](architecture/design-philosophy.md)
- [Storage & Metadata](architecture/storage-metadata.md)
- [Change ID Tracking](architecture/change-id-tracking.md)
```

**Step 2: Create each reference page**

Each reference page follows this format:

```markdown
# [Entity] Commands

## `jjj [entity] new`
[description, flags, example]

## `jjj [entity] list`
[description, flags, example]

## `jjj [entity] show`
...
```

Use `` ```bash,test `` for examples that should be validated. Pull flag names and descriptions directly from `src/cli.rs`.

`cli-workflow.md` covers: `start`, `submit`, `next`, `review` (shorthand), `lgtm` (shorthand), `board`, `dashboard`, `init`, `completion`.

`configuration.md` covers: `config.toml` format, `jjj.path` VS Code setting, environment variables.

**Step 3: Delete old reference pages**

Remove any files matching the old model: `cli-task.md`, `cli-feature.md`, `cli-bug.md`, `cli-review.md`.

**Step 4: Verify mdBook builds**

Run: `mdbook build`
Expected: Builds without broken links.

**Step 5: Commit**

```bash
git add docs/
git commit -m "docs: rewrite SUMMARY.md and add CLI reference pages"
```

---

### Task 12: Create guides

**Files:**
- Create: `docs/guides/problem-solving.md`
- Create: `docs/guides/critique-guidelines.md`
- Modify: `docs/guides/code-review.md`
- Modify: `docs/guides/board-dashboard.md`

**Context:** These guides teach users how to think about the workflow, not just which commands to run.

**Step 1: Write problem-solving.md**

Covers:
- When to create a problem (observed defect, feature request, performance issue, technical debt)
- Problem decomposition: parent/child hierarchy for large problems
- Priority guidelines: P0 = system down, P1 = major feature broken, P2 = normal work, P3 = nice-to-have
- When to dissolve: false premise, duplicate, environment-specific
- When to solve: accepted solution + no open sub-problems

**Step 2: Write critique-guidelines.md**

Covers:
- Severity levels with examples:
  - `critical`: Security vulnerability, data loss, crashes
  - `high`: Correctness issue, race condition, missing validation
  - `medium`: Design concern, missing tests, unclear naming
  - `low`: Style, minor optimization, documentation
- When to address (fix the code) vs dismiss (critique doesn't apply) vs validate (critique is correct, solution should be refuted)
- How to write effective critique arguments (evidence-based, specific, actionable)
- The philosophical basis: all criticism blocks progress until resolved

**Step 3: Update code-review.md**

Replace Feature/Task references with Problem/Solution/Critique. Show the complete flow: request review → LGTM → submit → auto-accept.

**Step 4: Update board-dashboard.md**

Replace with current board columns (Proposed | Testing | Accepted | Refuted) and dashboard content.

**Step 5: Verify mdBook builds**

Run: `mdbook build`

**Step 6: Commit**

```bash
git add docs/guides/
git commit -m "docs: add problem-solving and critique-guidelines guides, update existing guides"
```

---

### Task 13: Delete outdated documentation files

**Files:**
- Delete: any remaining files referencing Feature/Task/Bug model

**Context:** Clean up stale docs. Check all files in `docs/` for references to "feature", "task", "bug" commands and either update or delete them.

**Step 1: Search for outdated references**

```bash
grep -rl "jjj feature\|jjj task\|jjj bug\|FeatureProvider\|TaskProvider\|BugProvider" docs/
```

**Step 2: Delete or update each file found**

- Files that are entirely about the old model: delete
- Files with mixed content: update the outdated sections

**Step 3: Delete old reference stubs**

```bash
rm -f docs/reference/cli-task.md docs/reference/cli-feature.md docs/reference/cli-bug.md docs/reference/cli-review.md
```

**Step 4: Verify mdBook builds**

Run: `mdbook build`
Expected: No broken links.

**Step 5: Commit**

```bash
git add -A docs/
git commit -m "docs: remove outdated Feature/Task/Bug documentation"
```

---

## Phase 4: Testable Documentation

### Task 14: Create doc test harness

**Files:**
- Create: `tests/doc_test.rs`

**Context:** A Rust integration test that scans markdown files for `` ```bash,test `` blocks, creates a temp jj repo, and runs each block in order. Asserts exit code 0 for each command and checks `# expect:` lines against stdout.

**Step 1: Write the test harness**

Create `tests/doc_test.rs`:

```rust
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Extract ```bash,test code blocks from a markdown file
fn extract_test_blocks(content: &str) -> Vec<TestBlock> {
    let mut blocks = Vec::new();
    let mut in_block = false;
    let mut current_lines = Vec::new();
    let mut line_num = 0;

    for line in content.lines() {
        line_num += 1;
        if line.trim().starts_with("```bash,test") {
            in_block = true;
            current_lines.clear();
        } else if in_block && line.trim() == "```" {
            in_block = false;
            blocks.push(TestBlock {
                start_line: line_num - current_lines.len(),
                lines: current_lines.clone(),
            });
        } else if in_block {
            current_lines.push(line.to_string());
        }
    }
    blocks
}

struct TestBlock {
    start_line: usize,
    lines: Vec<String>,
}

fn jjj_binary() -> PathBuf {
    let mut path = std::env::current_exe().unwrap();
    path.pop(); // remove test binary name
    path.pop(); // remove deps/
    path.push("jjj");
    path
}

fn setup_test_repo() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();

    // Initialize jj repo
    Command::new("jj")
        .args(["git", "init"])
        .current_dir(dir.path())
        .output()
        .expect("jj must be installed");

    // Configure user
    Command::new("jj")
        .args(["config", "set", "--repo", "user.name", "Test User"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    Command::new("jj")
        .args(["config", "set", "--repo", "user.email", "test@example.com"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    // Initialize jjj
    let output = Command::new(jjj_binary())
        .arg("init")
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(output.status.success(), "jjj init failed");

    dir
}

fn run_doc_command(dir: &Path, cmd: &str) -> (bool, String, String) {
    let parts: Vec<&str> = cmd.split_whitespace().collect();
    if parts.is_empty() || parts[0] != "jjj" {
        return (true, String::new(), String::new()); // skip non-jjj commands
    }

    let output = Command::new(jjj_binary())
        .args(&parts[1..])
        .current_dir(dir)
        .output()
        .expect("failed to run jjj");

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    (output.status.success(), stdout, stderr)
}

#[test]
fn test_documentation_examples() {
    let docs_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("docs");
    let mut failures = Vec::new();

    for entry in walkdir::WalkDir::new(&docs_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|ext| ext == "md").unwrap_or(false))
    {
        let path = entry.path();
        let content = fs::read_to_string(path).unwrap();
        let blocks = extract_test_blocks(&content);

        if blocks.is_empty() {
            continue;
        }

        let rel_path = path.strip_prefix(&docs_dir).unwrap();
        let dir = setup_test_repo();

        for block in &blocks {
            for (i, line) in block.lines.iter().enumerate() {
                let trimmed = line.trim();

                // Skip comments and empty lines
                if trimmed.is_empty() || trimmed.starts_with('#') {
                    // Check if it's an expect assertion
                    if trimmed.starts_with("# expect:") {
                        // Handled below after command runs
                    }
                    continue;
                }

                let (success, stdout, stderr) = run_doc_command(dir.path(), trimmed);

                if !success {
                    failures.push(format!(
                        "{}:{} — command failed: {}\nstderr: {}",
                        rel_path.display(),
                        block.start_line + i,
                        trimmed,
                        stderr.trim(),
                    ));
                    break; // stop this block on first failure
                }

                // Check expect assertions on subsequent comment lines
                for expect_line in block.lines[i+1..].iter() {
                    let expect_trimmed = expect_line.trim();
                    if let Some(expected) = expect_trimmed.strip_prefix("# expect:") {
                        let expected = expected.trim().trim_matches('"');
                        if !stdout.contains(expected) {
                            failures.push(format!(
                                "{}:{} — expected '{}' in output of: {}\nstdout: {}",
                                rel_path.display(),
                                block.start_line + i,
                                expected,
                                trimmed,
                                stdout.trim(),
                            ));
                        }
                    } else {
                        break; // stop checking expects when we hit a non-comment line
                    }
                }
            }
        }
    }

    if !failures.is_empty() {
        panic!(
            "\n{} documentation test(s) failed:\n\n{}",
            failures.len(),
            failures.join("\n\n"),
        );
    }
}
```

**Step 2: Add walkdir dependency**

Add to `Cargo.toml` under `[dev-dependencies]`:

```toml
walkdir = "2"
```

**Step 3: Run the test**

Run: `cargo test test_documentation_examples`
Expected: PASS if the docs from Tasks 9-12 have correct `` ```bash,test `` blocks. If any fail, the output shows exactly which file:line and command failed.

**Step 4: Commit**

```bash
git add tests/doc_test.rs Cargo.toml
git commit -m "feat(tests): add documentation example test harness"
```

---

### Task 15: Add test markers to documentation code blocks

**Files:**
- Modify: All docs created/rewritten in Tasks 9-12

**Context:** Go through each doc file and ensure the `` ```bash,test `` markers are on the code blocks that should be tested. Add `# expect:` assertions where the output matters (e.g., JSON fields, status values).

**Step 1: Review and annotate each file**

For each doc file, verify:
- Commands that should run: mark with `` ```bash,test ``
- Commands that are conceptual/partial: keep as plain `` ```bash ``
- Output examples: add `# expect:` lines where useful

**Step 2: Run the doc tests**

Run: `cargo test test_documentation_examples`
Expected: All pass.

**Step 3: Fix any failures**

If a doc example fails, either fix the command in the doc or fix the test setup (e.g., some commands need a problem to exist first, so the blocks must be ordered correctly).

**Step 4: Commit**

```bash
git add docs/
git commit -m "docs: add test markers and assertions to documentation examples"
```

---

## Phase 5: VS Code Extension Updates

### Task 16: Update VS Code extension for new model fields

**Files:**
- Modify: `vscode/src/cli.ts`
- Modify: `vscode/src/documents/entityDocumentProvider.ts`
- Modify: `vscode/src/views/nextActionsProvider.ts`
- Modify: `vscode/src/views/projectTreeProvider.ts`
- Test: `vscode/test/unit/entityDocument.test.ts`

**Context:** Add `priority`, `dissolved_reason`, and `supersedes` to the TypeScript interfaces and update the views to display them.

**Step 1: Update interfaces in cli.ts**

Add to `Problem` interface:

```typescript
priority: "low" | "medium" | "high" | "critical";
dissolved_reason: string | null;
```

Add to `Solution` interface:

```typescript
supersedes: string | null;
```

**Step 2: Update entityDocumentProvider.ts**

In `renderProblem()`, add priority to the metadata line:

```typescript
`Status: ${p.status}  │  Priority: ${p.priority}  │  Milestone: ${p.milestone_id || "Backlog"}  │  Assignee: ${p.assignee || "unassigned"}`,
```

If problem is dissolved and has a reason, add a section:

```typescript
p.dissolved_reason ? `## Dissolved Reason\n\n${p.dissolved_reason}\n` : "",
```

In `renderSolution()`, if supersedes is set:

```typescript
s.supersedes ? `Supersedes: ${s.supersedes}` : "",
```

**Step 3: Update projectTreeProvider.ts**

Add priority indicator to ProblemNode description:

```typescript
this.description = `${problem.id} [${problem.status}] ${problem.priority !== "medium" ? `(${problem.priority})` : ""}`;
```

**Step 4: Update nextActionsProvider.ts**

No changes needed — the `jjj next --json` output already includes priority via the CLI changes.

**Step 5: Update tests**

Add `priority` and `dissolved_reason` to the `makeProblem` helper in `test/unit/entityDocument.test.ts`. Add `supersedes` to `makeSolution`. Add test cases for the new fields in rendering.

**Step 6: Verify TypeScript compiles and tests pass**

Run: `cd vscode && npx tsc --noEmit && npx tsc -p tsconfig.test.json && npx mocha`
Expected: Compiles clean, all tests pass.

**Step 7: Commit**

```bash
git add vscode/
git commit -m "feat(vscode): update extension for priority, dissolved_reason, supersedes fields"
```

---

## Summary

| Phase | Tasks | What it delivers |
|-------|-------|-----------------|
| 1. Model | Tasks 1-6 | Priority (P0-P3), dissolved reason, solution lineage, priority sorting in next |
| 2. UX | Tasks 7-8 | Smarter submit (auto-accept/solve, blocking feedback), premature solve warning |
| 3. Docs | Tasks 9-13 | Complete documentation rewrite for Popperian model |
| 4. Testable | Tasks 14-15 | CI-verified documentation examples |
| 5. VS Code | Task 16 | Extension displays new fields |

**Total: 16 tasks, ~16 commits**
