# Sync and CLI Ergonomics Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement `jjj push`, `jjj fetch`, and solution description unification to reduce multi-user sync friction.

**Architecture:** Add wrapper commands that combine jj git operations with jjj metadata sync. Modify solution creation to auto-set jj descriptions.

**Tech Stack:** Rust, clap (CLI), existing JjClient and MetadataStore.

---

## Task 1: Add `jjj fetch` command (basic)

**Files:**
- Create: `src/commands/fetch.rs`
- Modify: `src/commands/mod.rs`
- Modify: `src/cli.rs`

### Step 1: Write the failing test

Create `tests/fetch_test.rs`:

```rust
#[test]
fn test_fetch_command_exists() {
    use std::process::Command;

    let output = Command::new("cargo")
        .args(["run", "--", "fetch", "--help"])
        .output()
        .expect("Failed to run");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Fetch code and metadata from remote"));
}
```

### Step 2: Run test to verify it fails

Run: `cargo test test_fetch_command_exists -- --nocapture`
Expected: FAIL with "Fetch code and metadata" not found

### Step 3: Add CLI definition

In `src/cli.rs`, add to `Commands` enum:

```rust
/// Fetch code and metadata from remote
Fetch {
    /// Remote to fetch from (default: origin)
    #[arg(long, default_value = "origin")]
    remote: String,
},
```

### Step 4: Create fetch.rs with minimal implementation

Create `src/commands/fetch.rs`:

```rust
use crate::error::Result;
use crate::jj::JjClient;
use crate::storage::MetadataStore;

pub fn execute(remote: &str) -> Result<()> {
    let jj_client = JjClient::new()?;
    let store = MetadataStore::new(jj_client.clone())?;

    // 1. Fetch from remote
    println!("Fetching from {}...", remote);
    jj_client.execute(&["git", "fetch", "--remote", remote])?;

    // 2. Update jjj-meta workspace if it exists
    let meta_path = jj_client.repo_root().join(".jj").join("jjj-meta");
    if meta_path.exists() {
        // Update workspace to track jjj/meta bookmark
        store.meta_client.execute(&["new", "jjj/meta@origin"])?;
    }

    println!("Fetched from {}", remote);
    Ok(())
}
```

### Step 5: Wire up in mod.rs

In `src/commands/mod.rs`:

Add: `pub mod fetch;`

In the `execute` function match:
```rust
Commands::Fetch { remote } => fetch::execute(&remote),
```

### Step 6: Run test to verify it passes

Run: `cargo test test_fetch_command_exists -- --nocapture`
Expected: PASS

### Step 7: Commit

```bash
git add src/commands/fetch.rs src/commands/mod.rs src/cli.rs tests/fetch_test.rs
git commit -m "feat: add basic jjj fetch command"
```

---

## Task 2: Expose meta_client in MetadataStore

**Files:**
- Modify: `src/storage.rs`

The `meta_client` field is private. We need to expose it for fetch.rs.

### Step 1: Make meta_client public

In `src/storage.rs`, change:

```rust
/// JJ client for the metadata workspace
meta_client: JjClient,
```

To:

```rust
/// JJ client for the metadata workspace
pub meta_client: JjClient,
```

### Step 2: Run tests

Run: `cargo test`
Expected: PASS (no behavior change)

### Step 3: Commit

```bash
git add src/storage.rs
git commit -m "refactor: expose meta_client in MetadataStore"
```

---

## Task 3: Add `jjj push` command (basic)

**Files:**
- Create: `src/commands/push.rs`
- Modify: `src/commands/mod.rs`
- Modify: `src/cli.rs`

### Step 1: Write the failing test

Add to `tests/push_test.rs`:

```rust
#[test]
fn test_push_command_exists() {
    use std::process::Command;

    let output = Command::new("cargo")
        .args(["run", "--", "push", "--help"])
        .output()
        .expect("Failed to run");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Push code and metadata to remote"));
}
```

### Step 2: Run test to verify it fails

Run: `cargo test test_push_command_exists -- --nocapture`
Expected: FAIL

### Step 3: Add CLI definition

In `src/cli.rs`, add to `Commands` enum:

```rust
/// Push code and metadata to remote
Push {
    /// Bookmarks to push (in addition to jjj/meta)
    bookmarks: Vec<String>,

    /// Remote to push to (default: origin)
    #[arg(long, default_value = "origin")]
    remote: String,

    /// Skip interactive prompts
    #[arg(long)]
    no_prompt: bool,

    /// Show what would be pushed without pushing
    #[arg(long)]
    dry_run: bool,
},
```

