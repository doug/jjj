# Unified Critique System Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Unify Review and Critique systems into a single Critique entity that handles all feedback.

**Architecture:** Critiques absorb Reviews. Critique gains optional file/line location and threaded replies. Solution gains review tracking (requested_reviewers, reviewed_by). Review entity is removed.

**Tech Stack:** Rust, clap, serde, chrono

---

## Task 1: Add Reply Struct to Critique Model

**Files:**
- Modify: `src/models/critique.rs`
- Test: `src/models/critique.rs` (inline tests)

**Step 1: Write the failing test**

Add to `src/models/critique.rs` at the end of the `mod tests` block:

```rust
#[test]
fn test_add_reply() {
    let mut critique = Critique::new(
        "CQ-1".to_string(),
        "Test critique".to_string(),
        "S-1".to_string(),
    );

    critique.add_reply("alice".to_string(), "I disagree".to_string());

    assert_eq!(critique.replies.len(), 1);
    assert_eq!(critique.replies[0].author, "alice");
    assert_eq!(critique.replies[0].body, "I disagree");
    assert!(critique.replies[0].id.starts_with("CQ-1-R"));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test test_add_reply -p jjj`
Expected: FAIL with "no field `replies` on type `Critique`"

**Step 3: Add Reply struct and replies field**

Add after the `Critique` struct definition (around line 40):

```rust
/// A reply in a critique discussion thread
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reply {
    /// Unique reply identifier (e.g., "CQ-1-R1")
    pub id: String,

    /// Reply author
    pub author: String,

    /// Reply body (markdown)
    pub body: String,

    /// When the reply was created
    pub created_at: DateTime<Utc>,
}
```

Add to `Critique` struct (after `evidence` field):

```rust
    /// Discussion thread
    #[serde(default)]
    pub replies: Vec<Reply>,
```

Update `Critique::new()` to include:

```rust
            replies: Vec::new(),
```

Add method to `impl Critique`:

```rust
    /// Add a reply to the discussion
    pub fn add_reply(&mut self, author: String, body: String) {
        let reply_num = self.replies.len() + 1;
        let reply = Reply {
            id: format!("{}-R{}", self.id, reply_num),
            author,
            body,
            created_at: Utc::now(),
        };
        self.replies.push(reply);
        self.updated_at = Utc::now();
    }
```

**Step 4: Run test to verify it passes**

Run: `cargo test test_add_reply -p jjj`
Expected: PASS

**Step 5: Commit**

```bash
git add src/models/critique.rs
git commit -m "feat(critique): add Reply struct and discussion threading"
```

---

## Task 2: Add Code Location Fields to Critique

**Files:**
- Modify: `src/models/critique.rs`
- Test: `src/models/critique.rs` (inline tests)

**Step 1: Write the failing test**

```rust
#[test]
fn test_critique_with_location() {
    let mut critique = Critique::new(
        "CQ-1".to_string(),
        "SQL injection".to_string(),
        "S-1".to_string(),
    );

    critique.set_location(
        "src/db.rs".to_string(),
        42,
        Some(45),
        vec!["let query = format!(...)".to_string()],
    );

    assert_eq!(critique.file_path, Some("src/db.rs".to_string()));
    assert_eq!(critique.line_start, Some(42));
    assert_eq!(critique.line_end, Some(45));
    assert!(critique.has_location());
}

#[test]
fn test_critique_without_location() {
    let critique = Critique::new(
        "CQ-1".to_string(),
        "Conceptual critique".to_string(),
        "S-1".to_string(),
    );

    assert!(!critique.has_location());
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test test_critique_with_location test_critique_without_location -p jjj`
Expected: FAIL with "no field `file_path`"

**Step 3: Add location fields and methods**

Add to `Critique` struct (after `evidence` field, before `replies`):

```rust
    /// Optional file path for code-level critiques
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file_path: Option<String>,

    /// Starting line number
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line_start: Option<usize>,

    /// Ending line number
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line_end: Option<usize>,

    /// Code context for display/matching
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub code_context: Vec<String>,
```

Update `Critique::new()` to include:

