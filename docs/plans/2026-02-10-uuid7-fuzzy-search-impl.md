# UUID7 + Fuzzy Search Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace sequential IDs (`p1`, `s1`) with UUID7 identifiers and fuzzy title search for distributed entity creation without collisions.

**Architecture:** UUID7 generation replaces `next_*_id()` functions. A new `resolve` module handles ID resolution (exact UUID → prefix match → fuzzy search). An interactive picker handles ambiguous matches. Files rename from `p1.md` to `{uuid7}.md`. Branch renames from `jjj/meta` to `jjj`.

**Tech Stack:** `uuid` crate with v7 feature, `dialoguer` for interactive picker, existing SQLite FTS5 for fuzzy search.

---

## Task 1: Add UUID Dependency

**Files:**
- Modify: `Cargo.toml`

**Step 1: Add uuid crate with v7 feature**

In `Cargo.toml`, add to `[dependencies]` section after the `rusqlite` line:

```toml
# UUID generation
uuid = { version = "1.11", features = ["v7"] }
```

**Step 2: Verify compilation**

Run: `cargo check`
Expected: Compiles successfully with new dependency

**Step 3: Commit**

```bash
git add Cargo.toml
git commit -m "deps: add uuid crate with v7 feature"
```

---

## Task 2: Add Dialoguer Dependency

**Files:**
- Modify: `Cargo.toml`

**Step 1: Add dialoguer crate**

In `Cargo.toml`, add to `[dependencies]` section after the `uuid` line:

```toml
# Interactive prompts
dialoguer = "0.11"
```

**Step 2: Verify compilation**

Run: `cargo check`
Expected: Compiles successfully

**Step 3: Commit**

```bash
git add Cargo.toml
git commit -m "deps: add dialoguer for interactive picker"
```

---

## Task 3: Create UUID7 Generation Module

**Files:**
- Create: `src/id.rs`
- Modify: `src/lib.rs`

**Step 1: Write tests for UUID7 generation**

Create `src/id.rs`:

```rust
//! UUID7 generation and utilities for entity IDs.

use uuid::Uuid;

/// Generate a new UUID7 identifier.
///
/// UUID7 is time-ordered, so IDs sort chronologically.
pub fn generate_id() -> String {
    Uuid::now_v7().to_string()
}

/// Check if a string is a valid UUID.
pub fn is_uuid(s: &str) -> bool {
    Uuid::parse_str(s).is_ok()
}

/// Check if a string looks like a hex prefix (for prefix matching).
/// Must be 6+ hex characters.
pub fn is_hex_prefix(s: &str) -> bool {
    s.len() >= 6 && s.chars().all(|c| c.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_id_is_valid_uuid() {
        let id = generate_id();
        assert!(is_uuid(&id), "Generated ID should be valid UUID: {}", id);
    }

    #[test]
    fn test_generate_id_is_unique() {
        let id1 = generate_id();
        let id2 = generate_id();
        assert_ne!(id1, id2, "Generated IDs should be unique");
    }

    #[test]
    fn test_generate_id_is_time_ordered() {
        let id1 = generate_id();
        std::thread::sleep(std::time::Duration::from_millis(1));
        let id2 = generate_id();
        assert!(id1 < id2, "UUIDs should sort chronologically: {} vs {}", id1, id2);
    }

    #[test]
    fn test_is_uuid() {
        assert!(is_uuid("01957d3e-a8b2-7def-8c3a-9f4e5d6c7b8a"));
        assert!(is_uuid("550e8400-e29b-41d4-a716-446655440000"));
        assert!(!is_uuid("p1"));
        assert!(!is_uuid("not-a-uuid"));
        assert!(!is_uuid(""));
    }

    #[test]
    fn test_is_hex_prefix() {
        assert!(is_hex_prefix("a3f8c2"));
        assert!(is_hex_prefix("01957d3e"));
        assert!(is_hex_prefix("ABCDEF"));
        assert!(!is_hex_prefix("a3f8c")); // too short
        assert!(!is_hex_prefix("auth")); // not hex
        assert!(!is_hex_prefix("p1"));
    }
}
```

**Step 2: Add module to lib.rs**

In `src/lib.rs`, add after the existing module declarations:

```rust
pub mod id;
```

**Step 3: Run tests**

Run: `cargo test id::`
Expected: All 5 tests pass

