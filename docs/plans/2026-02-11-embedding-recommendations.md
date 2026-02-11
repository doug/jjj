# Embedding Recommendations Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add "Related" sections to show commands and TUI, plus duplicate detection on entity creation.

**Architecture:** Extend existing embedding infrastructure (Phase 1) with display integration in show commands and TUI, plus pre-creation similarity checks with user prompts.

**Tech Stack:** Rust, ratatui (TUI), existing EmbeddingClient and search infrastructure

---

## Task 1: Add Related Section to Problem Show

**Files:**
- Modify: `src/commands/problem.rs`

**Step 1: Read the current show_problem function**

Read `src/commands/problem.rs` to understand the current implementation.

**Step 2: Add related section after problem display**

Find the `show` function and add related items display at the end. Add these imports at the top:

```rust
use crate::db::{search, Database};
use crate::embeddings::EmbeddingClient;
use crate::local_config::LocalConfig;
```

After displaying the problem details, add:

```rust
    // Show related items if embeddings available
    show_related_items(ctx, "problem", &problem.id)?;
```

Add a helper function:

```rust
fn show_related_items(ctx: &CommandContext, entity_type: &str, entity_id: &str) -> Result<()> {
    let jj_client = ctx.jj();
    let repo_root = jj_client.repo_root();
    let db_path = repo_root.join(".jj").join("jjj.db");

    if !db_path.exists() {
        return Ok(());
    }

    let db = Database::open(&db_path)?;
    let conn = db.conn();

    // Check if this entity has an embedding
    let has_embedding = crate::db::embeddings::load_embedding(conn, entity_type, entity_id)?
        .is_some();

    if !has_embedding {
        return Ok(());
    }

    let results = search::find_similar(conn, entity_type, entity_id, None, 5)?;

    if results.is_empty() {
        return Ok(());
    }

    // Filter to similarity > 0.5
    let results: Vec<_> = results.into_iter().filter(|r| r.similarity > 0.5).collect();

    if results.is_empty() {
        return Ok(());
    }

    println!("\n{}", "─".repeat(50));
    println!("\nRelated:");
    for result in results {
        let short_id = &result.entity_id[..6.min(result.entity_id.len())];
        println!(
            "  {}/{}  [{:.2}]  \"{}\"",
            result.entity_type.chars().next().unwrap(),
            short_id,
            result.similarity,
            truncate_title(&result.title, 50)
        );
    }

    Ok(())
}

fn truncate_title(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}
```

**Step 3: Run tests**

Run: `cargo test commands::problem`
Expected: All tests pass

**Step 4: Commit**

```bash
git add src/commands/problem.rs
git commit -m "feat: add related items section to problem show"
```

---

## Task 2: Add Related Section to Solution, Critique, Milestone Show

**Files:**
- Modify: `src/commands/solution.rs`
- Modify: `src/commands/critique.rs`
- Modify: `src/commands/milestone.rs`

**Step 1: Move helper to shared location**

Move `show_related_items` and `truncate_title` from `problem.rs` to `src/commands/mod.rs` and make them `pub(crate)`.

**Step 2: Update solution show**

In `src/commands/solution.rs`, add after displaying solution:

```rust
    crate::commands::show_related_items(ctx, "solution", &solution.id)?;
```

**Step 3: Update critique show**

In `src/commands/critique.rs`, add after displaying critique:

```rust
    crate::commands::show_related_items(ctx, "critique", &critique.id)?;
```

**Step 4: Update milestone show**

In `src/commands/milestone.rs`, add after displaying milestone:

```rust
    crate::commands::show_related_items(ctx, "milestone", &milestone.id)?;
```

**Step 5: Run tests**

Run: `cargo test`
Expected: All tests pass

**Step 6: Commit**

```bash
git add src/commands/mod.rs src/commands/problem.rs src/commands/solution.rs src/commands/critique.rs src/commands/milestone.rs
git commit -m "feat: add related items section to all show commands"
```

---

## Task 3: Add Related Panel to TUI

**Files:**
- Modify: `src/tui/app.rs`
- Modify: `src/tui/ui.rs`

**Step 1: Add state for related items**

In `src/tui/app.rs`, add to the App struct:

```rust
    pub show_related: bool,
    pub related_items: Vec<crate::db::search::SimilarityResult>,
    pub related_selected: usize,
```

Initialize in `new()`:

```rust
    show_related: true,
    related_items: Vec::new(),
    related_selected: 0,
```

**Step 2: Add method to load related items**

```rust
impl App {
    pub fn load_related_for_current(&mut self) {
        if let Some((entity_type, entity_id)) = self.get_current_entity() {
            if let Ok(results) = crate::db::search::find_similar(
                self.db.conn(),
                &entity_type,
                &entity_id,
                None,
                5,
            ) {
                self.related_items = results.into_iter().filter(|r| r.similarity > 0.5).collect();
                self.related_selected = 0;
            }
        }
    }

    fn get_current_entity(&self) -> Option<(String, String)> {
        // Return entity type and ID based on current selection
        // Implementation depends on current TUI state
    }
}
```

**Step 3: Add keybinding for 'r' to toggle related**

