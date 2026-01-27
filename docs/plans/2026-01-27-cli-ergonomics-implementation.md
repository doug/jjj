# CLI Ergonomics Restructure Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Simplify the CLI by removing tags, consolidating dashboard+next into status, and replacing start with solution resume + enhanced solution new.

**Architecture:** Three independent changes: (1) Remove the tag system entirely, (2) Replace `start` with `solution resume` and enhanced `solution new` with auto-attach and interactive problem selection, (3) Merge `dashboard` and `next` into a new `status` command. Each change touches CLI definitions, command handlers, models, tests, and VS Code extension.

**Tech Stack:** Rust (clap CLI, serde), TypeScript (VS Code extension), cargo test

---

### Task 1: Remove Tags from Models

**Files:**
- Modify: `src/models/problem.rs` — remove `tags: HashSet<String>`, `add_tag()`, `remove_tag()`
- Modify: `src/models/solution.rs` — remove `tags: HashSet<String>`, `add_tag()`, `remove_tag()`
- Modify: `src/models/milestone.rs` — remove `tags: HashSet<String>` from both `Milestone` and `MilestoneFrontmatter`, remove `add_tag()`, `remove_tag()`
- Modify: `src/models/config.rs` — remove `Tag` struct, remove `tags: Vec<Tag>` from `ProjectConfig`, remove `add_tag()`, `get_tag()`, `get_tag_by_name()`, `remove_tag()`, `edit_tag()`, `next_tag_id()` methods

**Step 1: Remove tag fields and methods from problem.rs**
Remove `pub tags: HashSet<String>` from `Problem` struct, remove it from `ProblemFrontmatter`, remove it from the `new()` constructor, remove `add_tag()` and `remove_tag()` methods. Also remove from the `From<&Problem>` impl for `ProblemFrontmatter`.

**Step 2: Remove tag fields and methods from solution.rs**
Same pattern: remove `tags: HashSet<String>` from `Solution` and `SolutionFrontmatter`, remove `add_tag()`/`remove_tag()`, remove from constructors and From impls.

**Step 3: Remove tag fields and methods from milestone.rs**
Remove `tags: HashSet<String>` from `Milestone` and `MilestoneFrontmatter`, remove `add_tag()`/`remove_tag()`, remove from constructors and From impls.

**Step 4: Remove Tag struct and tag methods from config.rs**
Remove `Tag` struct entirely, remove `tags: Vec<Tag>` from `ProjectConfig`, remove `add_tag()`, `get_tag()`, `get_tag_by_name()`, `remove_tag()`, `edit_tag()`, `next_tag_id()`. Keep `ProjectConfig` with remaining fields (name, default_reviewers, settings).

**Step 5: Run cargo check**
Run: `cargo check 2>&1`
Expected: Compilation errors in commands/storage referencing removed fields (fix in next tasks).

**Step 6: Commit**
```bash
git add src/models/
git commit -m "refactor: remove tag fields and methods from all models"
```

---

### Task 2: Remove Tags from CLI, Commands, Utils, and Storage

**Files:**
- Modify: `src/cli.rs` — remove `TagAction` enum, remove `Commands::Tag`, remove `--tag`/`--add-tag`/`--remove-tag` from ProblemAction, SolutionAction, MilestoneAction
- Delete: `src/commands/tag.rs`
- Modify: `src/commands/mod.rs` — remove `pub mod tag`, remove `Commands::Tag` match arm
- Modify: `src/commands/problem.rs` — remove tag parameters from `new_problem()` and `edit_problem()`
- Modify: `src/commands/solution.rs` — remove tag parameters from `new_solution()` and `edit_solution()`, remove tag display from `show_solution()`
- Modify: `src/commands/milestone.rs` — remove tag parameters from handlers
- Modify: `src/utils.rs` — remove `resolve_tag()` function
- Modify: `src/storage.rs` — remove any tag-related loading/saving if present

**Step 1: Remove TagAction and tag CLI flags from cli.rs**
Delete the entire `TagAction` enum (lines 599-645). Remove `Commands::Tag { action: TagAction }`. Remove `tag: Vec<String>` from `ProblemAction::New`, `add_tag`/`remove_tag` from `ProblemAction::Edit`. Same for `SolutionAction::New`/`Edit` and `MilestoneAction::New`/`Edit`.

**Step 2: Delete src/commands/tag.rs**

**Step 3: Update src/commands/mod.rs**
Remove `pub mod tag;` and the `Commands::Tag { action } => tag::execute(action)` match arm.