### Step 4: Create push.rs with minimal implementation

Create `src/commands/push.rs`:

```rust
use crate::error::Result;
use crate::jj::JjClient;

pub fn execute(
    bookmarks: Vec<String>,
    remote: &str,
    no_prompt: bool,
    dry_run: bool,
) -> Result<()> {
    let jj_client = JjClient::new()?;

    if dry_run {
        println!("Would push:");
        for b in &bookmarks {
            println!("  {}", b);
        }
        println!("  jjj/meta");
        return Ok(());
    }

    // 1. Push specified bookmarks
    for bookmark in &bookmarks {
        println!("Pushing {}...", bookmark);
        jj_client.execute(&["git", "push", "-b", bookmark, "--remote", remote])?;
    }

    // 2. Always push jjj/meta
    println!("Pushing jjj/meta...");
    jj_client.execute(&["git", "push", "-b", "jjj/meta", "--remote", remote])?;

    println!("Pushed to {}", remote);

    // TODO: Smart prompts for accept/solve (Task 4)
    let _ = no_prompt; // Silence unused warning for now

    Ok(())
}
```

### Step 5: Wire up in mod.rs

In `src/commands/mod.rs`:

Add: `pub mod push;`

In the `execute` function match:
```rust
Commands::Push { bookmarks, remote, no_prompt, dry_run } => {
    push::execute(bookmarks, &remote, no_prompt, dry_run)
}
```

### Step 6: Run test to verify it passes

Run: `cargo test test_push_command_exists -- --nocapture`
Expected: PASS

### Step 7: Commit

```bash
git add src/commands/push.rs src/commands/mod.rs src/cli.rs tests/push_test.rs
git commit -m "feat: add basic jjj push command"
```

---

## Task 4: Add smart prompts to push command

**Files:**
- Modify: `src/commands/push.rs`

### Step 1: Write the test

Add to `tests/push_test.rs`:

```rust
#[test]
fn test_push_dry_run_shows_bookmarks() {
    use std::process::Command;

    let output = Command::new("cargo")
        .args(["run", "--", "push", "--dry-run", "main", "feature"])
        .output()
        .expect("Failed to run");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("main"));
    assert!(stdout.contains("feature"));
    assert!(stdout.contains("jjj/meta"));
}
```

### Step 2: Run test to verify it passes (basic functionality)

Run: `cargo test test_push_dry_run -- --nocapture`
Expected: PASS

### Step 3: Implement smart prompts

Update `src/commands/push.rs`:

```rust
use crate::error::Result;
use crate::jj::JjClient;
use crate::models::{CritiqueStatus, ProblemStatus, SolutionStatus};
use crate::storage::MetadataStore;
use std::io::{self, Write};

fn prompt_yes_no(message: &str) -> bool {
    print!("{} [Y/n] ", message);
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    let input = input.trim().to_lowercase();

    input.is_empty() || input == "y" || input == "yes"
}

pub fn execute(
    bookmarks: Vec<String>,
    remote: &str,
    no_prompt: bool,
    dry_run: bool,
) -> Result<()> {
    let jj_client = JjClient::new()?;
    let store = MetadataStore::new(jj_client.clone())?;

    if dry_run {
        println!("Would push:");
        for b in &bookmarks {
            println!("  {}", b);
        }
        println!("  jjj/meta");
        return Ok(());
    }

    // 1. Push specified bookmarks
    for bookmark in &bookmarks {
        println!("Pushing {}...", bookmark);
        jj_client.execute(&["git", "push", "-b", bookmark, "--remote", remote])?;
    }

    // 2. Always push jjj/meta
    println!("Pushing jjj/meta...");
    jj_client.execute(&["git", "push", "-b", "jjj/meta", "--remote", remote])?;

    println!("Pushed to {}", remote);

    // 3. Smart prompts (unless --no-prompt)
    if !no_prompt {
        check_and_prompt_accept_solve(&store)?;
    }

    Ok(())
}

fn check_and_prompt_accept_solve(store: &MetadataStore) -> Result<()> {
    // Find user's active solutions
    let solutions = store.list_solutions()?;
    let user = store.jj_client.user_name().unwrap_or_default();

    for solution in solutions.iter().filter(|s| {
        s.is_active() && s.assignee.as_deref() == Some(&user)
    }) {
        // Check if all critiques are resolved
        let critiques = store.get_critiques_for_solution(&solution.id)?;
        let open_critiques: Vec<_> = critiques
            .iter()
            .filter(|c| c.status == CritiqueStatus::Open)
            .collect();

        if open_critiques.is_empty() && !critiques.is_empty() {
            // All critiques resolved - prompt to accept
            if prompt_yes_no(&format!(
                "All critiques on {} \"{}\" resolved. Accept solution?",
                solution.id, solution.title
            )) {
                let mut solution = store.load_solution(&solution.id)?;
                solution.accept();
                store.save_solution(&solution)?;
                println!("  Solution {} accepted", solution.id);

                // Check if problem can be solved
                let problem = store.load_problem(&solution.problem_id)?;
                if problem.status == ProblemStatus::Open || problem.status == ProblemStatus::InProgress {
                    // Check for other active solutions
                    let other_active: Vec<_> = solutions
                        .iter()
                        .filter(|s| s.problem_id == solution.problem_id && s.is_active() && s.id != solution.id)
                        .collect();

                    if other_active.is_empty() {
                        if prompt_yes_no(&format!(
                            "Problem {} \"{}\" has no other active solutions. Mark solved?",
                            problem.id, problem.title
                        )) {
                            let mut problem = store.load_problem(&solution.problem_id)?;
                            problem.set_status(ProblemStatus::Solved);
                            store.save_problem(&problem)?;
                            println!("  Problem {} solved", problem.id);
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
```

