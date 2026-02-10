# Codebase Improvements Design

**Date:** 2026-02-09
**Approach:** Foundation First (incremental, each step enables the next)

## Overview

This design addresses the highest-impact improvements identified in a comprehensive code quality analysis. The changes are ordered so each step enables the next: error handling fixes are low-risk and immediate, CommandContext enables testability, tests enable safe TUI refactoring.

## Phase 1: Error Handling Fixes

**Goal:** Replace all unsafe `unwrap()` calls and fix silent failures to prevent crashes and surface data issues.

### Changes to `src/error.rs`

Add a new error variant for parse failures with context:

```rust
#[error("Failed to parse {field}: {value}")]
ParseError { field: String, value: String },
```

### Changes to `src/storage.rs`

**Line 76 — Strip prefix unwrap:**
```rust
// Before
current_section = line.strip_prefix("## ").unwrap().to_string();

// After
current_section = line.strip_prefix("## ")
    .ok_or_else(|| JjjError::ParseError {
        field: "section header".into(),
        value: line.to_string()
    })?
    .to_string();
```

**Line 155 — Path conversion:**
```rust
// Before
self.meta_path.to_str().unwrap()

// After
self.meta_path.to_str()
    .ok_or_else(|| JjjError::Other("Non-UTF8 path".into()))?
```

**Lines 273-296 — Silent list failures:** Change to collect failures and warn the user rather than silently skipping corrupted files:

```rust
let mut items = Vec::new();
let mut failures = Vec::new();

for entry in fs::read_dir(dir)? {
    match self.load_item(id) {
        Ok(item) => items.push(item),
        Err(e) => failures.push((id.to_string(), e)),
    }
}

if !failures.is_empty() {
    eprintln!("Warning: Failed to load {} item(s):", failures.len());
    for (id, err) in failures {
        eprintln!("  {}: {}", id, err);
    }
}
```

### Changes to `src/commands/`

- `critique.rs:330` — Check `replies.last()` before unwrapping
- `push.rs:9,12` — Handle I/O errors with `?` instead of unwrap
- `milestone.rs:43,79` — Use `ok_or_else` for datetime construction

### Changes to `src/jj.rs`

**Line 55 — Debug output:** Make conditional:

```rust
if std::env::var("JJJ_DEBUG").is_ok() {
    eprintln!("DEBUG: jj {}", args.join(" "));
}
```

---

## Phase 2: CommandContext Pattern

**Goal:** Extract repeated `JjClient` and `MetadataStore` instantiation into a single struct, eliminating 81 duplicate initialization blocks.

### New file: `src/context.rs`

```rust
use crate::jj::JjClient;
use crate::storage::MetadataStore;
use crate::error::Result;

/// Shared context for all command execution
pub struct CommandContext {
    pub store: MetadataStore,
}

impl CommandContext {
    pub fn new() -> Result<Self> {
        let jj_client = JjClient::new()?;
        let store = MetadataStore::new(jj_client)?;
        Ok(Self { store })
    }

    /// Access the underlying JJ client for working copy operations
    pub fn jj(&self) -> &JjClient {
        &self.store.jj_client
    }
}
```

### Changes to command files

Every command function currently does:
```rust
fn some_command(...) -> Result<()> {
    let jj_client = JjClient::new()?;
    let store = MetadataStore::new(jj_client)?;
    // ... use store
}
```

Changes to:
```rust
fn some_command(ctx: &CommandContext, ...) -> Result<()> {
    // ... use ctx.store directly
}
```

### Changes to `src/cli.rs`

Create `CommandContext` once at the top level, pass to subcommands:

```rust
fn main() -> Result<()> {
    let cli = Cli::parse();

    let ctx = CommandContext::new()?;

    match cli.command {
        Commands::Problem(args) => problem::run(&ctx, args),
        Commands::Solution(args) => solution::run(&ctx, args),
        // ...
    }
}
```

### Files affected

- `src/context.rs` — New file
- `src/lib.rs` — Export context module
- `src/cli.rs` — Create context once, pass to commands
- `src/commands/problem.rs` — ~12 functions updated
- `src/commands/solution.rs` — ~14 functions updated
- `src/commands/critique.rs` — ~10 functions updated
- `src/commands/milestone.rs` — ~8 functions updated
- `src/commands/status.rs` — ~4 functions updated
- `src/commands/workflow.rs` — ~3 functions updated

---

## Phase 3: Command Integration Tests

**Goal:** Add test coverage for 2000+ lines of untested command logic.

### New file: `tests/command_tests.rs`