**Step 4: Update problem.rs command handler**
Remove `tags` parameter from `new_problem()` and the loop that adds tags. Remove `add_tag`/`remove_tag` handling from `edit_problem()`. Remove tag display from `show_problem()` if present.

**Step 5: Update solution.rs command handler**
Remove `tags` parameter from `new_solution()` and the loop that adds tags. Remove `add_tag`/`remove_tag` handling from `edit_solution()`. Remove tag display from `show_solution()` ("Tags: ..." line).

**Step 6: Update milestone.rs command handler**
Remove tag parameters from milestone new/edit handlers. Remove tag display if present.

**Step 7: Remove resolve_tag() from utils.rs**
Delete the `resolve_tag()` function (lines 113-131). Remove the `HashSet` import if now unused.

**Step 8: Run cargo check and fix any remaining references**
Run: `cargo check 2>&1`
Expected: Clean compile.

**Step 9: Commit**
```bash
git add -A
git commit -m "refactor: remove tag system from CLI, commands, and utils"
```

---

### Task 3: Remove Tags from Tests

**Files:**
- Modify: `tests/config_management.rs` — remove `test_add_tags_to_config`, `test_add_duplicate_tag`, update `test_config_serialization_toml` and any other tests referencing tags
- Modify: `tests/integration_storage.rs` — remove `test_solution_tags`, remove tag references from `test_project_config_roundtrip`
- Modify: `tests/integration_test.rs` — remove `--tag integration` from solution new command in `test_init_and_create_problem_solution`

**Step 1: Update config_management.rs tests**
Remove `test_add_tags_to_config` and `test_add_duplicate_tag` tests entirely. In `test_config_serialization_toml` (and `test_extensive_project_settings`), remove lines that add tags and assert on tags.

**Step 2: Update integration_storage.rs**
Remove `test_solution_tags` test entirely. In `test_project_config_roundtrip`, remove tag-related lines.

**Step 3: Update integration_test.rs**
In `test_init_and_create_problem_solution`, remove `"--tag", "integration"` from the solution new command args.

**Step 4: Run all tests**
Run: `cargo test 2>&1`
Expected: All tests pass (count will decrease since we removed tag tests).

**Step 5: Commit**
```bash
git add tests/
git commit -m "test: remove tag-related tests"
```

---

### Task 4: Remove Tags from VS Code Extension

**Files:**
- Modify: `vscode/src/cli.ts` — remove `tags: string[]` from Problem, Solution, Milestone interfaces. Remove `listTags()` method.
- Modify: `vscode/src/documents/entityDocumentProvider.ts` — remove tag display lines from `renderProblem()`, `renderSolution()`, `renderMilestone()`
- Modify: `vscode/test/unit/entityDocument.test.ts` — remove `tags` from test fixtures, remove tag-related test
- Modify: `vscode/test/unit/cache.test.ts` — remove `tags` from test fixtures

**Step 1: Update cli.ts interfaces**
Remove `tags: string[]` from `Problem` interface (line 14), `Solution` interface (line 31), `Milestone` interface (line 74). Remove the `listTags()` method (line 180).

**Step 2: Update entityDocumentProvider.ts**
Remove the three tag display lines:
- `renderProblem()`: Remove line with `Tags: ${p.tags.length > 0 ? p.tags.join(", ") : "none"}`
- `renderSolution()`: Remove similar tags line
- `renderMilestone()`: Remove similar tags line

**Step 3: Update test fixtures**
In `entityDocument.test.ts`: Remove `tags` from all test fixture objects. Remove the `"includes tags"` test case.
In `cache.test.ts`: Remove `tags` from all fixture objects.

**Step 4: Run VS Code tests**
Run: `cd vscode && npm test 2>&1`
Expected: All tests pass (count decreases by 1 for removed tag test).

**Step 5: Commit**
```bash
git add vscode/
git commit -m "refactor: remove tags from VS Code extension"
```

---

### Task 5: Add `solution resume` Command

**Files:**
- Modify: `src/cli.rs` — add `SolutionAction::Resume` variant
- Modify: `src/commands/solution.rs` — add `resume_solution()` handler
- Modify: `src/commands/workflow.rs` — extract resume logic from `start()`

**Step 1: Add Resume variant to SolutionAction in cli.rs**
```rust
/// Resume working on an existing solution
Resume {
    /// Solution ID (e.g., S-1)
    solution_id: String,
},
```