```rust
            file_path: None,
            line_start: None,
            line_end: None,
            code_context: Vec::new(),
```

Add methods to `impl Critique`:

```rust
    /// Set code location for this critique
    pub fn set_location(
        &mut self,
        file_path: String,
        line_start: usize,
        line_end: Option<usize>,
        code_context: Vec<String>,
    ) {
        self.file_path = Some(file_path);
        self.line_start = Some(line_start);
        self.line_end = line_end.or(Some(line_start));
        self.code_context = code_context;
        self.updated_at = Utc::now();
    }

    /// Check if this critique has a code location
    pub fn has_location(&self) -> bool {
        self.file_path.is_some() && self.line_start.is_some()
    }
```

**Step 4: Run test to verify it passes**

Run: `cargo test test_critique_with_location test_critique_without_location -p jjj`
Expected: PASS

**Step 5: Commit**

```bash
git add src/models/critique.rs
git commit -m "feat(critique): add optional code location fields"
```

---

## Task 3: Update CritiqueFrontmatter for New Fields

**Files:**
- Modify: `src/models/critique.rs`

**Step 1: Update CritiqueFrontmatter struct**

Add new fields to `CritiqueFrontmatter`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CritiqueFrontmatter {
    pub id: String,
    pub title: String,
    pub solution_id: String,
    pub status: CritiqueStatus,
    pub severity: CritiqueSeverity,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line_start: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line_end: Option<usize>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

**Step 2: Update From implementation**

```rust
impl From<&Critique> for CritiqueFrontmatter {
    fn from(c: &Critique) -> Self {
        Self {
            id: c.id.clone(),
            title: c.title.clone(),
            solution_id: c.solution_id.clone(),
            status: c.status.clone(),
            severity: c.severity.clone(),
            author: c.author.clone(),
            file_path: c.file_path.clone(),
            line_start: c.line_start,
            line_end: c.line_end,
            created_at: c.created_at,
            updated_at: c.updated_at,
        }
    }
}
```

**Step 3: Run all tests**

Run: `cargo test -p jjj`
Expected: All tests pass

**Step 4: Commit**

```bash
git add src/models/critique.rs
git commit -m "feat(critique): update frontmatter for location fields"
```

---

## Task 4: Add Review Tracking to Solution Model

**Files:**
- Modify: `src/models/solution.rs`
- Test: `src/models/solution.rs` (inline tests)

**Step 1: Write the failing tests**

```rust
#[test]
fn test_request_reviewers() {
    let mut solution = Solution::new(
        "S-1".to_string(),
        "Test".to_string(),
        "P-1".to_string(),
    );

    solution.request_review("alice".to_string());
    solution.request_review("bob".to_string());

    assert_eq!(solution.requested_reviewers.len(), 2);
    assert!(solution.requested_reviewers.contains(&"alice".to_string()));
}

#[test]
fn test_lgtm() {
    let mut solution = Solution::new(
        "S-1".to_string(),
        "Test".to_string(),
        "P-1".to_string(),
    );

    solution.request_review("alice".to_string());
    solution.add_lgtm("alice".to_string());

    assert!(solution.reviewed_by.contains(&"alice".to_string()));
    assert!(solution.has_lgtm_from_requested_reviewer());
}

#[test]
fn test_lgtm_from_non_requested() {
    let mut solution = Solution::new(
        "S-1".to_string(),
        "Test".to_string(),
        "P-1".to_string(),
    );

    solution.request_review("alice".to_string());
    solution.add_lgtm("bob".to_string()); // Bob wasn't requested

    assert!(solution.reviewed_by.contains(&"bob".to_string()));
    assert!(!solution.has_lgtm_from_requested_reviewer());
}

#[test]
fn test_requires_review_override() {
    let mut solution = Solution::new(
        "S-1".to_string(),
        "Test".to_string(),
        "P-1".to_string(),
    );

    assert!(solution.requires_review.is_none());

    solution.set_requires_review(Some(true));
    assert_eq!(solution.requires_review, Some(true));

    solution.set_requires_review(Some(false));
    assert_eq!(solution.requires_review, Some(false));
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test test_request_reviewers test_lgtm test_requires_review -p jjj`
Expected: FAIL

**Step 3: Add review tracking fields and methods**

Add to `Solution` struct (after `assignee` field):

```rust
    /// Requested reviewers
    #[serde(default)]
    pub requested_reviewers: Vec<String>,

    /// People who have LGTM'd
    #[serde(default)]
    pub reviewed_by: Vec<String>,

    /// Override project default for review requirement
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requires_review: Option<bool>,
```

Update `Solution::new()` to include:

```rust
            requested_reviewers: Vec::new(),
            reviewed_by: Vec::new(),
            requires_review: None,
```

Add methods to `impl Solution`:

```rust
    /// Request review from someone
    pub fn request_review(&mut self, reviewer: String) {
        if !self.requested_reviewers.contains(&reviewer) {
            self.requested_reviewers.push(reviewer);
            self.updated_at = Utc::now();
        }
    }

    /// Record an LGTM from someone
    pub fn add_lgtm(&mut self, reviewer: String) {
        if !self.reviewed_by.contains(&reviewer) {
            self.reviewed_by.push(reviewer);
            self.updated_at = Utc::now();
        }
    }

    /// Check if any requested reviewer has LGTM'd
    pub fn has_lgtm_from_requested_reviewer(&self) -> bool {
        self.reviewed_by.iter().any(|r| self.requested_reviewers.contains(r))
    }

    /// Set the requires_review override
    pub fn set_requires_review(&mut self, requires: Option<bool>) {
        self.requires_review = requires;
        self.updated_at = Utc::now();
    }
```

**Step 4: Run tests to verify they pass**

Run: `cargo test test_request_reviewers test_lgtm test_requires_review -p jjj`
Expected: PASS

**Step 5: Commit**

```bash
git add src/models/solution.rs
git commit -m "feat(solution): add review tracking fields"
```

---

## Task 5: Update SolutionFrontmatter for Review Fields

**Files:**
- Modify: `src/models/solution.rs`

**Step 1: Update SolutionFrontmatter struct**

Add new fields:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolutionFrontmatter {
    pub id: String,
    pub title: String,
    pub problem_id: String,
    pub status: SolutionStatus,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub critique_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub change_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "HashSet::is_empty")]
    pub tags: HashSet<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assignee: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub requested_reviewers: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reviewed_by: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requires_review: Option<bool>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