```rust
use std::process::Command;
use tempfile::TempDir;

/// Creates an isolated jj repo with jjj initialized for testing
fn setup_test_repo() -> TempDir {
    let dir = TempDir::new().unwrap();

    Command::new("jj")
        .args(["git", "init"])
        .current_dir(dir.path())
        .output()
        .expect("jj init failed");

    Command::new("cargo")
        .args(["run", "--", "init"])
        .current_dir(dir.path())
        .output()
        .expect("jjj init failed");

    dir
}

fn run_jjj(dir: &TempDir, args: &[&str]) -> String {
    let output = Command::new("cargo")
        .args(["run", "--"])
        .args(args)
        .current_dir(dir.path())
        .output()
        .expect("command failed");

    String::from_utf8_lossy(&output.stdout).to_string()
}
```

### Test categories

**Problem commands:**
```rust
#[test]
fn test_problem_new_and_list() {
    let repo = setup_test_repo();
    run_jjj(&repo, &["problem", "new", "-t", "Test problem"]);

    let output = run_jjj(&repo, &["problem", "list"]);
    assert!(output.contains("Test problem"));
    assert!(output.contains("p1"));
}

#[test]
fn test_problem_list_with_status_filter() {
    let repo = setup_test_repo();
    run_jjj(&repo, &["problem", "new", "-t", "Open problem"]);
    run_jjj(&repo, &["problem", "new", "-t", "Closed problem"]);
    run_jjj(&repo, &["problem", "close", "p2"]);

    let output = run_jjj(&repo, &["problem", "list", "-s", "open"]);
    assert!(output.contains("Open problem"));
    assert!(!output.contains("Closed problem"));
}
```

**Solution commands:** Test new, list, accept, refute, link to problem

**Critique commands:** Test new, address, dismiss, reply chains

**Edge cases:** Empty repository, invalid IDs, filter combinations, JSON output

### Coverage targets

| Command file | Functions | Key scenarios |
|--------------|-----------|---------------|
| problem.rs | 12 | CRUD, filtering, dissolve, hierarchy |
| solution.rs | 14 | CRUD, accept/refute, link changes |
| critique.rs | 10 | CRUD, address/dismiss, replies |
| milestone.rs | 8 | CRUD, date handling |
| status.rs | 4 | Next actions calculation |

---

## Phase 4: TUI State Separation

**Goal:** Separate raw data from derived/UI state to eliminate sync bugs.

### Current problem

The `App` struct mixes 12 pieces of state with unclear ownership. When data changes, derived state (`next_actions`, `tree_items`) can get out of sync.

### New structure

Split into three focused structs:

```rust
/// Raw data from storage - single source of truth
pub struct ProjectData {
    pub milestones: Vec<Milestone>,
    pub problems: Vec<Problem>,
    pub solutions: Vec<Solution>,
    pub critiques: Vec<Critique>,
}

impl ProjectData {
    pub fn load(store: &MetadataStore) -> Result<Self> { ... }

    pub fn next_actions(&self) -> Vec<NextAction> { ... }

    pub fn build_tree(&self, expanded: &HashSet<String>) -> Vec<FlatTreeItem> { ... }
}

/// UI navigation and display state
pub struct UiState {
    pub focused_pane: FocusedPane,
    pub next_actions_index: usize,
    pub tree_index: usize,
    pub expanded_nodes: HashSet<String>,
    pub detail_scroll: u16,
    pub flash_message: Option<(String, Instant)>,
}

/// Main app combines data, UI state, and infrastructure
pub struct App {
    pub data: ProjectData,
    pub ui: UiState,
    pub should_quit: bool,
    store: MetadataStore,
}
```

### Caching with invalidation

```rust
pub struct App {
    data: ProjectData,
    ui: UiState,
    cache: RenderCache,
    dirty: bool,
}

impl App {
    fn invalidate(&mut self) {
        self.dirty = true;
    }

    fn ensure_cache(&mut self) {
        if self.dirty {
            self.cache.next_actions = self.data.next_actions();
            self.cache.tree_items = self.data.build_tree(&self.ui.expanded_nodes);
            self.dirty = false;
        }
    }
}
```

### Files affected

- `src/tui/app.rs` — Restructure into `ProjectData`, `UiState`, `App`
- `src/tui/mod.rs` — Update exports
- `src/tui/ui.rs` — Access state through new structure
- `src/tui/next_actions.rs` — Minor interface changes
- `src/tui/tree.rs` — Move tree building into `ProjectData`
- `src/tui/detail.rs` — Minor interface changes

---

## Implementation Order

1. **Phase 1: Error Handling** — Low risk, immediate value
2. **Phase 2: CommandContext** — Enables testing, removes duplication
3. **Phase 3: Tests** — Safety net for Phase 4
4. **Phase 4: TUI Refactor** — With confidence from test coverage

Each phase should be a separate commit (or PR if desired).
