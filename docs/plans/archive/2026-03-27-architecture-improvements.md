# Architecture Improvements Plan

> **For Claude:** REQUIRED SUB-SKILL: Use executing-plans to implement this plan task-by-task.

**Goal:** Address 7 architectural issues identified in code review: god files, leaky API surface, mixed concerns, fragile JSON handling, missing abstractions, error sprawl, and stale caches.

**Architecture:** Incremental refactors, each independently shippable. Ordered by dependency (traits before consumers, error consolidation before storage extraction). Each task leaves tests green.

**Tech Stack:** Rust, clap, serde, serde_json, ratatui, rusqlite, thiserror

---

## Task 1: Consolidate Error Variants

**Why first:** Other tasks (entity trait, storage extraction) will touch error handling. Clean this up before building on it.

**Files:**
- Modify: `src/error.rs`
- Modify: All files referencing `ProblemNotFound`, `SolutionNotFound`, `CritiqueNotFound`, `MilestoneNotFound`, `GhIo`, `GhCommandFailed`, `JjIo`, `JjCommandFailed`

**Changes:**

### 1a. Merge entity-specific NotFound into one variant

Replace:
```rust
#[error("Problem not found: {0}")]
ProblemNotFound(String),
#[error("Solution not found: {0}")]
SolutionNotFound(String),
#[error("Critique not found: {0}")]
CritiqueNotFound(String),
#[error("Milestone not found: {0}")]
MilestoneNotFound(String),
```

With:
```rust
#[error("{entity_type} not found: {entity_id}")]
NotFound { entity_type: &'static str, entity_id: String },
```