**Step 2: Update From implementation**

```rust
impl From<&Solution> for SolutionFrontmatter {
    fn from(s: &Solution) -> Self {
        Self {
            id: s.id.clone(),
            title: s.title.clone(),
            problem_id: s.problem_id.clone(),
            status: s.status.clone(),
            critique_ids: s.critique_ids.clone(),
            change_ids: s.change_ids.clone(),
            tags: s.tags.clone(),
            assignee: s.assignee.clone(),
            requested_reviewers: s.requested_reviewers.clone(),
            reviewed_by: s.reviewed_by.clone(),
            requires_review: s.requires_review,
            created_at: s.created_at,
            updated_at: s.updated_at,
        }
    }
}
```

**Step 3: Run all tests**

Run: `cargo test -p jjj`
Expected: All tests pass

**Step 4: Commit**

```bash
git add src/models/solution.rs
git commit -m "feat(solution): update frontmatter for review fields"
```

---

## Task 6: Add Review Config to ProjectConfig

**Files:**
- Modify: `src/models/config.rs`

**Step 1: Add ReviewConfig struct and field**

Add after `Tag` struct:

```rust
/// Review configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ReviewConfig {
    /// Whether solutions require LGTM by default
    #[serde(default)]
    pub default_required: bool,
}
```

Add to `ProjectConfig` struct:

```rust
    /// Review settings
    #[serde(default)]
    pub review: ReviewConfig,
```

**Step 2: Run all tests**

Run: `cargo test -p jjj`
Expected: All tests pass

**Step 3: Commit**

```bash
git add src/models/config.rs
git commit -m "feat(config): add review configuration section"
```

---

## Task 7: Add --file and --line Flags to Critique New Command