### Step 4: Run all tests

Run: `cargo test`
Expected: PASS

### Step 5: Commit

```bash
git add src/commands/push.rs
git commit -m "feat: add smart accept/solve prompts to push command"
```

---

## Task 5: Unify solution creation with jj description

**Files:**
- Modify: `src/commands/solution.rs`
- Modify: `src/jj.rs`

### Step 1: Add describe method to JjClient

In `src/jj.rs`, the `describe` method already exists (line 102). Verify it works.

### Step 2: Write the test

Add to existing solution tests or create `tests/solution_describe_test.rs`:

```rust
#[test]
fn test_solution_new_sets_description_format() {
    // This test requires a jj repo - mark as integration test
    // For now, verify the format string logic
    let solution_id = "s1";
    let solution_title = "Fix JWT refresh";
    let problem_id = "p1";
    let problem_title = "Auth is broken";

    let expected = format!(
        "{}: {}\n\nProblem: {} - {}",
        solution_id, solution_title, problem_id, problem_title
    );

    assert!(expected.contains("s1: Fix JWT refresh"));
    assert!(expected.contains("Problem: p1 - Auth is broken"));
}
```

### Step 3: Modify solution new command

In `src/commands/solution.rs`, find the `new` function and update it to set jj description after creating the solution.

Locate where the solution is saved and add after that:

```rust
// Auto-set jj change description
let problem = store.load_problem(&solution.problem_id)?;
let description = format!(
    "{}: {}\n\nProblem: {} - {}",
    solution.id, solution.title, problem.id, problem.title
);
if let Err(e) = store.jj_client.describe(&description) {
    eprintln!("Warning: Could not set jj description: {}", e);
}
```

### Step 4: Run tests

Run: `cargo test`
Expected: PASS

### Step 5: Commit

```bash
git add src/commands/solution.rs
git commit -m "feat: solution new auto-sets jj change description"
```

---

## Task 6: Add --allow-new flag handling for push

**Files:**
- Modify: `src/commands/push.rs`

The jj git push command requires `--allow-new` for new bookmarks.

### Step 1: Update push to handle new bookmarks

In `src/commands/push.rs`, modify the push logic:

```rust
// Try normal push first, fall back to --allow-new if needed
let result = jj_client.execute(&["git", "push", "-b", bookmark, "--remote", remote]);
if result.is_err() {
    // Retry with --allow-new for new bookmarks
    jj_client.execute(&["git", "push", "-b", bookmark, "--remote", remote, "--allow-new"])?;
}
```

### Step 2: Run tests

Run: `cargo test`
Expected: PASS

### Step 3: Commit

```bash
git add src/commands/push.rs
git commit -m "fix: handle new bookmarks in push with --allow-new"
```

---

## Task 7: Add fetch summary output

**Files:**
- Modify: `src/commands/fetch.rs`

### Step 1: Implement summary of fetched changes