**Step 2: Add resume_solution() to solution.rs**
Move the resume logic from `workflow::start()` (lines 11-37) into a new `resume_solution(solution_id: String)` function. The logic:
1. Load the solution
2. If it has an active change, switch to it via `jj_client.edit()`
3. If no active change, create new empty change, attach it, set testing status
4. Update problem status to InProgress if still Open
5. Print status messages

**Step 3: Wire up in solution::execute() match**
Add `SolutionAction::Resume { solution_id } => resume_solution(solution_id)` to the execute match.

**Step 4: Run cargo check**
Run: `cargo check 2>&1`
Expected: Clean compile.

**Step 5: Commit**
```bash
git add src/cli.rs src/commands/solution.rs
git commit -m "feat: add solution resume command"
```

---

### Task 6: Enhance `solution new` with Auto-Attach and Interactive Problem Selection

**Files:**
- Modify: `src/cli.rs` — make `--problem` optional in `SolutionAction::New`
- Modify: `src/commands/solution.rs` — add auto-attach logic and interactive problem selection to `new_solution()`

**Step 1: Make --problem optional in cli.rs**
Change `SolutionAction::New` `problem` field from required to `Option<String>`:
```rust
/// Problem this solution addresses (optional - will prompt if not provided)
#[arg(long)]
problem: Option<String>,
```

**Step 2: Add interactive problem selection to new_solution()**
If `problem` is None:
1. Load all problems, filter to open ones
2. If no open problems, error with "No open problems found"
3. Display numbered list of open problems
4. Read selection from stdin
5. Use selected problem ID

If `problem` is Some, use it directly (existing behavior).

**Step 3: Add auto-attach logic to new_solution()**
After creating the solution:
1. Call `jj_client.new_empty_change(&title)` to create a new jj change
2. Get current change ID
3. Attach change to solution
4. Set solution status to Testing
5. Update problem status to InProgress

This mirrors the logic from `workflow::start()` for new solutions.

**Step 4: Run cargo check**
Run: `cargo check 2>&1`
Expected: Clean compile.

**Step 5: Commit**
```bash
git add src/cli.rs src/commands/solution.rs
git commit -m "feat: enhance solution new with auto-attach and interactive problem selection"
```

---

### Task 7: Remove `start` Command and Update References

**Files:**
- Modify: `src/cli.rs` — remove `Commands::Start` variant
- Modify: `src/commands/workflow.rs` — remove `start()` function
- Modify: `src/commands/mod.rs` — remove `Commands::Start` match arm
- Modify: `src/commands/next.rs` — update suggested command from `jjj start` to `jjj solution new`

**Step 1: Remove Commands::Start from cli.rs**
Delete the `Start { arg, problem }` variant.

**Step 2: Remove start() from workflow.rs**
Delete the entire `start()` function. Keep `submit()`.

**Step 3: Remove Start match arm from mod.rs**
Remove `Commands::Start { arg, problem } => workflow::start(arg, problem)`.

**Step 4: Update next.rs suggested command**
Change `format!("jjj start \"solution title\" --problem {}", problem.id)` to `format!("jjj solution new \"title\" --problem {}", problem.id)`.

**Step 5: Run cargo check**
Run: `cargo check 2>&1`
Expected: Clean compile.

**Step 6: Commit**
```bash
git add src/cli.rs src/commands/workflow.rs src/commands/mod.rs src/commands/next.rs
git commit -m "refactor: remove start command, update references to solution new"
```

---

### Task 8: Create `status` Command (Replaces dashboard + next)

**Files:**
- Create: `src/commands/status.rs` — new status command combining dashboard and next functionality
- Modify: `src/cli.rs` — add `Commands::Status` variant, remove `Commands::Dashboard` and `Commands::Next`
- Modify: `src/commands/mod.rs` — add `pub mod status`, remove `pub mod dashboard` and `pub mod next`, update match arms
- Delete: `src/commands/dashboard.rs`
- Delete: `src/commands/next.rs`

**Step 1: Create src/commands/status.rs**
Combine functionality from dashboard.rs and next.rs into a single command:

```rust
pub fn execute(all: bool, mine: bool, limit: Option<usize>, json: bool) -> Result<()>
```

