# Codebase Improvements Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Improve code quality through error handling fixes, CommandContext extraction, integration tests, and TUI state separation.

**Architecture:** Four phases executed sequentially. Each phase enables the next: error handling is low-risk, CommandContext enables testing, tests enable safe TUI refactoring.

**Tech Stack:** Rust, ratatui, clap, serde, tempfile (for tests)

**Worktree:** `/Users/dougfritz/src/jjj/.worktrees/codebase-improvements`

---

## Phase 1: Error Handling Fixes

### Task 1.1: Add ParseError variant to error.rs

**Files:**
- Modify: `src/error.rs:6-79`

**Step 1: Add the new error variant**

Add after line 54 (after `FrontmatterParse`):

```rust
#[error("Failed to parse {field}: {value}")]
ParseError { field: String, value: String },
```

**Step 2: Run tests to verify no breakage**

Run: `cargo test`
Expected: All 84 tests pass

**Step 3: Commit**

```bash
git add src/error.rs
git commit -m "feat(error): add ParseError variant with field context"
```

---

### Task 1.2: Fix unwrap in parse_body_sections

**Files:**
- Modify: `src/storage.rs:76`

**Step 1: Replace unwrap with expect (safe here due to if-guard)**

The line is inside `if line.starts_with("## ")` so strip_prefix will always succeed. Change from:

```rust
current_section = line.strip_prefix("## ").unwrap().to_string();
```

To:

```rust
current_section = line.strip_prefix("## ")
    .expect("strip_prefix failed after starts_with check")
    .to_string();
```

**Step 2: Run tests**

Run: `cargo test storage::tests`
Expected: All storage tests pass

**Step 3: Commit**

```bash
git add src/storage.rs
git commit -m "fix(storage): use expect with explanation for guarded strip_prefix"
```

---

### Task 1.3: Fix path unwrap in ensure_meta_checkout

**Files:**
- Modify: `src/storage.rs:148-161`

**Step 1: Replace unwrap with proper error handling**

Change from:

```rust
fn ensure_meta_checkout(&self) -> Result<()> {
    if !self.meta_path.exists() {
        // Create workspace for metadata
        self.jj_client.execute(&[
            "workspace",
            "add",
            self.meta_path.to_str().unwrap(),
            "-r",
            META_BOOKMARK,
        ])?;
    }
    Ok(())
}
```

To:

```rust
fn ensure_meta_checkout(&self) -> Result<()> {
    if !self.meta_path.exists() {
        let meta_path_str = self.meta_path.to_str()
            .ok_or_else(|| JjjError::PathError(self.meta_path.clone()))?;
        self.jj_client.execute(&[
            "workspace",
            "add",
            meta_path_str,
            "-r",
            META_BOOKMARK,
        ])?;
    }
    Ok(())
}
```

**Step 2: Run tests**

Run: `cargo test`
Expected: All tests pass

**Step 3: Commit**

```bash
git add src/storage.rs
git commit -m "fix(storage): handle non-UTF8 paths in ensure_meta_checkout"
```

---

### Task 1.4: Fix silent failures in list_problems

**Files:**
- Modify: `src/storage.rs:272-296`

**Step 1: Add warning for failed loads**

Change from:

```rust
pub fn list_problems(&self) -> Result<Vec<Problem>> {
    self.ensure_meta_checkout()?;

    let problems_dir = self.meta_path.join(PROBLEMS_DIR);
    if !problems_dir.exists() {
        return Ok(Vec::new());
    }

    let mut problems = Vec::new();
    for entry in fs::read_dir(problems_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) == Some("md") {
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                if let Ok(problem) = self.load_problem(stem) {
                    problems.push(problem);
                }
            }
        }
    }

    Ok(problems)
}
```

To:

```rust
pub fn list_problems(&self) -> Result<Vec<Problem>> {
    self.ensure_meta_checkout()?;

    let problems_dir = self.meta_path.join(PROBLEMS_DIR);
    if !problems_dir.exists() {
        return Ok(Vec::new());
    }

    let mut problems = Vec::new();
    let mut failures = Vec::new();

    for entry in fs::read_dir(problems_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) == Some("md") {
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                match self.load_problem(stem) {
                    Ok(problem) => problems.push(problem),
                    Err(e) => failures.push((stem.to_string(), e)),
                }
            }
        }
    }

    if !failures.is_empty() {
        eprintln!("Warning: Failed to load {} problem(s):", failures.len());
        for (id, err) in failures {
            eprintln!("  {}: {}", id, err);
        }
    }

    Ok(problems)
}
```

**Step 2: Run tests**

Run: `cargo test`
Expected: All tests pass

**Step 3: Commit**

```bash
git add src/storage.rs
git commit -m "fix(storage): warn on failed problem loads instead of silent skip"
```

---

### Task 1.5: Fix silent failures in list_solutions

**Files:**
- Modify: `src/storage.rs:423-447`

**Step 1: Apply same pattern as list_problems**

Change the `if let Ok(solution)` pattern to match/warn pattern:

```rust
pub fn list_solutions(&self) -> Result<Vec<Solution>> {
    self.ensure_meta_checkout()?;

    let solutions_dir = self.meta_path.join(SOLUTIONS_DIR);
    if !solutions_dir.exists() {
        return Ok(Vec::new());
    }

    let mut solutions = Vec::new();
    let mut failures = Vec::new();

    for entry in fs::read_dir(solutions_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) == Some("md") {
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                match self.load_solution(stem) {
                    Ok(solution) => solutions.push(solution),
                    Err(e) => failures.push((stem.to_string(), e)),
                }
            }
        }
    }

    if !failures.is_empty() {
        eprintln!("Warning: Failed to load {} solution(s):", failures.len());
        for (id, err) in failures {
            eprintln!("  {}: {}", id, err);
        }
    }

    Ok(solutions)
}
```

**Step 2: Run tests**

Run: `cargo test`
Expected: All tests pass

**Step 3: Commit**

```bash
git add src/storage.rs
git commit -m "fix(storage): warn on failed solution loads"
```

---

### Task 1.6: Fix silent failures in list_critiques

**Files:**
- Modify: `src/storage.rs:552-576`

**Step 1: Apply same pattern**

```rust
pub fn list_critiques(&self) -> Result<Vec<Critique>> {
    self.ensure_meta_checkout()?;

    let critiques_dir = self.meta_path.join(CRITIQUES_DIR);
    if !critiques_dir.exists() {
        return Ok(Vec::new());
    }

    let mut critiques = Vec::new();
    let mut failures = Vec::new();

    for entry in fs::read_dir(critiques_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) == Some("md") {
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                match self.load_critique(stem) {
                    Ok(critique) => critiques.push(critique),
                    Err(e) => failures.push((stem.to_string(), e)),
                }
            }
        }
    }

    if !failures.is_empty() {
        eprintln!("Warning: Failed to load {} critique(s):", failures.len());
        for (id, err) in failures {
            eprintln!("  {}: {}", id, err);
        }
    }

    Ok(critiques)
}
```

**Step 2: Run tests**

Run: `cargo test`
Expected: All tests pass

**Step 3: Commit**

```bash
git add src/storage.rs
git commit -m "fix(storage): warn on failed critique loads"
```

---

### Task 1.7: Fix silent failures in list_milestones

**Files:**
- Modify: `src/storage.rs:673-697`

**Step 1: Apply same pattern**

```rust
pub fn list_milestones(&self) -> Result<Vec<Milestone>> {
    self.ensure_meta_checkout()?;

    let milestones_dir = self.meta_path.join(MILESTONES_DIR);
    if !milestones_dir.exists() {
        return Ok(Vec::new());
    }

    let mut milestones = Vec::new();
    let mut failures = Vec::new();

    for entry in fs::read_dir(milestones_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) == Some("md") {
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                match self.load_milestone(stem) {
                    Ok(milestone) => milestones.push(milestone),
                    Err(e) => failures.push((stem.to_string(), e)),
                }
            }
        }
    }

    if !failures.is_empty() {
        eprintln!("Warning: Failed to load {} milestone(s):", failures.len());
        for (id, err) in failures {
            eprintln!("  {}: {}", id, err);
        }
    }

    Ok(milestones)
}
```

**Step 2: Run tests**

Run: `cargo test`
Expected: All tests pass

**Step 3: Commit**

```bash
git add src/storage.rs
git commit -m "fix(storage): warn on failed milestone loads"
```

---

### Task 1.8: Make debug output conditional in jj.rs

**Files:**
- Modify: `src/jj.rs:53-68`

**Step 1: Make debug conditional on JJJ_DEBUG env var**

Change from:

```rust
pub fn execute(&self, args: &[&str]) -> Result<String> {
    eprintln!("DEBUG: jj {}", args.join(" "));
    let output = Command::new(&self.jj_path)
```

To:

```rust
pub fn execute(&self, args: &[&str]) -> Result<String> {
    if std::env::var("JJJ_DEBUG").is_ok() {
        eprintln!("DEBUG: jj {}", args.join(" "));
    }
    let output = Command::new(&self.jj_path)
```

**Step 2: Run tests**

Run: `cargo test`
Expected: All tests pass

**Step 3: Commit**

```bash
git add src/jj.rs
git commit -m "fix(jj): make debug output conditional on JJJ_DEBUG env var"
```

---

### Task 1.9: Fix unwrap in push.rs prompt_yes_no

**Files:**
- Modify: `src/commands/push.rs:7-16`

**Step 1: Handle I/O errors gracefully**

Change from:

```rust
fn prompt_yes_no(message: &str) -> bool {
    print!("{} [Y/n] ", message);
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    let input = input.trim().to_lowercase();

    input.is_empty() || input == "y" || input == "yes"
}
```

To:

```rust
fn prompt_yes_no(message: &str) -> bool {
    print!("{} [Y/n] ", message);
    if io::stdout().flush().is_err() {
        return false; // Default to no on I/O error
    }

    let mut input = String::new();
    if io::stdin().read_line(&mut input).is_err() {
        return false; // Default to no on I/O error
    }
    let input = input.trim().to_lowercase();

    input.is_empty() || input == "y" || input == "yes"
}
```

**Step 2: Run tests**

Run: `cargo test`
Expected: All tests pass

**Step 3: Commit**

```bash
git add src/commands/push.rs
git commit -m "fix(push): handle I/O errors in prompt_yes_no gracefully"
```