**Files:**
- Modify: `src/cli.rs`
- Modify: `src/commands/critique.rs`

**Step 1: Update CritiqueAction::New in cli.rs**

```rust
    /// Add a critique to a solution
    New {
        /// Solution to critique (e.g., S-1)
        solution_id: String,

        /// Critique title
        title: String,

        /// Severity (low, medium, high, critical)
        #[arg(long, default_value = "medium")]
        severity: String,

        /// File path for code-level critique
        #[arg(long)]
        file: Option<String>,

        /// Line number for code-level critique
        #[arg(long)]
        line: Option<usize>,
    },
```

**Step 2: Update execute match in critique.rs**

```rust
CritiqueAction::New {
    solution_id,
    title,
    severity,
    file,
    line,
} => new_critique(solution_id, title, severity, file, line),
```

**Step 3: Update new_critique function signature and body**

```rust
fn new_critique(
    solution_id: String,
    title: String,
    severity_str: String,
    file: Option<String>,
    line: Option<usize>,
) -> Result<()> {
    let jj_client = JjClient::new()?;
    let store = MetadataStore::new(jj_client)?;

    let severity: CritiqueSeverity = severity_str.parse().map_err(|e: String| e)?;
    let solution = store.load_solution(&solution_id)?;

    if solution.is_finalized() {
        println!(
            "Warning: Solution {} is already {:?}. Creating critique anyway.",
            solution_id, solution.status
        );
    }

    store.with_metadata(&format!("Create critique on {}: {}", solution_id, title), || {
        let critique_id = store.next_critique_id()?;
        let mut critique = Critique::new(critique_id.clone(), title.clone(), solution_id.clone());
        critique.set_severity(severity.clone());

        let author = store.jj_client.user_identity()?;
        critique.author = Some(author);

        // Set location if provided
        if let (Some(file_path), Some(line_num)) = (file.clone(), line) {
            // Try to read context from file
            let context = store.jj_client
                .file_at_revision("@", &file_path)
                .ok()
                .map(|content| {
                    let lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
                    let start = line_num.saturating_sub(2);
                    let end = (line_num + 2).min(lines.len());
                    lines[start..end].to_vec()
                })
                .unwrap_or_default();

            critique.set_location(file_path, line_num, None, context);
        }

        store.save_critique(&critique)?;

        let mut solution = store.load_solution(&solution_id)?;
        solution.add_critique(critique_id.clone());
        store.save_solution(&solution)?;

        println!(
            "Created critique {} ({}) on solution {}",
            critique.id, critique.title, solution_id
        );
        println!("  Severity: {}", severity);

        if let Some(ref fp) = file {
            println!("  Location: {}:{}", fp, line.unwrap_or(0));
        }

        Ok(())
    })
}
```

**Step 4: Run build**

Run: `cargo build -p jjj`
Expected: Compiles successfully

**Step 5: Commit**

```bash
git add src/cli.rs src/commands/critique.rs
git commit -m "feat(critique): add --file and --line flags for code-level critiques"
```

---

## Task 8: Add Critique Reply Command

**Files:**
- Modify: `src/cli.rs`
- Modify: `src/commands/critique.rs`

**Step 1: Add Reply variant to CritiqueAction in cli.rs**

Add after `Dismiss` variant:

```rust
    /// Reply to a critique
    Reply {
        /// Critique ID (e.g., CQ-1)
        critique_id: String,

        /// Reply body
        body: String,
    },
```

**Step 2: Update execute match in critique.rs**

Add to the match:

```rust
CritiqueAction::Reply { critique_id, body } => reply_to_critique(critique_id, body),
```

**Step 3: Add reply_to_critique function**

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

**Step 4: Update show_critique to display replies**

Add after showing evidence:

```rust
    // Show replies
    if !critique.replies.is_empty() {
        println!("\n## Discussion ({} replies)", critique.replies.len());
        for reply in &critique.replies {
            println!("\n### {} @ {}", reply.author, reply.created_at.format("%Y-%m-%d %H:%M"));
            println!("{}", reply.body);
        }
    }
```