**Step 4: Commit**

```bash
git add src/id.rs src/lib.rs
git commit -m "feat: add UUID7 generation module"
```

---

## Task 4: Create Truncated Prefix Display Module

**Files:**
- Create: `src/display.rs`
- Modify: `src/lib.rs`

**Step 1: Write tests for truncated prefix calculation**

Create `src/display.rs`:

```rust
//! Display formatting utilities for entity IDs.

/// Minimum prefix length for truncated display.
const MIN_PREFIX_LEN: usize = 6;

/// Calculate unambiguous prefixes for a list of UUIDs.
///
/// Returns a Vec of (uuid, prefix) pairs where each prefix is the shortest
/// unambiguous prefix (minimum 6 chars).
pub fn truncated_prefixes(uuids: &[&str]) -> Vec<(String, String)> {
    uuids
        .iter()
        .map(|uuid| {
            let prefix = shortest_unambiguous_prefix(uuid, uuids);
            (uuid.to_string(), prefix)
        })
        .collect()
}

/// Find the shortest unambiguous prefix for a UUID within a set.
fn shortest_unambiguous_prefix(uuid: &str, all_uuids: &[&str]) -> String {
    // Remove hyphens for prefix calculation
    let normalized: String = uuid.chars().filter(|c| *c != '-').collect();
    let all_normalized: Vec<String> = all_uuids
        .iter()
        .map(|u| u.chars().filter(|c| *c != '-').collect())
        .collect();

    for len in MIN_PREFIX_LEN..=normalized.len() {
        let prefix = &normalized[..len];
        let matches = all_normalized
            .iter()
            .filter(|u| u.starts_with(prefix))
            .count();
        if matches == 1 {
            return prefix.to_lowercase();
        }
    }

    // Fallback to full UUID (normalized)
    normalized.to_lowercase()
}

/// Format an entity for mixed-type listings with type prefix.
pub fn format_with_type_prefix(entity_type: &str, prefix: &str) -> String {
    let type_char = match entity_type {
        "problem" => "p",
        "solution" => "s",
        "critique" => "c",
        "milestone" => "m",
        _ => "?",
    };
    format!("{}/{}", type_char, prefix)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_uuid_uses_min_prefix() {
        let uuids = vec!["01957d3e-a8b2-7def-8c3a-9f4e5d6c7b8a"];
        let result = truncated_prefixes(&uuids);
        assert_eq!(result[0].1.len(), MIN_PREFIX_LEN);
    }

    #[test]
    fn test_different_uuids_use_min_prefix() {
        let uuids = vec![
            "01957d3e-a8b2-7def-8c3a-9f4e5d6c7b8a",
            "02957d3e-b1c4-7abc-9d2e-3f4a5b6c7d8e",
        ];
        let result = truncated_prefixes(&uuids);
        assert_eq!(result[0].1.len(), MIN_PREFIX_LEN);
        assert_eq!(result[1].1.len(), MIN_PREFIX_LEN);
    }

    #[test]
    fn test_similar_uuids_extend_prefix() {
        let uuids = vec![
            "a3f8c2de-a8b2-7def-8c3a-9f4e5d6c7b8a",
            "a3f8c2df-b1c4-7abc-9d2e-3f4a5b6c7d8e",
        ];
        let result = truncated_prefixes(&uuids);
        // Both start with "a3f8c2d", need to extend to 8 chars
        assert!(result[0].1.len() > MIN_PREFIX_LEN, "Prefix should extend: {}", result[0].1);
        assert!(result[1].1.len() > MIN_PREFIX_LEN, "Prefix should extend: {}", result[1].1);
        assert_ne!(result[0].1, result[1].1);
    }

    #[test]
    fn test_format_with_type_prefix() {
        assert_eq!(format_with_type_prefix("problem", "a3f8c2"), "p/a3f8c2");
        assert_eq!(format_with_type_prefix("solution", "b7e2f9"), "s/b7e2f9");
        assert_eq!(format_with_type_prefix("critique", "c1d2e3"), "c/c1d2e3");
        assert_eq!(format_with_type_prefix("milestone", "d4e5f6"), "m/d4e5f6");
    }
}
```

**Step 2: Add module to lib.rs**

In `src/lib.rs`, add after `pub mod id;`:

```rust
pub mod display;
```