In the key handler, add:

```rust
    KeyCode::Char('r') => {
        self.show_related = !self.show_related;
    }
```

**Step 4: Render related panel in ui.rs**

Add a function to render the related panel at the bottom when `show_related` is true:

```rust
fn render_related_panel(f: &mut Frame, app: &App, area: Rect) {
    if !app.show_related || app.related_items.is_empty() {
        return;
    }

    let items: Vec<ListItem> = app
        .related_items
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let style = if i == app.related_selected {
                Style::default().bg(Color::DarkGray)
            } else {
                Style::default()
            };
            let short_id = &r.entity_id[..6.min(r.entity_id.len())];
            ListItem::new(format!(
                "{}/{}  [{:.2}]  {}",
                r.entity_type.chars().next().unwrap(),
                short_id,
                r.similarity,
                r.title
            ))
            .style(style)
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Related (r to toggle)"));

    f.render_widget(list, area);
}
```

**Step 5: Run tests**

Run: `cargo test tui`
Expected: All tests pass

**Step 6: Commit**

```bash
git add src/tui/app.rs src/tui/ui.rs
git commit -m "feat: add related items panel to TUI"
```

---

## Task 4: Add Duplicate Detection Config

**Files:**
- Modify: `src/local_config.rs`

**Step 1: Add duplicate detection fields to EmbeddingsConfig**

```rust
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EmbeddingsConfig {
    // ... existing fields ...

    /// Similarity threshold for duplicate warnings (default: 0.85)
    #[serde(default)]
    pub duplicate_threshold: Option<f32>,

    /// Enable duplicate checking on create (default: true when embeddings enabled)
    #[serde(default)]
    pub duplicate_check_enabled: Option<bool>,
}
```

**Step 2: Add env var overrides**

In `apply_env_overrides()`:

```rust
    if let Ok(val) = std::env::var("JJJ_EMBEDDINGS_DUPLICATE_THRESHOLD") {
        if let Ok(threshold) = val.parse() {
            self.embeddings.duplicate_threshold = Some(threshold);
        }
    }
    if let Ok(val) = std::env::var("JJJ_EMBEDDINGS_DUPLICATE_CHECK") {
        self.embeddings.duplicate_check_enabled = Some(val == "true" || val == "1");
    }
```

**Step 3: Add helper methods**

```rust
impl LocalConfig {
    pub fn duplicate_threshold(&self) -> f32 {
        self.embeddings.duplicate_threshold.unwrap_or(0.85)
    }

    pub fn duplicate_check_enabled(&self) -> bool {
        self.embeddings.duplicate_check_enabled.unwrap_or(true)
    }
}
```

**Step 4: Run tests**

Run: `cargo test local_config`
Expected: All tests pass

**Step 5: Commit**

```bash
git add src/local_config.rs
git commit -m "feat: add duplicate detection config options"
```

---

## Task 5: Add --force Flag to Problem Create

**Files:**
- Modify: `src/cli.rs`
- Modify: `src/commands/mod.rs`
- Modify: `src/commands/problem.rs`

**Step 1: Add --force flag to ProblemAction::New**

In `src/cli.rs`, find `ProblemAction::New` and add:

```rust
    New {
        /// Problem title
        title: String,

        /// Skip duplicate checking
        #[arg(long, short = 'f')]
        force: bool,
    },
```

**Step 2: Update command dispatch**

In `src/commands/mod.rs`, update the Problem::New dispatch to pass `force`.

**Step 3: Update problem create function**

In `src/commands/problem.rs`, update the signature and add duplicate check:

```rust
pub fn create(ctx: &CommandContext, title: &str, force: bool) -> Result<()> {
    // If not forcing, check for duplicates
    if !force {
        if let Some(similar) = check_for_duplicates(ctx, title)? {
            if !prompt_create_anyway(&similar)? {
                println!("Cancelled.");
                return Ok(());
            }
        }
    }

    // ... existing create logic ...
}

fn check_for_duplicates(ctx: &CommandContext, title: &str) -> Result<Option<Vec<SimilarityResult>>> {
    let jj_client = ctx.jj();
    let repo_root = jj_client.repo_root();
    let db_path = repo_root.join(".jj").join("jjj.db");

    if !db_path.exists() {
        return Ok(None);
    }

    let local_config = LocalConfig::load(repo_root);
    if !local_config.duplicate_check_enabled() {
        return Ok(None);
    }

    let client = EmbeddingClient::from_config(&local_config, false);
    let client = match client {
        Some(c) => c,
        None => return Ok(None),
    };

    let db = Database::open(&db_path)?;
    let conn = db.conn();

    // Embed the title
    let embedding = match client.embed(title) {
        Ok(e) => e,
        Err(_) => return Ok(None),
    };

    // Find similar problems
    let threshold = local_config.duplicate_threshold();
    let results = search::similarity_search(conn, &embedding, Some("problem"), None, 5)?;
    let similar: Vec<_> = results.into_iter().filter(|r| r.similarity >= threshold).collect();

    if similar.is_empty() {
        Ok(None)
    } else {
        Ok(Some(similar))
    }
}

fn prompt_create_anyway(similar: &[SimilarityResult]) -> Result<bool> {
    println!("\nSimilar existing problems found:\n");
    for result in similar {
        let short_id = &result.entity_id[..6.min(result.entity_id.len())];
        println!(
            "  p/{}  [{:.2}]  \"{}\"",
            short_id,
            result.similarity,
            result.title
        );
    }
    println!();

    print!("Create anyway? [y/N] ");
    std::io::Write::flush(&mut std::io::stdout())?;

    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    let input = input.trim().to_lowercase();

    Ok(input == "y" || input == "yes")
}
```