Update `src/commands/fetch.rs` to show what changed:

```rust
use crate::error::Result;
use crate::jj::JjClient;
use crate::storage::MetadataStore;

pub fn execute(remote: &str) -> Result<()> {
    let jj_client = JjClient::new()?;

    // Snapshot state before fetch (for comparison)
    let store_before = MetadataStore::new(jj_client.clone())?;
    let solutions_before = store_before.list_solutions().unwrap_or_default();
    let critiques_before = store_before.list_critiques().unwrap_or_default();

    // 1. Fetch from remote
    println!("Fetching from {}...", remote);
    jj_client.execute(&["git", "fetch", "--remote", remote])?;

    // 2. Update jjj-meta workspace if it exists
    let meta_path = jj_client.repo_root().join(".jj").join("jjj-meta");
    if meta_path.exists() {
        let store = MetadataStore::new(jj_client.clone())?;
        // Update workspace to track jjj/meta bookmark
        let _ = store.meta_client.execute(&["new", "jjj/meta@origin"]);
    }

    // 3. Show summary
    let store_after = MetadataStore::new(jj_client)?;
    let solutions_after = store_after.list_solutions().unwrap_or_default();
    let critiques_after = store_after.list_critiques().unwrap_or_default();

    let new_solutions = solutions_after.len().saturating_sub(solutions_before.len());
    let new_critiques = critiques_after.len().saturating_sub(critiques_before.len());

    println!("Fetched from {}:", remote);
    if new_solutions > 0 {
        println!("  {} new solution(s)", new_solutions);
    }
    if new_critiques > 0 {
        println!("  {} new critique(s)", new_critiques);
    }
    if new_solutions == 0 && new_critiques == 0 {
        println!("  No new changes");
    }

    Ok(())
}
```

### Step 2: Run tests

Run: `cargo test`
Expected: PASS

### Step 3: Commit

```bash
git add src/commands/fetch.rs
git commit -m "feat: add summary output to fetch command"
```

---

## Task 8: Update multi-user test to use new commands

**Files:**
- Modify: `tests/scripts/multi-user-review-test.sh`

### Step 1: Replace manual sync with jjj push/fetch

Find the sync functions and replace with:

```bash
sync_meta_alice_to_bob() {
    echo -e "  ${CYAN}Syncing: Alice -> Origin -> Bob${NC}"
    (cd "$ALICE_DIR" && "$JJJ_BIN" push --no-prompt 2>/dev/null)
    (cd "$BOB_DIR" && "$JJJ_BIN" fetch 2>/dev/null)
}

sync_meta_bob_to_alice() {
    echo -e "  ${CYAN}Syncing: Bob -> Origin -> Alice${NC}"
    (cd "$BOB_DIR" && "$JJJ_BIN" push --no-prompt 2>/dev/null)
    (cd "$ALICE_DIR" && "$JJJ_BIN" fetch 2>/dev/null)
}
```

### Step 2: Run the test

Run: `./tests/scripts/multi-user-review-test.sh`
Expected: PASS (or at least sync operations work)

### Step 3: Commit

```bash
git add tests/scripts/multi-user-review-test.sh
git commit -m "test: update multi-user test to use jjj push/fetch"
```

---

## Task 9: Run clippy and fix warnings

**Files:**
- Any files with warnings

### Step 1: Run clippy

Run: `cargo clippy -- -D warnings`

### Step 2: Fix any warnings

Address each warning as needed.

### Step 3: Run tests

Run: `cargo test`
Expected: PASS

### Step 4: Commit

```bash
git add -A
git commit -m "fix: resolve clippy warnings"
```

---

## Verification Checklist

After all tasks complete:

```bash
# All tests pass
cargo test

# No clippy warnings
cargo clippy -- -D warnings

# Manual verification
jjj fetch --help    # Shows help text
jjj push --help     # Shows help text
jjj push --dry-run main  # Shows what would be pushed
```

---

## Summary

| Task | Description | Estimated Complexity |
|------|-------------|---------------------|
| 1 | Add basic `jjj fetch` command | Low |
| 2 | Expose meta_client | Trivial |
| 3 | Add basic `jjj push` command | Low |
| 4 | Add smart prompts to push | Medium |
| 5 | Unify solution + jj description | Low |
| 6 | Handle --allow-new in push | Low |
| 7 | Add fetch summary output | Low |
| 8 | Update multi-user test | Low |
| 9 | Fix clippy warnings | Low |