**Step 3: Run tests**

Run: `cargo test display::`
Expected: All 4 tests pass

**Step 4: Commit**

```bash
git add src/display.rs src/lib.rs
git commit -m "feat: add truncated prefix display utilities"
```

---

## Task 5: Create Entity Resolution Module

**Files:**
- Create: `src/resolve.rs`
- Modify: `src/lib.rs`

**Step 1: Write the resolve module with tests**

Create `src/resolve.rs`:

```rust
//! Entity resolution from user input to UUID.
//!
//! Resolution priority:
//! 1. Exact UUID match
//! 2. Prefix match (hex string starting with input)
//! 3. Fuzzy title search via SQLite FTS

use crate::id::{is_hex_prefix, is_uuid};

/// Result of resolving user input to entities.
#[derive(Debug, Clone)]
pub enum ResolveResult {
    /// Exactly one entity matched
    Single(String),
    /// Multiple entities matched - need disambiguation
    Multiple(Vec<ResolveMatch>),
    /// No entities matched
    None,
}

/// A matched entity with its ID and title.
#[derive(Debug, Clone)]
pub struct ResolveMatch {
    pub id: String,
    pub title: String,
}

/// Resolve user input to entity ID(s).
///
/// Takes a list of (id, title) pairs representing available entities.
pub fn resolve(input: &str, entities: &[(String, String)]) -> ResolveResult {
    // 1. Exact UUID match
    if is_uuid(input) {
        if entities.iter().any(|(id, _)| id == input) {
            return ResolveResult::Single(input.to_string());
        }
        return ResolveResult::None;
    }

    // 2. Prefix match (if input looks like hex)
    if is_hex_prefix(input) {
        let normalized_input: String = input.chars().filter(|c| *c != '-').collect();
        let matches: Vec<_> = entities
            .iter()
            .filter(|(id, _)| {
                let normalized_id: String = id.chars().filter(|c| *c != '-').collect();
                normalized_id.to_lowercase().starts_with(&normalized_input.to_lowercase())
            })
            .map(|(id, title)| ResolveMatch {
                id: id.clone(),
                title: title.clone(),
            })
            .collect();

        return match matches.len() {
            0 => ResolveResult::None,
            1 => ResolveResult::Single(matches[0].id.clone()),
            _ => ResolveResult::Multiple(matches),
        };
    }

    // 3. Fuzzy title search (simple contains for now, FTS in actual use)
    let input_lower = input.to_lowercase();
    let matches: Vec<_> = entities
        .iter()
        .filter(|(_, title)| title.to_lowercase().contains(&input_lower))
        .map(|(id, title)| ResolveMatch {
            id: id.clone(),
            title: title.clone(),
        })
        .collect();

    match matches.len() {
        0 => ResolveResult::None,
        1 => ResolveResult::Single(matches[0].id.clone()),
        _ => ResolveResult::Multiple(matches),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_entities() -> Vec<(String, String)> {
        vec![
            ("01957d3e-a8b2-7def-8c3a-9f4e5d6c7b8a".to_string(), "Fix auth timeout bug".to_string()),
            ("01957d3e-b1c4-7abc-9d2e-3f4a5b6c7d8e".to_string(), "Auth token refresh fails".to_string()),
            ("02957d3e-c2d5-7fed-ae4b-5c6d7e8f9a0b".to_string(), "Database connection pooling".to_string()),
        ]
    }

    #[test]
    fn test_resolve_exact_uuid() {
        let entities = test_entities();
        match resolve("01957d3e-a8b2-7def-8c3a-9f4e5d6c7b8a", &entities) {
            ResolveResult::Single(id) => assert_eq!(id, "01957d3e-a8b2-7def-8c3a-9f4e5d6c7b8a"),
            other => panic!("Expected Single, got {:?}", other),
        }
    }

    #[test]
    fn test_resolve_uuid_not_found() {
        let entities = test_entities();
        match resolve("99999999-9999-9999-9999-999999999999", &entities) {
            ResolveResult::None => {}
            other => panic!("Expected None, got {:?}", other),
        }
    }

    #[test]
    fn test_resolve_prefix_unique() {
        let entities = test_entities();
        // "02957d" only matches the third entity
        match resolve("02957d", &entities) {
            ResolveResult::Single(id) => assert_eq!(id, "02957d3e-c2d5-7fed-ae4b-5c6d7e8f9a0b"),
            other => panic!("Expected Single, got {:?}", other),
        }
    }

    #[test]
    fn test_resolve_prefix_ambiguous() {
        let entities = test_entities();
        // "01957d" matches two entities
        match resolve("01957d", &entities) {
            ResolveResult::Multiple(matches) => {
                assert_eq!(matches.len(), 2);
            }
            other => panic!("Expected Multiple, got {:?}", other),
        }
    }

    #[test]
    fn test_resolve_title_unique() {
        let entities = test_entities();
        match resolve("database", &entities) {
            ResolveResult::Single(id) => assert_eq!(id, "02957d3e-c2d5-7fed-ae4b-5c6d7e8f9a0b"),
            other => panic!("Expected Single, got {:?}", other),
        }
    }

    #[test]
    fn test_resolve_title_ambiguous() {
        let entities = test_entities();
        // "auth" matches two entities
        match resolve("auth", &entities) {
            ResolveResult::Multiple(matches) => {
                assert_eq!(matches.len(), 2);
            }
            other => panic!("Expected Multiple, got {:?}", other),
        }
    }

    #[test]
    fn test_resolve_title_not_found() {
        let entities = test_entities();
        match resolve("nonexistent", &entities) {
            ResolveResult::None => {}
            other => panic!("Expected None, got {:?}", other),
        }
    }
}
```