Output structure:
1. **Active Solution** section — find solution attached to current jj change (from dashboard's "my solutions" concept). Show title, problem, status, reviewer state, open critiques count.
2. **Next Actions** section — the 5-category prioritized list from next.rs (BLOCKED, READY, REVIEW, WAITING, TODO). Same logic, same sorting.
3. **Summary** section — project-level counts from dashboard (open problems, testing solutions, open critiques).

JSON mode outputs all three sections as structured data.

**Step 2: Update cli.rs**
Remove `Commands::Dashboard` and `Commands::Next`. Add:
```rust
/// Show status and next actions
Status {
    /// Show all items (no limit)
    #[arg(long)]
    all: bool,

    /// Only my authored work
    #[arg(long)]
    mine: bool,

    /// Show top N items (default: 5)
    #[arg(long)]
    limit: Option<usize>,

    /// Output as JSON
    #[arg(long)]
    json: bool,
},
```

**Step 3: Update mod.rs**
Replace `pub mod dashboard` and `pub mod next` with `pub mod status`. Update match: `Commands::Status { all, mine, limit, json } => status::execute(all, mine, limit, json)`. Remove Dashboard and Next match arms.

**Step 4: Delete dashboard.rs and next.rs**

**Step 5: Run cargo check**
Run: `cargo check 2>&1`
Expected: Clean compile.

**Step 6: Commit**
```bash
git add -A
git commit -m "feat: add status command replacing dashboard and next"
```

---

### Task 9: Update Tests for CLI Restructure

**Files:**
- Modify: `tests/integration_test.rs` — update any tests referencing start/dashboard/next commands
- Modify: `tests/workflow_test.rs` — update tests for start→solution resume/new, remove dashboard/next tests if any
- Add new tests for: `solution resume`, enhanced `solution new`, `status` command

**Step 1: Update workflow_test.rs**
- `test_workflow_start_new_solution`: Change from `jjj start` to `jjj solution new` with appropriate args
- `test_workflow_start_resume_solution`: Change from `jjj start S-1` to `jjj solution resume S-1`
- Any tests using `jjj next` or `jjj dashboard`: Update to use `jjj status`

**Step 2: Update integration_test.rs**
- `test_next_priority_sorting`: Update command from `next` to `status`, update expected output format if needed

**Step 3: Add test for interactive problem selection**
Test that `solution new` without `--problem` lists open problems (may need to test the error case since interactive stdin is hard to test in integration tests).

**Step 4: Run all tests**
Run: `cargo test 2>&1`
Expected: All tests pass.

**Step 5: Commit**
```bash
git add tests/
git commit -m "test: update tests for CLI restructure"
```

---

### Task 10: Update VS Code Extension for CLI Changes

**Files:**
- Modify: `vscode/src/cli.ts` — update command invocations (start→solution new/resume, dashboard→status, next→status)
- Modify any VS Code UI that references removed commands

**Step 1: Search for start/dashboard/next references in VS Code**
Find all places in the VS Code extension that invoke these CLI commands and update them.

**Step 2: Update command invocations**
- Any call to `["start", ...]` → `["solution", "new", ...]` or `["solution", "resume", ...]`
- Any call to `["dashboard", ...]` → `["status", ...]`
- Any call to `["next", ...]` → `["status", ...]`

**Step 3: Run VS Code tests**
Run: `cd vscode && npm test 2>&1`
Expected: All tests pass.

**Step 4: Commit**
```bash
git add vscode/
git commit -m "refactor: update VS Code extension for CLI restructure"
```

---

### Task 11: Update Documentation

**Files:**
- Modify: Documentation files under `docs/` that reference removed/changed commands

**Step 1: Search docs/ for references to start, dashboard, next, tag**
Update all references to reflect the new CLI structure.

**Step 2: Update command references**
- `jjj start` → `jjj solution new` / `jjj solution resume`
- `jjj dashboard` → `jjj status`
- `jjj next` → `jjj status`
- `jjj tag *` → removed
- Remove any tag-related documentation sections

**Step 3: Commit**
```bash
git add docs/
git commit -m "docs: update documentation for CLI restructure"
```

---

### Task 12: Final Test Pass

**Step 1: Run all Rust tests**
Run: `cargo test 2>&1`
Expected: All tests pass.

**Step 2: Run VS Code tests**
Run: `cd vscode && npm test 2>&1`
Expected: All tests pass.

**Step 3: Run cargo clippy**
Run: `cargo clippy 2>&1`
Expected: No warnings.

**Step 4: Commit any final fixes**
```bash
git add -A
git commit -m "chore: final cleanup after CLI restructure"
```