**Step 5: Run build and test**

Run: `cargo build -p jjj && cargo test -p jjj`
Expected: Success

**Step 6: Commit**

```bash
git add src/cli.rs src/commands/critique.rs
git commit -m "feat(critique): add reply command for threaded discussions"
```

---

## Task 9: Add Solution Review and LGTM Commands

**Files:**
- Modify: `src/cli.rs`
- Modify: `src/commands/solution.rs`

**Step 1: Add Review and Lgtm variants to SolutionAction in cli.rs**

Add after `Assign` variant:

```rust
    /// Request review on a solution
    Review {
        /// Solution ID (e.g., S-1)
        solution_id: String,

        /// Reviewers (e.g., @alice, @bob)
        reviewers: Vec<String>,
    },

    /// LGTM a solution (mark as reviewed)
    Lgtm {
        /// Solution ID (e.g., S-1)
        solution_id: String,
    },
```

**Step 2: Update execute match in solution.rs**

Add imports at top if not present:

```rust
use crate::utils;
```

Add to the match:

```rust
SolutionAction::Review { solution_id, reviewers } => request_review(solution_id, reviewers),
SolutionAction::Lgtm { solution_id } => lgtm_solution(solution_id),
```

**Step 3: Add request_review function**

```rust
fn request_review(solution_id: String, reviewers: Vec<String>) -> Result<()> {
    let jj_client = JjClient::new()?;
    let store = MetadataStore::new(jj_client)?;

    let reviewers: Vec<String> = reviewers.iter().map(|r| utils::parse_mention(r)).collect();

    store.with_metadata(&format!("Request review on {}", solution_id), || {
        let mut solution = store.load_solution(&solution_id)?;

        for reviewer in &reviewers {
            solution.request_review(reviewer.clone());
        }

        store.save_solution(&solution)?;

        println!("✓ Review requested for solution {}", solution_id);
        println!("  Reviewers: {}", reviewers.iter().map(|r| format!("@{}", r)).collect::<Vec<_>>().join(", "));

        Ok(())
    })
}
```

**Step 4: Add lgtm_solution function**

```rust
fn lgtm_solution(solution_id: String) -> Result<()> {
    let jj_client = JjClient::new()?;
    let store = MetadataStore::new(jj_client)?;

    store.with_metadata(&format!("LGTM solution {}", solution_id), || {
        let mut solution = store.load_solution(&solution_id)?;
        let user = store.jj_client.user_identity()?;

        // Check if user is a requested reviewer
        if !solution.requested_reviewers.iter().any(|r| user.contains(r)) {
            println!("Warning: You were not a requested reviewer for this solution.");
            println!("Only LGTMs from requested reviewers count toward acceptance.");
        }

        solution.add_lgtm(user.clone());
        store.save_solution(&solution)?;

        println!("✓ LGTM recorded for solution {}", solution_id);

        Ok(())
    })
}
```

**Step 5: Run build**

Run: `cargo build -p jjj`
Expected: Success

**Step 6: Commit**

```bash
git add src/cli.rs src/commands/solution.rs
git commit -m "feat(solution): add review and lgtm commands"
```

---

## Task 10: Add Top-Level Review and LGTM Shorthand Commands

**Files:**
- Modify: `src/cli.rs`
- Modify: `src/main.rs`

**Step 1: Add shorthand commands to Commands enum in cli.rs**

Add after `Submit`:

```rust
    /// Request review (shorthand for solution review on current change's solution)
    #[command(name = "review")]
    ReviewShorthand {
        /// Reviewers (e.g., @alice, @bob)
        reviewers: Vec<String>,
    },

    /// LGTM (shorthand for solution lgtm on current change's solution)
    #[command(name = "lgtm")]
    LgtmShorthand,
```

**Step 2: Handle shorthand commands in main.rs**