**Step 2: Add module to lib.rs**

In `src/lib.rs`, add after `pub mod display;`:

```rust
pub mod resolve;
```

**Step 3: Run tests**

Run: `cargo test resolve::`
Expected: All 7 tests pass

**Step 4: Commit**

```bash
git add src/resolve.rs src/lib.rs
git commit -m "feat: add entity resolution module"
```

---

## Task 6: Create Interactive Picker Module

**Files:**
- Create: `src/picker.rs`
- Modify: `src/lib.rs`

**Step 1: Write the picker module**

Create `src/picker.rs`:

```rust
//! Interactive picker for disambiguation.

use crate::display::truncated_prefixes;
use crate::error::{JjjError, Result};
use crate::resolve::ResolveMatch;
use dialoguer::{theme::ColorfulTheme, FuzzySelect};
use std::io::IsTerminal;

/// Pick one entity from multiple matches.
///
/// If stdout is a TTY, shows an interactive picker.
/// Otherwise, returns an error with suggestions.
pub fn pick_one(matches: &[ResolveMatch], entity_type: &str) -> Result<String> {
    if matches.is_empty() {
        return Err(JjjError::EntityNotFound(format!("No {}s found", entity_type)));
    }

    if matches.len() == 1 {
        return Ok(matches[0].id.clone());
    }

    if std::io::stdout().is_terminal() {
        pick_interactive(matches, entity_type)
    } else {
        pick_non_interactive(matches, entity_type)
    }
}

fn pick_interactive(matches: &[ResolveMatch], entity_type: &str) -> Result<String> {
    // Calculate truncated prefixes for display
    let uuids: Vec<&str> = matches.iter().map(|m| m.id.as_str()).collect();
    let prefixes = truncated_prefixes(&uuids);

    // Build display strings
    let items: Vec<String> = matches
        .iter()
        .zip(prefixes.iter())
        .map(|(m, (_, prefix))| format!("{}  {}", prefix, m.title))
        .collect();

    let selection = FuzzySelect::with_theme(&ColorfulTheme::default())
        .with_prompt(format!("Select {}:", entity_type))
        .items(&items)
        .default(0)
        .interact_opt()
        .map_err(|e| JjjError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;

    match selection {
        Some(index) => Ok(matches[index].id.clone()),
        None => Err(JjjError::Cancelled("Selection cancelled".to_string())),
    }
}

fn pick_non_interactive(matches: &[ResolveMatch], entity_type: &str) -> Result<String> {
    // Calculate truncated prefixes for display
    let uuids: Vec<&str> = matches.iter().map(|m| m.id.as_str()).collect();
    let prefixes = truncated_prefixes(&uuids);

    let mut msg = format!("Multiple {}s match. Be more specific or use the short ID:\n", entity_type);

    let display_count = matches.len().min(10);
    for (m, (_, prefix)) in matches.iter().zip(prefixes.iter()).take(display_count) {
        msg.push_str(&format!("  {}  {}\n", prefix, m.title));
    }

    if matches.len() > 10 {
        msg.push_str(&format!("  ... and {} more\n", matches.len() - 10));
    }

    Err(JjjError::AmbiguousMatch(msg))
}
```