---

### Task 1.10: Fix unwrap in critique.rs reply

**Files:**
- Modify: `src/commands/critique.rs:319-335`

**Step 1: Check replies before unwrap**

Change from:

```rust
fn reply_to_critique(critique_id: String, body: String) -> Result<()> {
    let jj_client = JjClient::new()?;
    let store = MetadataStore::new(jj_client)?;

    store.with_metadata(&format!("Reply to critique {}", critique_id), || {
        let mut critique = store.load_critique(&critique_id)?;
        let author = store.jj_client.user_identity()?;

        critique.add_reply(author.clone(), body.clone());
        store.save_critique(&critique)?;

        let reply_id = &critique.replies.last().unwrap().id;
        println!("Added reply {} to critique {}", reply_id, critique_id);

        Ok(())
    })
}
```

To:

```rust
fn reply_to_critique(critique_id: String, body: String) -> Result<()> {
    let jj_client = JjClient::new()?;
    let store = MetadataStore::new(jj_client)?;

    store.with_metadata(&format!("Reply to critique {}", critique_id), || {
        let mut critique = store.load_critique(&critique_id)?;
        let author = store.jj_client.user_identity()?;

        critique.add_reply(author.clone(), body.clone());
        store.save_critique(&critique)?;

        if let Some(reply) = critique.replies.last() {
            println!("Added reply {} to critique {}", reply.id, critique_id);
        } else {
            println!("Added reply to critique {}", critique_id);
        }

        Ok(())
    })
}
```

**Step 2: Run tests**

Run: `cargo test`
Expected: All tests pass

**Step 3: Commit**

```bash
git add src/commands/critique.rs
git commit -m "fix(critique): safely access last reply without unwrap"
```

---

### Task 1.11: Fix unwrap in TUI user_identity

**Files:**
- Modify: `src/tui/app.rs:43` and `src/tui/app.rs:559`

**Step 1: Both lines already use unwrap_or_default(), verify and move on**

Check the lines - they're already safe:
- Line 43: `let user = store.jj_client.user_identity().unwrap_or_default();`
- Line 559: `let user = self.store.jj_client.user_identity().unwrap_or_default();`

These are already handled correctly. No change needed.

**Step 2: Run tests to confirm**

Run: `cargo test`
Expected: All tests pass

---

### Task 1.12: Phase 1 Complete - Run full test suite

**Step 1: Run all tests**

Run: `cargo test`
Expected: All 84 tests pass

**Step 2: Create phase commit if not already committed individually**

If tasks were committed individually, create a summary commit or tag:

```bash
git log --oneline -10  # Review recent commits
```

---

## Phase 2: CommandContext Pattern

### Task 2.1: Create context.rs module

**Files:**
- Create: `src/context.rs`

**Step 1: Create the new module**

```rust
//! Command execution context providing shared access to storage and JJ client.

use crate::error::Result;
use crate::jj::JjClient;
use crate::storage::MetadataStore;

/// Shared context for all command execution.
///
/// This struct is created once at CLI startup and passed to all commands,
/// eliminating the need for each command to instantiate JjClient and MetadataStore.
pub struct CommandContext {
    /// The metadata storage layer
    pub store: MetadataStore,
}

impl CommandContext {
    /// Create a new command context.
    ///
    /// This discovers the jj executable, finds the repository root,
    /// and initializes the metadata store.
    pub fn new() -> Result<Self> {
        let jj_client = JjClient::new()?;
        let store = MetadataStore::new(jj_client)?;
        Ok(Self { store })
    }

    /// Access the underlying JJ client for working copy operations.
    pub fn jj(&self) -> &JjClient {
        &self.store.jj_client
    }
}
```

**Step 2: Run cargo check**

Run: `cargo check`
Expected: Compiles (module not yet used)

**Step 3: Commit**

```bash
git add src/context.rs
git commit -m "feat(context): add CommandContext for shared command state"
```

---

### Task 2.2: Export context module from lib.rs

**Files:**
- Modify: `src/lib.rs`

**Step 1: Read current lib.rs to find where to add export**

**Step 2: Add the module export**

Add after other module declarations:

```rust
pub mod context;
```

**Step 3: Run cargo check**

Run: `cargo check`
Expected: Compiles successfully

**Step 4: Commit**

```bash
git add src/lib.rs
git commit -m "feat(lib): export context module"
```

---

### Task 2.3: Update problem.rs to use CommandContext

**Files:**
- Modify: `src/commands/problem.rs`

**Step 1: Add import at top of file**

Add after existing imports:

```rust
use crate::context::CommandContext;
```

**Step 2: Update execute function signature**

Change:

```rust
pub fn execute(action: ProblemAction) -> Result<()> {
```

To:

```rust
pub fn execute(ctx: &CommandContext, action: ProblemAction) -> Result<()> {
```

**Step 3: Update each match arm to pass ctx**

Change each function call to pass ctx, e.g.:

```rust
ProblemAction::New { ... } => new_problem(ctx, ...),
```

**Step 4: Update each internal function**

For each function like `new_problem`, change from:

```rust
fn new_problem(...) -> Result<()> {
    let jj_client = JjClient::new()?;
    let store = MetadataStore::new(jj_client)?;
    // use store
}
```