**Step 4: Run tests**

Run: `cargo test`
Expected: All tests pass

**Step 5: Commit**

```bash
git add src/cli.rs src/commands/mod.rs src/commands/problem.rs
git commit -m "feat: add duplicate detection on problem create"
```

---

## Task 6: Add --force Flag to Solution Create

**Files:**
- Modify: `src/cli.rs`
- Modify: `src/commands/mod.rs`
- Modify: `src/commands/solution.rs`

**Step 1: Add --force flag to SolutionAction::New**

In `src/cli.rs`, find `SolutionAction::New` and add the force flag.

**Step 2: Update command dispatch and solution create**

Similar to problem create, add duplicate check for solutions (checking against other solutions for the same problem).

**Step 3: Run tests**

Run: `cargo test`
Expected: All tests pass

**Step 4: Commit**

```bash
git add src/cli.rs src/commands/mod.rs src/commands/solution.rs
git commit -m "feat: add duplicate detection on solution create"
```

---

## Task 7: Integration Tests for Phase 2+3

**Files:**
- Create: `tests/embedding_recommendations_test.rs`

**Step 1: Create integration tests**

```rust
//! Integration tests for embedding recommendations (Phase 2+3).

use jjj::db::{Database, embeddings, search};
use jjj::models::Problem;
use jjj::db::entities::upsert_problem;
use jjj::local_config::LocalConfig;

#[test]
fn test_find_similar_filters_by_threshold() {
    let db = Database::open_in_memory().expect("Failed to open database");
    let conn = db.conn();

    // Create problems with embeddings
    let p1 = Problem::new("p1".to_string(), "Auth bug".to_string());
    let p2 = Problem::new("p2".to_string(), "Similar auth issue".to_string());
    let p3 = Problem::new("p3".to_string(), "Unrelated".to_string());

    upsert_problem(conn, &p1).expect("insert");
    upsert_problem(conn, &p2).expect("insert");
    upsert_problem(conn, &p3).expect("insert");

    embeddings::upsert_embedding(conn, "problem", "p1", "test", &[1.0, 0.0, 0.0]).expect("embed");
    embeddings::upsert_embedding(conn, "problem", "p2", "test", &[0.95, 0.05, 0.0]).expect("embed");
    embeddings::upsert_embedding(conn, "problem", "p3", "test", &[0.0, 0.0, 1.0]).expect("embed");

    let results = search::find_similar(conn, "problem", "p1", None, 10).expect("search");

    // p2 should be very similar (>0.9), p3 should be dissimilar (<0.1)
    assert_eq!(results.len(), 2);
    assert!(results[0].similarity > 0.9);
    assert!(results[1].similarity < 0.1);
}

#[test]
fn test_duplicate_threshold_config() {
    let config = LocalConfig::default();
    assert_eq!(config.duplicate_threshold(), 0.85);
    assert!(config.duplicate_check_enabled());
}
```

**Step 2: Run tests**

Run: `cargo test embedding_recommendations`
Expected: All tests pass

**Step 3: Commit**

```bash
git add tests/embedding_recommendations_test.rs
git commit -m "test: add integration tests for embedding recommendations"
```

---

## Task 8: Final Verification

**Step 1: Run full test suite**

Run: `cargo test`
Expected: All tests pass

**Step 2: Run clippy**

Run: `cargo clippy`
Expected: No errors

**Step 3: Run fmt**

Run: `cargo fmt --check`
Expected: No formatting issues (or run `cargo fmt` to fix)

**Step 4: Build release**

Run: `cargo build --release`
Expected: Build succeeds

**Step 5: Commit any final fixes**

```bash
git add -A
git commit -m "chore: final verification for Phase 2+3"
```

---

## Summary

Phase 2+3 implementation:

1. **Task 1**: Related section in problem show
2. **Task 2**: Related section in solution/critique/milestone show
3. **Task 3**: Related panel in TUI
4. **Task 4**: Duplicate detection config options
5. **Task 5**: Duplicate detection on problem create with --force
6. **Task 6**: Duplicate detection on solution create with --force
7. **Task 7**: Integration tests
8. **Task 8**: Final verification

**Key features delivered:**
- "Related:" section at bottom of all `jjj show` commands
- Related panel in TUI (toggle with 'r')
- Duplicate warning on `jjj problem new` and `jjj solution new`
- `--force` flag to bypass duplicate checking
- Configurable threshold (`duplicate_threshold = 0.85`)
- Configurable enable/disable (`duplicate_check_enabled = true`)