**Step 2: Add error variants to error.rs**

In `src/error.rs`, add these variants to the `JjjError` enum:

```rust
    #[error("Selection cancelled: {0}")]
    Cancelled(String),

    #[error("{0}")]
    AmbiguousMatch(String),

    #[error("Entity not found: {0}")]
    EntityNotFound(String),
```

**Step 3: Add module to lib.rs**

In `src/lib.rs`, add after `pub mod resolve;`:

```rust
pub mod picker;
```

**Step 4: Verify compilation**

Run: `cargo check`
Expected: Compiles successfully

**Step 5: Commit**

```bash
git add src/picker.rs src/error.rs src/lib.rs
git commit -m "feat: add interactive picker for disambiguation"
```

---

## Task 7: Rename Branch Constant

**Files:**
- Modify: `src/storage.rs`

**Step 1: Update the META_BOOKMARK constant**

In `src/storage.rs`, change line 12 from:

```rust
const META_BOOKMARK: &str = "jjj/meta";
```

to:

```rust
const META_BOOKMARK: &str = "jjj";
```

**Step 2: Run tests**

Run: `cargo test`
Expected: All tests pass (storage tests use the constant)

**Step 3: Commit**

```bash
git add src/storage.rs
git commit -m "refactor: rename metadata branch from jjj/meta to jjj"
```

---

## Task 8: Replace next_problem_id with UUID7

**Files:**
- Modify: `src/storage.rs`

**Step 1: Update next_problem_id function**

In `src/storage.rs`, replace the `next_problem_id` function (around lines 321-330) with:

```rust
    /// Generate next problem ID using UUID7.
    pub fn next_problem_id(&self) -> Result<String> {
        Ok(crate::id::generate_id())
    }
```

**Step 2: Run tests**

Run: `cargo test`
Expected: Tests pass (existing tests may need updates in later tasks)

**Step 3: Commit**

```bash
git add src/storage.rs
git commit -m "feat: use UUID7 for problem ID generation"
```

---

## Task 9: Replace next_solution_id with UUID7

**Files:**
- Modify: `src/storage.rs`

**Step 1: Update next_solution_id function**

In `src/storage.rs`, replace the `next_solution_id` function (around lines 483-494) with:

```rust
    /// Generate next solution ID using UUID7.
    pub fn next_solution_id(&self) -> Result<String> {
        Ok(crate::id::generate_id())
    }
```

**Step 2: Run tests**

Run: `cargo test`
Expected: Tests pass

**Step 3: Commit**

```bash
git add src/storage.rs
git commit -m "feat: use UUID7 for solution ID generation"
```

---

## Task 10: Replace next_critique_id with UUID7

**Files:**
- Modify: `src/storage.rs`

**Step 1: Update next_critique_id function**

In `src/storage.rs`, replace the `next_critique_id` function (around lines 622-632) with:

```rust
    /// Generate next critique ID using UUID7.
    pub fn next_critique_id(&self) -> Result<String> {
        Ok(crate::id::generate_id())
    }
```

**Step 2: Run tests**

Run: `cargo test`
Expected: Tests pass

**Step 3: Commit**

```bash
git add src/storage.rs
git commit -m "feat: use UUID7 for critique ID generation"
```

---

## Task 11: Replace next_milestone_id with UUID7

**Files:**
- Modify: `src/storage.rs`

**Step 1: Update next_milestone_id function**

In `src/storage.rs`, replace the `next_milestone_id` function (around lines 755-765) with:

```rust
    /// Generate next milestone ID using UUID7.
    pub fn next_milestone_id(&self) -> Result<String> {
        Ok(crate::id::generate_id())
    }
```

**Step 2: Run tests**

Run: `cargo test`
Expected: Tests pass

**Step 3: Commit**

```bash
git add src/storage.rs
git commit -m "feat: use UUID7 for milestone ID generation"
```

---

## Task 12: Create Entity Resolver Helper in Context

**Files:**
- Modify: `src/context.rs`

**Step 1: Read current context.rs file**

First read the file to understand current structure.

**Step 2: Add resolver methods**