To:

```rust
fn new_problem(ctx: &CommandContext, ...) -> Result<()> {
    // use ctx.store directly
}
```

Remove the `JjClient::new()` and `MetadataStore::new()` lines from all functions.

**Step 5: Run cargo check**

Run: `cargo check`
Expected: Errors about cli.rs not passing ctx (expected, we'll fix next)

**Step 6: Commit (partial)**

```bash
git add src/commands/problem.rs
git commit -m "refactor(problem): use CommandContext instead of inline initialization"
```

---

### Task 2.4: Update solution.rs to use CommandContext

**Files:**
- Modify: `src/commands/solution.rs`

**Step 1-4: Same pattern as problem.rs**

- Add `use crate::context::CommandContext;`
- Update `execute` signature to take `ctx: &CommandContext`
- Update match arms to pass ctx
- Update all internal functions to use `ctx.store` instead of creating new store

**Step 5: Run cargo check**

Run: `cargo check`
Expected: Errors continue (cli.rs not updated yet)

**Step 6: Commit**

```bash
git add src/commands/solution.rs
git commit -m "refactor(solution): use CommandContext"
```

---

### Task 2.5: Update critique.rs to use CommandContext

**Files:**
- Modify: `src/commands/critique.rs`

Apply same pattern. Remove all `JjClient::new()` and `MetadataStore::new()` calls.

**Commit:**

```bash
git add src/commands/critique.rs
git commit -m "refactor(critique): use CommandContext"
```

---

### Task 2.6: Update milestone.rs to use CommandContext

**Files:**
- Modify: `src/commands/milestone.rs`

Apply same pattern.

**Commit:**

```bash
git add src/commands/milestone.rs
git commit -m "refactor(milestone): use CommandContext"
```

---

### Task 2.7: Update status.rs to use CommandContext

**Files:**
- Modify: `src/commands/status.rs`

Apply same pattern.

**Commit:**

```bash
git add src/commands/status.rs
git commit -m "refactor(status): use CommandContext"
```

---

### Task 2.8: Update workflow.rs to use CommandContext

**Files:**
- Modify: `src/commands/workflow.rs`

Apply same pattern.

**Commit:**

```bash
git add src/commands/workflow.rs
git commit -m "refactor(workflow): use CommandContext"
```

---

### Task 2.9: Update push.rs to use CommandContext

**Files:**
- Modify: `src/commands/push.rs`

Apply same pattern.

**Commit:**

```bash
git add src/commands/push.rs
git commit -m "refactor(push): use CommandContext"
```

---

### Task 2.10: Update fetch.rs to use CommandContext

**Files:**
- Modify: `src/commands/fetch.rs`

Apply same pattern.

**Commit:**

```bash
git add src/commands/fetch.rs
git commit -m "refactor(fetch): use CommandContext"
```

---

### Task 2.11: Update cli.rs to create and pass CommandContext

**Files:**
- Modify: `src/cli.rs`

**Step 1: Add import**

```rust
use crate::context::CommandContext;
```

**Step 2: Create context in run function**

Find where commands are dispatched (likely in a `run` or `main` function) and create the context before the match:

```rust
let ctx = CommandContext::new()?;
```

**Step 3: Pass context to each command**

Update match arms to pass `&ctx`:

```rust
Commands::Problem(action) => problem::execute(&ctx, action),
Commands::Solution(action) => solution::execute(&ctx, action),
// etc.
```

Note: Some commands like `init` and `ui` may need special handling since they don't need the context (init creates the store, ui has its own App).

**Step 4: Handle init command specially**

The `init` command shouldn't require an existing store. Keep its current behavior or create a separate path.

**Step 5: Run cargo check**

Run: `cargo check`
Expected: Compiles successfully

**Step 6: Run tests**

Run: `cargo test`
Expected: All tests pass

**Step 7: Commit**

```bash
git add src/cli.rs
git commit -m "refactor(cli): create CommandContext once and pass to commands"
```

---

### Task 2.12: Phase 2 Complete - Verify all tests pass

**Step 1: Run full test suite**

Run: `cargo test`
Expected: All 84 tests pass

**Step 2: Manual smoke test**

```bash
cd /tmp && mkdir test-jjj && cd test-jjj
jj git init
cargo run --manifest-path /path/to/jjj/Cargo.toml -- init
cargo run --manifest-path /path/to/jjj/Cargo.toml -- problem new -t "Test problem"
cargo run --manifest-path /path/to/jjj/Cargo.toml -- problem list
```

Expected: Commands work correctly

---

## Phase 3: Command Integration Tests

### Task 3.1: Create test helpers module

**Files:**
- Create: `tests/test_helpers.rs`

**Step 1: Create shared test utilities**

```rust
use std::process::{Command, Output};
use tempfile::TempDir;

/// Creates an isolated jj repo with jjj initialized for testing
pub fn setup_test_repo() -> TempDir {
    let dir = TempDir::new().expect("Failed to create temp dir");

    // Initialize jj repo
    let output = Command::new("jj")
        .args(["git", "init"])
        .current_dir(dir.path())
        .output()
        .expect("Failed to run jj git init");

    assert!(output.status.success(), "jj git init failed: {:?}", output);

    // Initialize jjj
    let output = Command::new(env!("CARGO_BIN_EXE_jjj"))
        .arg("init")
        .current_dir(dir.path())
        .output()
        .expect("Failed to run jjj init");

    assert!(output.status.success(), "jjj init failed: {:?}", output);

    dir
}

/// Runs a jjj command and returns the full output
pub fn run_jjj(dir: &TempDir, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_jjj"))
        .args(args)
        .current_dir(dir.path())
        .output()
        .expect("Failed to run jjj command")
}

/// Runs a jjj command and returns stdout as string
pub fn run_jjj_stdout(dir: &TempDir, args: &[&str]) -> String {
    let output = run_jjj(dir, args);
    String::from_utf8_lossy(&output.stdout).to_string()
}

/// Runs a jjj command and asserts success
pub fn run_jjj_success(dir: &TempDir, args: &[&str]) -> String {
    let output = run_jjj(dir, args);
    assert!(
        output.status.success(),
        "Command failed: jjj {}\nstdout: {}\nstderr: {}",
        args.join(" "),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).to_string()
}
```

**Step 2: Commit**

```bash
git add tests/test_helpers.rs
git commit -m "test: add test_helpers module with setup utilities"
```

---

### Task 3.2: Create problem command tests

**Files:**
- Create: `tests/command_problem_test.rs`

**Step 1: Create test file**

```rust
mod test_helpers;

use test_helpers::{setup_test_repo, run_jjj_success, run_jjj_stdout};

#[test]
fn test_problem_new_creates_problem() {
    let repo = setup_test_repo();
    let output = run_jjj_success(&repo, &["problem", "new", "-t", "Test problem"]);

    assert!(output.contains("Created problem p1"));
    assert!(output.contains("Test problem"));
}

#[test]
fn test_problem_list_shows_problems() {
    let repo = setup_test_repo();
    run_jjj_success(&repo, &["problem", "new", "-t", "First problem"]);
    run_jjj_success(&repo, &["problem", "new", "-t", "Second problem"]);

    let output = run_jjj_success(&repo, &["problem", "list"]);

    assert!(output.contains("First problem"));
    assert!(output.contains("Second problem"));
    assert!(output.contains("p1"));
    assert!(output.contains("p2"));
}

#[test]
fn test_problem_list_filter_by_status() {
    let repo = setup_test_repo();
    run_jjj_success(&repo, &["problem", "new", "-t", "Open problem"]);
    run_jjj_success(&repo, &["problem", "new", "-t", "Will close"]);

    // Create a solution and accept it to close the problem
    run_jjj_success(&repo, &["solution", "new", "-p", "p2", "-t", "Fix"]);
    run_jjj_success(&repo, &["solution", "accept", "s1"]);
    run_jjj_success(&repo, &["problem", "solve", "p2"]);

    let output = run_jjj_success(&repo, &["problem", "list", "-s", "open"]);

    assert!(output.contains("Open problem"));
    assert!(!output.contains("Will close"));
}

#[test]
fn test_problem_show_displays_details() {
    let repo = setup_test_repo();
    run_jjj_success(&repo, &["problem", "new", "-t", "Detailed problem", "-d", "Some description"]);

    let output = run_jjj_success(&repo, &["problem", "show", "p1"]);

    assert!(output.contains("Detailed problem"));
    assert!(output.contains("Some description"));
}

#[test]
fn test_problem_json_output() {
    let repo = setup_test_repo();
    run_jjj_success(&repo, &["problem", "new", "-t", "JSON test"]);

    let output = run_jjj_success(&repo, &["problem", "list", "--json"]);

    // Should be valid JSON
    let parsed: serde_json::Value = serde_json::from_str(&output)
        .expect("Output should be valid JSON");

    assert!(parsed.is_array());
}
```

**Step 2: Run the new tests**

Run: `cargo test --test command_problem_test`
Expected: All tests pass

**Step 3: Commit**

```bash
git add tests/command_problem_test.rs
git commit -m "test: add problem command integration tests"
```

---

### Task 3.3: Create solution command tests

**Files:**
- Create: `tests/command_solution_test.rs`

**Step 1: Create test file**

```rust
mod test_helpers;

use test_helpers::{setup_test_repo, run_jjj_success, run_jjj};

#[test]
fn test_solution_new_requires_problem() {
    let repo = setup_test_repo();
    run_jjj_success(&repo, &["problem", "new", "-t", "Test problem"]);

    let output = run_jjj_success(&repo, &["solution", "new", "-p", "p1", "-t", "My solution"]);

    assert!(output.contains("Created solution s1"));
}

#[test]
fn test_solution_new_fails_without_problem() {
    let repo = setup_test_repo();

    let output = run_jjj(&repo, &["solution", "new", "-p", "p999", "-t", "Bad solution"]);

    assert!(!output.status.success());
}

#[test]
fn test_solution_accept() {
    let repo = setup_test_repo();
    run_jjj_success(&repo, &["problem", "new", "-t", "Problem"]);
    run_jjj_success(&repo, &["solution", "new", "-p", "p1", "-t", "Solution"]);

    let output = run_jjj_success(&repo, &["solution", "accept", "s1"]);

    assert!(output.contains("accepted") || output.contains("Accepted"));

    // Verify status changed
    let list = run_jjj_success(&repo, &["solution", "list"]);
    assert!(list.contains("accepted") || list.contains("Accepted"));
}

#[test]
fn test_solution_refute() {
    let repo = setup_test_repo();
    run_jjj_success(&repo, &["problem", "new", "-t", "Problem"]);
    run_jjj_success(&repo, &["solution", "new", "-p", "p1", "-t", "Bad solution"]);

    let output = run_jjj_success(&repo, &["solution", "refute", "s1"]);

    assert!(output.contains("refuted") || output.contains("Refuted"));
}

#[test]
fn test_solution_list_filter_by_problem() {
    let repo = setup_test_repo();
    run_jjj_success(&repo, &["problem", "new", "-t", "Problem 1"]);
    run_jjj_success(&repo, &["problem", "new", "-t", "Problem 2"]);
    run_jjj_success(&repo, &["solution", "new", "-p", "p1", "-t", "Sol for P1"]);
    run_jjj_success(&repo, &["solution", "new", "-p", "p2", "-t", "Sol for P2"]);

    let output = run_jjj_success(&repo, &["solution", "list", "-p", "p1"]);

    assert!(output.contains("Sol for P1"));
    assert!(!output.contains("Sol for P2"));
}
```

**Step 2: Run tests**

Run: `cargo test --test command_solution_test`
Expected: All tests pass

**Step 3: Commit**

```bash
git add tests/command_solution_test.rs
git commit -m "test: add solution command integration tests"
```

---

### Task 3.4: Create critique command tests

**Files:**
- Create: `tests/command_critique_test.rs`

**Step 1: Create test file**

```rust
mod test_helpers;

use test_helpers::{setup_test_repo, run_jjj_success};

#[test]
fn test_critique_new() {
    let repo = setup_test_repo();
    run_jjj_success(&repo, &["problem", "new", "-t", "Problem"]);
    run_jjj_success(&repo, &["solution", "new", "-p", "p1", "-t", "Solution"]);

    let output = run_jjj_success(&repo, &["critique", "new", "-s", "s1", "-t", "This is flawed"]);

    assert!(output.contains("Created critique"));
}

#[test]
fn test_critique_address() {
    let repo = setup_test_repo();
    run_jjj_success(&repo, &["problem", "new", "-t", "Problem"]);
    run_jjj_success(&repo, &["solution", "new", "-p", "p1", "-t", "Solution"]);
    run_jjj_success(&repo, &["critique", "new", "-s", "s1", "-t", "Flaw"]);

    let output = run_jjj_success(&repo, &["critique", "address", "c1"]);

    assert!(output.contains("addressed"));
}

#[test]
fn test_critique_dismiss() {
    let repo = setup_test_repo();
    run_jjj_success(&repo, &["problem", "new", "-t", "Problem"]);
    run_jjj_success(&repo, &["solution", "new", "-p", "p1", "-t", "Solution"]);
    run_jjj_success(&repo, &["critique", "new", "-s", "s1", "-t", "Not a real issue"]);

    let output = run_jjj_success(&repo, &["critique", "dismiss", "c1"]);

    assert!(output.contains("dismissed"));
}

#[test]
fn test_critique_list_filter_by_solution() {
    let repo = setup_test_repo();
    run_jjj_success(&repo, &["problem", "new", "-t", "Problem"]);
    run_jjj_success(&repo, &["solution", "new", "-p", "p1", "-t", "Sol 1"]);
    run_jjj_success(&repo, &["solution", "new", "-p", "p1", "-t", "Sol 2"]);
    run_jjj_success(&repo, &["critique", "new", "-s", "s1", "-t", "Critique on S1"]);
    run_jjj_success(&repo, &["critique", "new", "-s", "s2", "-t", "Critique on S2"]);

    let output = run_jjj_success(&repo, &["critique", "list", "-s", "s1"]);

    assert!(output.contains("Critique on S1"));
    assert!(!output.contains("Critique on S2"));
}
```

**Step 2: Run tests**

Run: `cargo test --test command_critique_test`
Expected: All tests pass

**Step 3: Commit**

```bash
git add tests/command_critique_test.rs
git commit -m "test: add critique command integration tests"
```

---

### Task 3.5: Create milestone command tests

**Files:**
- Create: `tests/command_milestone_test.rs`

**Step 1: Create test file**

```rust
mod test_helpers;

use test_helpers::{setup_test_repo, run_jjj_success};

#[test]
fn test_milestone_new() {
    let repo = setup_test_repo();

    let output = run_jjj_success(&repo, &["milestone", "new", "-t", "v1.0 Release"]);

    assert!(output.contains("Created milestone"));
    assert!(output.contains("v1.0 Release"));
}

#[test]
fn test_milestone_with_date() {
    let repo = setup_test_repo();

    let output = run_jjj_success(&repo, &["milestone", "new", "-t", "Q1 Goals", "-d", "2026-03-31"]);

    assert!(output.contains("2026-03-31"));
}

#[test]
fn test_milestone_add_problem() {
    let repo = setup_test_repo();
    run_jjj_success(&repo, &["milestone", "new", "-t", "Sprint 1"]);
    run_jjj_success(&repo, &["problem", "new", "-t", "Task A"]);

    let output = run_jjj_success(&repo, &["milestone", "add-problem", "m1", "p1"]);

    assert!(output.contains("Added") || output.contains("added"));
}

#[test]
fn test_milestone_list() {
    let repo = setup_test_repo();
    run_jjj_success(&repo, &["milestone", "new", "-t", "M1"]);
    run_jjj_success(&repo, &["milestone", "new", "-t", "M2"]);

    let output = run_jjj_success(&repo, &["milestone", "list"]);

    assert!(output.contains("M1"));
    assert!(output.contains("M2"));
}
```

**Step 2: Run tests**

Run: `cargo test --test command_milestone_test`
Expected: All tests pass

**Step 3: Commit**

```bash
git add tests/command_milestone_test.rs
git commit -m "test: add milestone command integration tests"
```

---

### Task 3.6: Create status command tests

**Files:**
- Create: `tests/command_status_test.rs`

**Step 1: Create test file**

```rust
mod test_helpers;

use test_helpers::{setup_test_repo, run_jjj_success};

#[test]
fn test_status_empty_repo() {
    let repo = setup_test_repo();

    let output = run_jjj_success(&repo, &["status"]);

    // Should complete without error on empty repo
    assert!(output.contains("Next Actions") || output.is_empty() || output.contains("No"));
}

#[test]
fn test_status_shows_open_problems() {
    let repo = setup_test_repo();
    run_jjj_success(&repo, &["problem", "new", "-t", "Open task"]);

    let output = run_jjj_success(&repo, &["status"]);

    assert!(output.contains("Open task") || output.contains("p1"));
}

#[test]
fn test_status_shows_solutions_awaiting_review() {
    let repo = setup_test_repo();
    run_jjj_success(&repo, &["problem", "new", "-t", "Problem"]);
    run_jjj_success(&repo, &["solution", "new", "-p", "p1", "-t", "Needs review"]);

    let output = run_jjj_success(&repo, &["status"]);

    // Status should mention the solution or review needed
    assert!(output.contains("s1") || output.contains("Needs review") || output.contains("review"));
}
```

**Step 2: Run tests**

Run: `cargo test --test command_status_test`
Expected: All tests pass

**Step 3: Commit**

```bash
git add tests/command_status_test.rs
git commit -m "test: add status command integration tests"
```

---

### Task 3.7: Phase 3 Complete - Run all tests

**Step 1: Run full test suite**

Run: `cargo test`
Expected: All tests pass (84 original + new command tests)

**Step 2: Commit summary if needed**

---

## Phase 4: TUI State Separation

### Task 4.1: Create ProjectData struct

**Files:**
- Modify: `src/tui/app.rs`

**Step 1: Add ProjectData struct after imports**

Add after line 8:

```rust
/// Raw data from storage - single source of truth
pub struct ProjectData {
    pub milestones: Vec<Milestone>,
    pub problems: Vec<Problem>,
    pub solutions: Vec<Solution>,
    pub critiques: Vec<Critique>,
}

impl ProjectData {
    pub fn load(store: &MetadataStore) -> Result<Self> {
        Ok(Self {
            milestones: store.list_milestones()?,
            problems: store.list_problems()?,
            solutions: store.list_solutions()?,
            critiques: store.list_critiques()?,
        })
    }
}
```

**Step 2: Run cargo check**

Run: `cargo check`
Expected: Compiles (struct not yet used)

**Step 3: Commit**

```bash
git add src/tui/app.rs
git commit -m "refactor(tui): add ProjectData struct for raw data"
```

---

### Task 4.2: Create UiState struct

**Files:**
- Modify: `src/tui/app.rs`

**Step 1: Add UiState struct after ProjectData**

```rust
/// UI navigation and display state
pub struct UiState {
    pub focused_pane: FocusedPane,
    pub next_actions_index: usize,
    pub tree_index: usize,
    pub expanded_nodes: HashSet<String>,
    pub detail_scroll: u16,
    pub flash_message: Option<(String, Instant)>,
}

impl UiState {
    pub fn new() -> Self {
        let mut expanded_nodes = HashSet::new();
        expanded_nodes.insert("backlog".to_string());

        Self {
            focused_pane: FocusedPane::NextActions,
            next_actions_index: 0,
            tree_index: 0,
            expanded_nodes,
            detail_scroll: 0,
            flash_message: None,
        }
    }
}
```

**Step 2: Run cargo check**

Run: `cargo check`
Expected: Compiles

**Step 3: Commit**

```bash
git add src/tui/app.rs
git commit -m "refactor(tui): add UiState struct for navigation state"
```

---

### Task 4.3: Create RenderCache struct

**Files:**
- Modify: `src/tui/app.rs`

**Step 1: Add RenderCache struct**

```rust
/// Cached derived data for rendering
struct RenderCache {
    next_actions: Vec<super::NextAction>,
    tree_items: Vec<super::FlatTreeItem>,
    selected_detail: super::DetailContent,
}
```

**Step 2: Commit**

```bash
git add src/tui/app.rs
git commit -m "refactor(tui): add RenderCache for derived state"
```

---

### Task 4.4: Refactor App to use new structs

**Files:**
- Modify: `src/tui/app.rs`

**Step 1: Update App struct**

Change from the flat structure to:

```rust
pub struct App {
    pub should_quit: bool,
    pub data: ProjectData,
    pub ui: UiState,
    cache: RenderCache,
    dirty: bool,
    store: MetadataStore,
}
```

**Step 2: Update App::new()**

```rust
impl App {
    pub fn new() -> Result<Self> {
        let jj_client = JjClient::new()?;
        let store = MetadataStore::new(jj_client)?;
        let data = ProjectData::load(&store)?;
        let mut ui = UiState::new();

        // Expand first milestone by default
        if let Some(m) = data.milestones.first() {
            ui.expanded_nodes.insert(m.id.clone());
        }

        let user = store.jj_client.user_identity().unwrap_or_default();
        let next_actions = super::build_next_actions(&data.problems, &data.solutions, &data.critiques, &user);
        let tree_items = super::build_flat_tree(
            &data.milestones, &data.problems, &data.solutions, &data.critiques, &ui.expanded_nodes
        );

        let cache = RenderCache {
            next_actions,
            tree_items,
            selected_detail: super::DetailContent::None,
        };

        let mut app = Self {
            should_quit: false,
            data,
            ui,
            cache,
            dirty: false,
            store,
        };
        app.update_selected_detail();
        Ok(app)
    }
}
```

**Step 3: Update all field accesses**

Throughout the file, update references:
- `self.milestones` -> `self.data.milestones`
- `self.problems` -> `self.data.problems`
- `self.solutions` -> `self.data.solutions`
- `self.critiques` -> `self.data.critiques`
- `self.next_actions` -> `self.cache.next_actions`
- `self.tree_items` -> `self.cache.tree_items`
- `self.next_actions_index` -> `self.ui.next_actions_index`
- `self.tree_index` -> `self.ui.tree_index`
- `self.expanded_nodes` -> `self.ui.expanded_nodes`
- `self.detail_scroll` -> `self.ui.detail_scroll`
- `self.flash_message` -> `self.ui.flash_message`
- `self.focused_pane` -> `self.ui.focused_pane`
- `self.selected_detail` -> `self.cache.selected_detail`

**Step 4: Add invalidate method**

```rust
fn invalidate(&mut self) {
    self.dirty = true;
}
```

**Step 5: Update refresh_data to use invalidate**

```rust
fn refresh_data(&mut self) -> Result<()> {
    self.data = ProjectData::load(&self.store)?;
    self.rebuild_cache();
    Ok(())
}

fn rebuild_cache(&mut self) {
    let user = self.store.jj_client.user_identity().unwrap_or_default();
    self.cache.next_actions = super::build_next_actions(
        &self.data.problems, &self.data.solutions, &self.data.critiques, &user
    );
    self.cache.tree_items = super::build_flat_tree(
        &self.data.milestones, &self.data.problems, &self.data.solutions,
        &self.data.critiques, &self.ui.expanded_nodes
    );
    self.update_selected_detail();
    self.dirty = false;
}
```

**Step 6: Run cargo check**

Run: `cargo check`
Expected: Compiles

**Step 7: Run tests**

Run: `cargo test`
Expected: All tests pass

**Step 8: Commit**

```bash
git add src/tui/app.rs
git commit -m "refactor(tui): restructure App to use ProjectData, UiState, RenderCache"
```

---

### Task 4.5: Update ui.rs for new App structure

**Files:**
- Modify: `src/tui/ui.rs`

**Step 1: Update all references to use new paths**

Change:
- `app.next_actions` -> `app.cache.next_actions`
- `app.tree_items` -> `app.cache.tree_items`
- `app.focused_pane` -> `app.ui.focused_pane`
- `app.next_actions_index` -> `app.ui.next_actions_index`
- `app.tree_index` -> `app.ui.tree_index`
- `app.detail_scroll` -> `app.ui.detail_scroll`
- `app.flash_message` -> `app.ui.flash_message`
- `app.selected_detail` -> `app.cache.selected_detail`

**Step 2: Run cargo check**

Run: `cargo check`
Expected: Compiles

**Step 3: Commit**

```bash
git add src/tui/ui.rs
git commit -m "refactor(tui): update ui.rs for new App structure"
```

---

### Task 4.6: Phase 4 Complete - Full verification

**Step 1: Run all tests**

Run: `cargo test`
Expected: All tests pass

**Step 2: Manual TUI test**

```bash
cd /tmp && mkdir tui-test && cd tui-test
jj git init
cargo run --manifest-path /path/to/jjj/Cargo.toml -- init
cargo run --manifest-path /path/to/jjj/Cargo.toml -- problem new -t "Test"
cargo run --manifest-path /path/to/jjj/Cargo.toml -- ui
```

Expected: TUI launches and works correctly

**Step 3: Commit**

```bash
git add -A
git commit -m "refactor(tui): complete state separation"
```

---

## Final Verification

### Task F.1: Run complete test suite

Run: `cargo test`
Expected: All tests pass

### Task F.2: Run clippy

Run: `cargo clippy`
Expected: No warnings (or only pre-existing ones)

### Task F.3: Create summary commit

```bash
git log --oneline main..HEAD
```

Review all commits for the feature branch.