Add handling in the main match (you'll need to look up the solution from current change):

```rust
Commands::ReviewShorthand { reviewers } => {
    // Find solution for current change
    let jj_client = jjj::jj::JjClient::new()?;
    let store = jjj::storage::MetadataStore::new(jj_client.clone())?;
    let change_id = jj_client.current_change_id()?;

    let solutions = store.list_solutions()?;
    let solution = solutions.iter()
        .find(|s| s.change_ids.contains(&change_id))
        .ok_or_else(|| jjj::error::JjjError::Generic(
            format!("No solution found for current change {}. Use 'jjj solution review <id>' instead.", change_id)
        ))?;

    jjj::commands::solution::execute(jjj::cli::SolutionAction::Review {
        solution_id: solution.id.clone(),
        reviewers,
    })
}
Commands::LgtmShorthand => {
    let jj_client = jjj::jj::JjClient::new()?;
    let store = jjj::storage::MetadataStore::new(jj_client.clone())?;
    let change_id = jj_client.current_change_id()?;

    let solutions = store.list_solutions()?;
    let solution = solutions.iter()
        .find(|s| s.change_ids.contains(&change_id))
        .ok_or_else(|| jjj::error::JjjError::Generic(
            format!("No solution found for current change {}. Use 'jjj solution lgtm <id>' instead.", change_id)
        ))?;

    jjj::commands::solution::execute(jjj::cli::SolutionAction::Lgtm {
        solution_id: solution.id.clone(),
    })
}
```

**Step 3: Run build**

Run: `cargo build -p jjj`
Expected: Success

**Step 4: Commit**

```bash
git add src/cli.rs src/main.rs
git commit -m "feat: add review and lgtm shorthand commands"
```

---

## Task 11: Remove Review System

**Files:**
- Delete: `src/models/review.rs`
- Delete: `src/commands/review.rs`
- Modify: `src/models/mod.rs`
- Modify: `src/commands/mod.rs`
- Modify: `src/cli.rs`
- Modify: `src/main.rs`
- Modify: `src/lib.rs`
- Modify: `src/storage.rs`

**Step 1: Remove review module from models/mod.rs**

Remove the line:
```rust
mod review;
```

And remove from pub use:
```rust
pub use review::{Comment, CommentLocation, ReviewManifest, ReviewStatus};
```

**Step 2: Remove review module from commands/mod.rs**

Remove:
```rust
pub mod review;
```

**Step 3: Remove ReviewAction from cli.rs**

Remove the entire `Review` variant from `Commands` enum and the entire `ReviewAction` enum.

**Step 4: Remove review handling from main.rs**

Remove:
```rust
Commands::Review { action } => commands::review::execute(action),
```

**Step 5: Update lib.rs exports**

Remove review-related exports from the `pub use models::` statement.

**Step 6: Remove review functions from storage.rs**

Remove functions: `save_review`, `load_review`, `list_reviews`, `save_comment`, `load_comment`, `list_comments`, `next_comment_id`.

**Step 7: Delete the files**

```bash
rm src/models/review.rs
rm src/commands/review.rs
```

**Step 8: Run build**

Run: `cargo build -p jjj`
Expected: Success (may have some warnings about unused imports to clean up)

**Step 9: Clean up any remaining references**

Fix any compilation errors from lingering references to review types.

**Step 10: Run all tests**

Run: `cargo test -p jjj`
Expected: All tests pass

**Step 11: Commit**

```bash
git add -A
git commit -m "refactor: remove Review system (absorbed by Critique)"
```

---

## Task 12: Update Solution Accept with Critique Blocking

**Files:**
- Modify: `src/commands/solution.rs`
- Modify: `src/cli.rs`

**Step 1: Add --force flag to Accept command in cli.rs**

```rust
    /// Accept solution (requires no open critiques)
    Accept {
        /// Solution ID (e.g., S-1)
        solution_id: String,

        /// Force accept even with open critiques
        #[arg(long)]
        force: bool,
    },
```

**Step 2: Update execute match for Accept**

```rust
SolutionAction::Accept { solution_id, force } => accept_solution(solution_id, force),
```

**Step 3: Rewrite accept_solution function**

```rust
fn accept_solution(solution_id: String, force: bool) -> Result<()> {
    let jj_client = JjClient::new()?;
    let store = MetadataStore::new(jj_client)?;
    let config = store.load_config()?;

    let solution = store.load_solution(&solution_id)?;
    let critiques = store.list_critiques()?;

    // Find open critiques for this solution
    let open_critiques: Vec<_> = critiques
        .iter()
        .filter(|c| c.solution_id == solution_id && c.status == CritiqueStatus::Open)
        .collect();

    // Check critique blocking
    if !open_critiques.is_empty() {
        if !force {
            eprintln!("Error: Cannot accept {} - {} open critique(s):\n", solution_id, open_critiques.len());
            for c in &open_critiques {
                let location = c.file_path.as_ref()
                    .map(|f| format!(" - {}:{}", f, c.line_start.unwrap_or(0)))
                    .unwrap_or_default();
                eprintln!("  {}: {} [{}]{}", c.id, c.title, c.severity, location);
            }
            eprintln!();
            eprintln!("Resolve with: jjj critique address {}", open_critiques[0].id);
            eprintln!("Or dismiss:   jjj critique dismiss {}", open_critiques[0].id);
            eprintln!("Or force:     jjj solution accept {} --force", solution_id);
            return Err(crate::error::JjjError::Generic("Open critiques block acceptance".to_string()));
        }
        eprintln!("Warning: Accepting with {} open critique(s):", open_critiques.len());
        for c in &open_critiques {
            eprintln!("  {}: {} [{}]", c.id, c.title, c.severity);
        }
    }

    // Check review requirement
    let requires_review = solution.requires_review.unwrap_or(config.review.default_required);
    if requires_review {
        if solution.requested_reviewers.is_empty() {
            if !force {
                eprintln!("Error: Solution requires review but no reviewers requested.");
                eprintln!("Request review: jjj solution review {} @reviewer", solution_id);
                eprintln!("Or force:       jjj solution accept {} --force", solution_id);
                return Err(crate::error::JjjError::Generic("No reviewers requested".to_string()));
            }
            eprintln!("Warning: Accepting without requested reviewers.");
        } else if !solution.has_lgtm_from_requested_reviewer() {
            if !force {
                eprintln!("Error: Solution requires LGTM from a requested reviewer.");
                eprintln!("Requested: {}", solution.requested_reviewers.join(", "));
                eprintln!("LGTM'd:    {}", if solution.reviewed_by.is_empty() { "none".to_string() } else { solution.reviewed_by.join(", ") });
                eprintln!("Or force:  jjj solution accept {} --force", solution_id);
                return Err(crate::error::JjjError::Generic("No LGTM from requested reviewer".to_string()));
            }
            eprintln!("Warning: Accepting without LGTM from requested reviewer.");
        }
    }

    store.with_metadata(&format!("Accept solution {}", solution_id), || {
        let mut solution = store.load_solution(&solution_id)?;
        solution.accept();
        store.save_solution(&solution)?;

        let status = if force && !open_critiques.is_empty() {
            "accepted (forced)"
        } else {
            "accepted"
        };
        println!("✓ Solution {} {}", solution_id, status);

        Ok(())
    })
}
```

**Step 4: Run build and test**

Run: `cargo build -p jjj && cargo test -p jjj`
Expected: Success

**Step 5: Commit**

```bash
git add src/cli.rs src/commands/solution.rs
git commit -m "feat(solution): enforce critique blocking on accept with --force escape"
```

---

## Summary

After completing all 12 tasks, the unified critique system will be fully implemented:

1. ✅ Critique has Reply struct for threading
2. ✅ Critique has optional code location fields
3. ✅ CritiqueFrontmatter updated
4. ✅ Solution has review tracking fields
5. ✅ SolutionFrontmatter updated
6. ✅ ProjectConfig has review settings
7. ✅ `jjj critique new --file --line` works
8. ✅ `jjj critique reply` works
9. ✅ `jjj solution review` and `jjj solution lgtm` work
10. ✅ `jjj review` and `jjj lgtm` shorthands work
11. ✅ Review system removed
12. ✅ Solution accept enforces critique blocking

Run final verification:
```bash
cargo build -p jjj
cargo test -p jjj
cargo clippy -p jjj
```