Add these helper methods to `CommandContext`:

```rust
    /// Resolve a problem ID from user input.
    pub fn resolve_problem(&self, input: &str) -> Result<String> {
        use crate::resolve::{resolve, ResolveResult};
        use crate::picker::pick_one;

        let problems = self.store.list_problems()?;
        let entities: Vec<(String, String)> = problems
            .iter()
            .map(|p| (p.id.clone(), p.title.clone()))
            .collect();

        match resolve(input, &entities) {
            ResolveResult::Single(id) => Ok(id),
            ResolveResult::Multiple(matches) => pick_one(&matches, "problem"),
            ResolveResult::None => Err(crate::error::JjjError::ProblemNotFound(input.to_string())),
        }
    }

    /// Resolve a solution ID from user input.
    pub fn resolve_solution(&self, input: &str) -> Result<String> {
        use crate::resolve::{resolve, ResolveResult};
        use crate::picker::pick_one;

        let solutions = self.store.list_solutions()?;
        let entities: Vec<(String, String)> = solutions
            .iter()
            .map(|s| (s.id.clone(), s.title.clone()))
            .collect();

        match resolve(input, &entities) {
            ResolveResult::Single(id) => Ok(id),
            ResolveResult::Multiple(matches) => pick_one(&matches, "solution"),
            ResolveResult::None => Err(crate::error::JjjError::SolutionNotFound(input.to_string())),
        }
    }

    /// Resolve a critique ID from user input.
    pub fn resolve_critique(&self, input: &str) -> Result<String> {
        use crate::resolve::{resolve, ResolveResult};
        use crate::picker::pick_one;

        let critiques = self.store.list_critiques()?;
        let entities: Vec<(String, String)> = critiques
            .iter()
            .map(|c| (c.id.clone(), c.title.clone()))
            .collect();

        match resolve(input, &entities) {
            ResolveResult::Single(id) => Ok(id),
            ResolveResult::Multiple(matches) => pick_one(&matches, "critique"),
            ResolveResult::None => Err(crate::error::JjjError::CritiqueNotFound(input.to_string())),
        }
    }

    /// Resolve a milestone ID from user input.
    pub fn resolve_milestone(&self, input: &str) -> Result<String> {
        use crate::resolve::{resolve, ResolveResult};
        use crate::picker::pick_one;

        let milestones = self.store.list_milestones()?;
        let entities: Vec<(String, String)> = milestones
            .iter()
            .map(|m| (m.id.clone(), m.title.clone()))
            .collect();

        match resolve(input, &entities) {
            ResolveResult::Single(id) => Ok(id),
            ResolveResult::Multiple(matches) => pick_one(&matches, "milestone"),
            ResolveResult::None => Err(crate::error::JjjError::MilestoneNotFound(input.to_string())),
        }
    }
```

**Step 3: Verify compilation**

Run: `cargo check`
Expected: Compiles successfully

**Step 4: Commit**

```bash
git add src/context.rs
git commit -m "feat: add entity resolution helpers to CommandContext"
```

---

## Task 13: Update Problem Commands to Use Resolution

**Files:**
- Modify: `src/commands/problem.rs`

**Step 1: Update show_problem function**

Find the `show_problem` function and replace the direct `store.load_problem` call with resolution:

```rust
fn show_problem(ctx: &CommandContext, problem_input: String, json: bool) -> Result<()> {
    let problem_id = ctx.resolve_problem(&problem_input)?;
    let problem = ctx.store.load_problem(&problem_id)?;
    // ... rest of function unchanged
}
```

**Step 2: Update edit_problem function**

```rust
fn edit_problem(
    ctx: &CommandContext,
    problem_input: String,
    title: Option<String>,
    status: Option<String>,
    priority: Option<String>,
    parent: Option<String>,
) -> Result<()> {
    let problem_id = ctx.resolve_problem(&problem_input)?;
    // ... rest of function, replacing problem_id usages
}
```

**Step 3: Update solve_problem function**

```rust
fn solve_problem(ctx: &CommandContext, problem_input: String) -> Result<()> {
    let problem_id = ctx.resolve_problem(&problem_input)?;
    // ... rest unchanged
}
```

**Step 4: Update dissolve_problem function**