Update all call sites:
- `JjjError::ProblemNotFound(id)` → `JjjError::NotFound { entity_type: "Problem", entity_id: id }`
- Same for Solution, Critique, Milestone
- Keep `EntityNotFound(String)` for resolve.rs cases where entity type is unknown at compile time
- Keep `MetaBranchNotFound` as-is (it's a distinct system error, not an entity lookup)

**Step 1:** Grep for all `ProblemNotFound`, `SolutionNotFound`, `CritiqueNotFound`, `MilestoneNotFound` usage across the codebase. Record every file and line.

**Step 2:** Add the new `NotFound` variant to `error.rs`. Keep old variants temporarily.

**Step 3:** Migrate call sites file-by-file. Run `cargo check` after each file.

**Step 4:** Remove old variants from `error.rs`.

**Step 5:** Run `cargo test`. Fix any match exhaustiveness issues.

**Step 6:** Commit: `refactor: consolidate entity NotFound error variants`

### 1b. Merge subprocess command errors

Replace:
```rust
JjIo { args: String, #[source] source: std::io::Error },
JjCommandFailed { args: String, stderr: String },
GhIo { args: String, #[source] source: std::io::Error },
GhCommandFailed { args: String, stderr: String },
```

With:
```rust
#[error("{tool} I/O error (running `{args}`): {source}")]
CommandIo { tool: &'static str, args: String, #[source] source: std::io::Error },
#[error("{tool} failed (running `{args}`): {stderr}")]
CommandFailed { tool: &'static str, args: String, stderr: String },
```

**Step 7:** Grep for `JjIo`, `JjCommandFailed`, `GhIo`, `GhCommandFailed`.

**Step 8:** Add new variants, migrate call sites (`tool: "jj"` or `tool: "gh"`), remove old variants.

**Step 9:** Run `cargo test`.

**Step 10:** Commit: `refactor: consolidate subprocess error variants`

**Net result:** 44 → 38 error variants. More importantly, adding a new entity type or external tool no longer requires new error variants.

---

## Task 2: Introduce Entity Trait

**Why now:** Provides the shared abstraction needed by Task 3 (storage) and Task 5 (actions split). No downstream breakage — purely additive.

**Files:**
- Create: `src/models/entity.rs`
- Modify: `src/models/mod.rs` (re-export)
- Modify: `src/models/problem.rs`, `solution.rs`, `critique.rs`, `milestone.rs`

**Changes:**

### 2a. Define the trait

```rust
// src/models/entity.rs

/// Common interface for all jjj entities.
pub trait Entity {
    fn id(&self) -> &str;
    fn title(&self) -> &str;
    fn entity_type(&self) -> &'static str;  // "Problem", "Solution", etc.
    fn created_at(&self) -> &chrono::DateTime<chrono::Utc>;
    fn updated_at(&self) -> &chrono::DateTime<chrono::Utc>;
}
```

Keep it minimal. Do NOT include status (types differ), assignee (not all have it), or serialization (already covered by serde). The trait exists for polymorphic display, listing, and resolution — nothing more.

**Step 1:** Create `src/models/entity.rs` with the trait definition.

**Step 2:** Add `pub mod entity;` and `pub use entity::Entity;` to `src/models/mod.rs`.

**Step 3:** Implement `Entity` for `Problem`, `Solution`, `Critique`, `Milestone`. Each impl is 5 trivial accessor methods.

**Step 4:** Run `cargo test`.

**Step 5:** Commit: `feat: add Entity trait for shared model interface`

### 2b. Use the trait in resolve.rs and display.rs

**Step 6:** In `src/resolve.rs`, look for places that pattern-match on entity type to extract id/title. Replace with `impl Entity` bounds where it simplifies.

**Step 7:** In `src/display.rs`, look for repeated formatting logic per entity type. Use `&dyn Entity` for shared formatting (e.g., short_id + title).

**Step 8:** Run `cargo test`.

**Step 9:** Commit: `refactor: use Entity trait in resolve and display`

---

## Task 3: Extract Frontmatter Parsing from Storage

**Why now:** Depends on nothing. Reduces storage/mod.rs from 615 lines to ~450 by extracting ~165 lines of pure parsing logic.

**Files:**
- Create: `src/storage/frontmatter.rs`
- Modify: `src/storage/mod.rs` (move functions out, re-import)
- Modify: `src/storage/problems.rs`, `solutions.rs`, `critiques.rs`, `milestones.rs` (update imports if needed)

**Changes:**

### 3a. Extract parsing functions

Move these functions from `storage/mod.rs` to `storage/frontmatter.rs`:
- `parse_frontmatter<T>()` (lines 65–97)
- `add_frontmatter_context()` (lines 100–109)
- `to_markdown<T>()` (lines 112–115)
- `parse_body_sections()` (lines 119–146)
- `normalize_section_header()` (lines 150–171)
- `build_body()` (lines 174–181)

All are already free functions (not methods on MetadataStore), so extraction is mechanical.

**Step 1:** Create `src/storage/frontmatter.rs`. Move the 6 functions. Keep them `pub(crate)`.

**Step 2:** Add `pub(crate) mod frontmatter;` to `src/storage/mod.rs`.

**Step 3:** In `storage/mod.rs`, replace moved functions with `use frontmatter::*;` (or explicit imports).

**Step 4:** Check that `storage/problems.rs`, `solutions.rs`, `critiques.rs`, `milestones.rs` can still access these functions via `super::frontmatter::*` or adjust imports.

**Step 5:** Run `cargo test`.

**Step 6:** Commit: `refactor: extract frontmatter parsing into storage/frontmatter.rs`

---

## Task 4: Split `tui/app/actions.rs` by Entity Type

**Why now:** Largest file in the project (1,602 lines). Splitting makes each entity's TUI logic independently reviewable and testable.

**Files:**
- Create: `src/tui/app/actions/mod.rs`
- Create: `src/tui/app/actions/problem.rs` (~330 lines)
- Create: `src/tui/app/actions/solution.rs` (~200 lines)
- Create: `src/tui/app/actions/critique.rs` (~100 lines)
- Create: `src/tui/app/actions/milestone.rs` (~100 lines)
- Create: `src/tui/app/actions/ordering.rs` (~250 lines)
- Create: `src/tui/app/actions/common.rs` (~350 lines)
- Delete: `src/tui/app/actions.rs` (replaced by directory module)

**Changes:**

### 4a. Convert file to directory module

**Step 1:** Create `src/tui/app/actions/` directory.

**Step 2:** Create `src/tui/app/actions/mod.rs` — this will just contain the module declarations and re-exports so that `mod.rs` parent doesn't need changes:

```rust
mod common;
mod critique;
mod milestone;
mod ordering;
mod problem;
mod solution;
```

All submodules implement methods on `App` via `impl App` blocks (same pattern as before, just split across files).

### 4b. Split by entity/concern

Use the entity categorization from the research:

**`common.rs`** — polymorphic and general operations:
- `update_title()` (line 64)
- `start_new_item()` (line 117)
- `start_edit_title()` (line 163)
- `start_edit_tags()` (line 217)
- `update_tags()` (line 261)
- `refresh_data()` (line 809)
- `rebuild_cache()` (line 832)
- `start_delete()` (line 919)
- `batch_delete()` (line 1009)
- `delete_entity()` (line 1040)

**`problem.rs`** — problem-specific:
- `create_problem()` (line 8)
- `cycle_confidence()` (line 303)
- `dissolve_problem()` (line 848)
- `start_move_to_milestone()` (line 1077)
- `batch_move_to_milestone()` (line 1126)
- `move_problem_to_milestone()` (line 1190)

**`solution.rs`** — solution lifecycle:
- `create_solution()` (line 30)
- `approve_solution()` (line 632)
- `withdraw_solution()` (line 698)
- `submit_solution()` (line 721)

**`critique.rs`** — critique lifecycle:
- `create_critique()` (line 47)
- `address_critique()` (line 767)
- `dismiss_critique()` (line 788)

**`milestone.rs`** — milestone-specific:
- `create_milestone()` (line 1252)

**`ordering.rs`** — ranking/voting (all problem-under-milestone):
- `toggle_ordering_view()` (line 1271)
- `tier_drill_in()` (line 1279)
- `tier_drill_out()` (line 1331)
- `get_effective_ordering()` (line 1339)
- `selected_milestone_problem()` (line 1365)
- `default_ordering_for_milestone()` (line 1377)
- `move_problem_up()` (line 1392)
- `move_problem_down()` (line 1436)
- `add_vote()` (line 1478)
- `remove_vote()` (line 1541)

**Action key handlers** (`handle_action_a`, `handle_action_d`, `handle_action_s`, `handle_action_o`, `handle_action_v`, `handle_action_u`, `handle_action_shift_a`) dispatch across entity types. Place these in `common.rs` since they switch on entity type and delegate to entity-specific functions.

**Step 3:** Move functions into respective files. Each file has `use super::super::App;` (or appropriate path) and `impl App { ... }`.

**Step 4:** Delete old `src/tui/app/actions.rs`.

**Step 5:** Update `src/tui/app/mod.rs`: change `mod actions;` — Rust will automatically pick up the directory module.

**Step 6:** Run `cargo test`.

**Step 7:** Commit: `refactor: split tui/app/actions.rs into entity-specific modules`

---

## Task 5: Typed GitHub API Structs

**Why now:** Independent of other tasks. Replaces fragile `json["field"]` access with compile-time safety.

**Files:**
- Create: `src/sync/github/api_types.rs`
- Modify: `src/sync/github/mapping.rs`
- Modify: `src/sync/github/client.rs` (return typed structs instead of `serde_json::Value`)

**Changes:**

### 5a. Define API response types

```rust
// src/sync/github/api_types.rs
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct GitHubIssue {
    pub number: u64,
    pub title: String,
    pub body: Option<String>,
    pub state: String,
    pub labels: Vec<GitHubLabel>,
    pub author: Option<GitHubUser>,
    pub assignees: Vec<GitHubUser>,
}

#[derive(Debug, Deserialize)]
pub struct GitHubLabel {
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct GitHubUser {
    pub login: String,
}

#[derive(Debug, Deserialize)]
pub struct GitHubPr {
    pub number: u64,
    pub title: String,
    pub body: Option<String>,
    pub state: String,
    #[serde(rename = "headRefName")]
    pub head_ref_name: Option<String>,
    pub labels: Vec<GitHubLabel>,
    pub author: Option<GitHubUser>,
    pub reviews: Option<GitHubReviewConnection>,
}

#[derive(Debug, Deserialize)]
pub struct GitHubReviewConnection {
    pub nodes: Vec<GitHubReview>,
}

#[derive(Debug, Deserialize)]
pub struct GitHubReview {
    pub id: serde_json::Value,  // GitHub returns both string and int
    pub state: String,
    pub body: Option<String>,
    pub author: Option<GitHubUser>,
}
```

**Step 1:** Create `src/sync/github/api_types.rs` with the structs above. Derive `Deserialize` only — we never serialize these.

**Step 2:** Add `pub mod api_types;` to `src/sync/github/mod.rs`.

**Step 3:** Run `cargo check` to verify structs compile.

**Step 4:** Commit: `feat: add typed GitHub API response structs`

### 5b. Migrate mapping.rs to use typed structs

**Step 5:** In `client.rs`, find functions that return `serde_json::Value` for issues/PRs/reviews. Change return types to the new structs. Use `serde_json::from_value()` or `serde_json::from_str()` at the boundary.

**Step 6:** In `mapping.rs`, update `issue_to_problem()`, `parse_reviews()`, and PR conversion functions to accept typed structs instead of `&serde_json::Value`. Replace `json["field"].as_str()` with direct field access.

**Step 7:** Run `cargo test` (especially `github_sync_e2e_test.rs`).

**Step 8:** Commit: `refactor: use typed structs for GitHub API responses`

---

## Task 6: Restrict `lib.rs` Public Surface

**Why now:** After all other refactors are done, lock down the API. This is last because earlier tasks may temporarily need cross-module access.

**Caveat:** jjj is published to crates.io (v0.3.2). Restricting exports is a **semver-breaking change** (0.4.0). If external consumers exist, this needs a major version bump. Since the library API is incidental (jjj is primarily a binary), this is likely fine but should be a conscious decision.

**Files:**
- Modify: `src/lib.rs`

**Changes:**

Modules that should remain `pub` (used by external consumers or integration tests):
```rust
pub mod cli;        // needed by main.rs
pub mod commands;   // needed by main.rs
pub mod context;    // CommandContext used in tests
pub mod error;      // JjjError + Result
pub mod models;     // core types (already re-exported)
pub mod storage;    // MetadataStore used in tests
pub mod jj;         // JjClient used in tests
```

Modules that should become `pub(crate)`:
```rust
pub(crate) mod automation;
pub(crate) mod db;
pub(crate) mod display;
pub(crate) mod embeddings;
pub(crate) mod id;
pub(crate) mod local_config;
pub(crate) mod picker;
pub(crate) mod ranking;
pub(crate) mod resolve;
pub(crate) mod sync;
pub(crate) mod tui;
pub(crate) mod utils;
```

**Step 1:** Change visibility in `lib.rs`.

**Step 2:** Run `cargo test`. Integration tests in `tests/` are external to the crate — they can only see `pub` items. Fix any test compilation errors by either:
- Making the needed item `pub` (if tests legitimately need it)
- Moving the test to an inline `#[cfg(test)]` module (if it tests internals)

**Step 3:** Run `cargo test` until green.

**Step 4:** Commit: `refactor: restrict lib.rs public surface to core types`

---

## Task 7: SQLite Auto-Rebuild

**Why now:** Independent behavioral improvement. The SQLite DB is a cache but requires manual `jjj db rebuild` — this makes it self-healing.

**Files:**
- Modify: `src/db/mod.rs`
- Modify: `src/context.rs` (trigger rebuild on context creation)
- Modify: `src/commands/init.rs` (ensure DB exists after init)

**Changes:**

### 7a. Auto-detect staleness

The DB already has `needs_rebuild()` checking schema version and dirty flag. Extend it to also check if the jjj bookmark has advanced since last sync:

**Step 1:** Add a `last_sync_commit: Option<String>` to the SQLite `meta` table. Store the jjj bookmark's commit ID after each successful `load_from_markdown()`.

**Step 2:** In `needs_rebuild()`, compare stored commit ID against current `jj log -r 'jjj' --no-graph -T commit_id --limit 1`. If they differ, the DB is stale.

**Step 3:** Run `cargo test`.

**Step 4:** Commit: `feat: track sync commit in SQLite meta table`

### 7b. Auto-rebuild on access

**Step 5:** In `CommandContext` construction (or a lazy accessor), if `db.needs_rebuild()` returns true, run `db.load_from_markdown(&store)` automatically. Print a one-line notice: `Rebuilding search index...`

**Step 6:** This means `jjj db rebuild` is still available for manual use but rarely needed.

**Step 7:** Run `cargo test`.

**Step 8:** Commit: `feat: auto-rebuild SQLite cache when stale`

---

## Execution Order

```
Task 1 (errors)    ─┐
Task 2 (trait)      ├─ No dependencies between these three
Task 3 (frontmatter)┘
         │
         ▼
Task 4 (actions split) ── uses cleaner error types from Task 1
         │
         ▼
Task 5 (github types)  ── independent, do whenever
         │
         ▼
Task 6 (lib.rs)     ── must be last (tests need pub during refactors)
         │
Task 7 (auto-rebuild) ── independent, do whenever
```

Tasks 1, 2, 3 can run in parallel. Tasks 5 and 7 are independent of everything else. Task 6 must be last.

---

## Risk Assessment

| Task | Risk | Mitigation |
|------|------|------------|
| 1. Error consolidation | Match arms in tests break | Mechanical: grep + replace |
| 2. Entity trait | Trait could grow scope-creep | Keep minimal: 5 methods, no status |
| 3. Frontmatter extraction | Import path changes | All within `storage` module, `pub(crate)` |
| 4. Actions split | Methods reference each other across files | All still `impl App`, Rust resolves across files |
| 5. GitHub types | API field names wrong | Test against real `gh` output in e2e test |
| 6. lib.rs restriction | Breaks integration tests | Fix test imports; may need some items to stay pub |
| 7. Auto-rebuild | Slows first command after sync | One-time cost, <1s for typical projects |