```rust
fn dissolve_problem(ctx: &CommandContext, problem_input: String, reason: Option<String>) -> Result<()> {
    let problem_id = ctx.resolve_problem(&problem_input)?;
    // ... rest unchanged
}
```

**Step 5: Update assign_problem function**

```rust
fn assign_problem(ctx: &CommandContext, problem_input: String, to: Option<String>) -> Result<()> {
    let problem_id = ctx.resolve_problem(&problem_input)?;
    // ... rest unchanged
}
```

**Step 6: Update parent resolution in new_problem and edit_problem**

Where `parent` is used, resolve it:

```rust
if let Some(ref parent_input) = parent {
    let parent_id = ctx.resolve_problem(parent_input)?;
    // use parent_id
}
```

**Step 7: Run tests**

Run: `cargo test command_problem`
Expected: Tests may fail due to ID format changes - that's expected

**Step 8: Commit**

```bash
git add src/commands/problem.rs
git commit -m "feat: use entity resolution in problem commands"
```

---

## Task 14: Update Solution Commands to Use Resolution

**Files:**
- Modify: `src/commands/solution.rs`

**Step 1: Update all functions that take solution_id**

Apply the same pattern:

```rust
fn show_solution(ctx: &CommandContext, solution_input: String, json: bool) -> Result<()> {
    let solution_id = ctx.resolve_solution(&solution_input)?;
    // ...
}
```

**Step 2: Update functions that take problem references**

Where `--problem` is used:

```rust
if let Some(ref problem_input) = problem {
    let problem_id = ctx.resolve_problem(problem_input)?;
    // use problem_id
}
```

**Step 3: Update functions that take supersedes references**

```rust
if let Some(ref supersedes_input) = supersedes {
    let supersedes_id = ctx.resolve_solution(supersedes_input)?;
    // use supersedes_id
}
```

**Step 4: Run tests**

Run: `cargo test command_solution`
Expected: Some tests may fail - expected

**Step 5: Commit**

```bash
git add src/commands/solution.rs
git commit -m "feat: use entity resolution in solution commands"
```

---

## Task 15: Update Critique Commands to Use Resolution

**Files:**
- Modify: `src/commands/critique.rs`

**Step 1: Update all functions that take critique_id or solution_id**

Apply the same resolution pattern to all functions.

**Step 2: Run tests**

Run: `cargo test command_critique`

**Step 3: Commit**

```bash
git add src/commands/critique.rs
git commit -m "feat: use entity resolution in critique commands"
```

---

## Task 16: Update Milestone Commands to Use Resolution

**Files:**
- Modify: `src/commands/milestone.rs`

**Step 1: Update all functions that take milestone_id or problem_id**

Apply the same resolution pattern.

**Step 2: Run tests**

Run: `cargo test command_milestone`

**Step 3: Commit**

```bash
git add src/commands/milestone.rs
git commit -m "feat: use entity resolution in milestone commands"
```

---

## Task 17: Update List Command Display with Truncated Prefixes

**Files:**
- Modify: `src/commands/problem.rs`

**Step 1: Update list_problems display**

In the flat list display section, use truncated prefixes:

```rust
fn list_problems(/* ... */) -> Result<()> {
    // ... existing code to get problems ...

    if json {
        println!("{}", serde_json::to_string_pretty(&problems)?);
        return Ok(());
    }

    if tree {
        // ... tree display unchanged
    } else {
        if problems.is_empty() {
            println!("No problems found");
            return Ok(());
        }

        // Calculate truncated prefixes
        use crate::display::truncated_prefixes;
        let uuids: Vec<&str> = problems.iter().map(|p| p.id.as_str()).collect();
        let prefixes = truncated_prefixes(&uuids);

        for (problem, (_, prefix)) in problems.iter().zip(prefixes.iter()) {
            let status_str = format!("{}", problem.status);
            println!(
                "{}  {:12}  {}",
                prefix, status_str, problem.title
            );
        }
    }

    Ok(())
}
```

**Step 2: Run tests**

Run: `cargo test`

**Step 3: Commit**

```bash
git add src/commands/problem.rs
git commit -m "feat: display truncated prefixes in problem list"
```

---

## Task 18: Update Solution List Display

**Files:**
- Modify: `src/commands/solution.rs`

**Step 1: Update list_solutions display**

Apply same truncated prefix display pattern as problems.

**Step 2: Commit**

```bash
git add src/commands/solution.rs
git commit -m "feat: display truncated prefixes in solution list"
```

---

## Task 19: Update Critique List Display

**Files:**
- Modify: `src/commands/critique.rs`

**Step 1: Update list_critiques display**

Apply same truncated prefix display pattern.

**Step 2: Commit**

```bash
git add src/commands/critique.rs
git commit -m "feat: display truncated prefixes in critique list"
```

---

## Task 20: Update Milestone List Display

**Files:**
- Modify: `src/commands/milestone.rs`

**Step 1: Update list_milestones display**

Apply same truncated prefix display pattern.

**Step 2: Commit**

```bash
git add src/commands/milestone.rs
git commit -m "feat: display truncated prefixes in milestone list"
```

---

## Task 21: Update Status Command with Type Prefixes

**Files:**
- Modify: `src/commands/status.rs`

**Step 1: Update status display to use type prefixes**

In mixed-type listings, use `format_with_type_prefix`:

```rust
use crate::display::{truncated_prefixes, format_with_type_prefix};

// When displaying problems in status
let prefix = format_with_type_prefix("problem", &short_id);

// When displaying solutions in status
let prefix = format_with_type_prefix("solution", &short_id);
```

**Step 2: Run tests**

Run: `cargo test command_status`

**Step 3: Commit**

```bash
git add src/commands/status.rs
git commit -m "feat: use type prefixes in status command"
```

---

## Task 22: Update Integration Tests

**Files:**
- Modify: `tests/integration_test.rs`
- Modify: `tests/command_problem_test.rs`
- Modify: `tests/command_solution_test.rs`
- Modify: `tests/command_critique_test.rs`
- Modify: `tests/command_milestone_test.rs`

**Step 1: Update tests to work with UUID format**

Tests that check for specific ID formats like `p1` need to be updated to either:
1. Parse the output for UUIDs
2. Use title-based resolution in test assertions
3. Capture the returned ID from creation commands

Example pattern:

```rust
#[test]
fn test_problem_new_creates_problem() {
    let (_temp_dir, jjj) = setup_test_env();

    // Create a problem and capture output
    let output = jjj(&["problem", "new", "Test problem"]);
    assert!(output.contains("Created problem"), "Output: {}", output);

    // Extract UUID from output or use title search
    let list_output = jjj(&["problem", "list"]);
    assert!(list_output.contains("Test problem"));
}
```

**Step 2: Run all tests**

Run: `cargo test`
Expected: All tests pass

**Step 3: Commit**

```bash
git add tests/
git commit -m "test: update integration tests for UUID7 format"
```

---

## Task 23: Final Integration Test

**Files:**
- None (just running tests)

**Step 1: Run full test suite**

Run: `cargo test`
Expected: All tests pass

**Step 2: Test manual workflow**

Run:
```bash
cargo run -- init
cargo run -- problem new "Test auth bug"
cargo run -- problem list
cargo run -- problem show "auth"
```

Expected: Commands work with fuzzy search

**Step 3: Commit any final fixes**

---

## Task 24: Update Documentation

**Files:**
- Modify: `CLAUDE.md`

**Step 1: Update Entity IDs section**

Add to CLAUDE.md:

```markdown
### Entity IDs
- All entities use UUID7 identifiers (e.g., "01957d3e-a8b2-7def-8c3a-9f4e5d6c7b8a")
- UUIDs are time-ordered for natural chronological sorting
- Users can reference entities by:
  - Full UUID
  - Truncated hex prefix (minimum 6 chars, e.g., "01957d")
  - Fuzzy title match (e.g., "auth bug")
- Listings show short prefixes auto-extended for uniqueness
- Mixed-type listings use type prefixes: p/, s/, c/, m/
```

**Step 2: Commit**

```bash
git add CLAUDE.md
git commit -m "docs: update CLAUDE.md with UUID7 ID system"
```

---

## Summary

This plan implements:

1. **UUID7 generation** - Time-ordered unique IDs
2. **Entity resolution** - UUID → prefix → fuzzy search chain
3. **Interactive picker** - Dialoguer-based selection for ambiguous matches
4. **Truncated prefixes** - Git-style short IDs in listings
5. **Type prefixes** - p/, s/, c/, m/ for mixed listings
6. **Branch rename** - jjj/meta → jjj

All existing tests are updated to work with the new ID format.
