# Manual Ordering with Quadratic Voting Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use executing-plans to implement this plan task-by-task.

**Goal:** Replace Glicko-2 pairwise ranking with direct manual ordering + Borda count aggregation + quadratic voting, all within the existing TUI tree view.

**Architecture:** Per-user ordering files (`rankings/{milestone_id}/{user}.json`) store ordered problem lists and QV allocations. Borda count + QV boost aggregates multiple users into a global ranking. TUI uses Shift+Up/Down to reorder, +/- for votes, `r` to toggle personal/global views, Shift+Left/Right for tier drilling.

**Tech Stack:** Rust, serde_json, ratatui, crossterm (KeyModifiers::SHIFT)

---

### Task 1: Define ordering data types

**Files:**
- Create: `src/ranking/ordering.rs`
- Modify: `src/ranking/mod.rs`

**Step 1: Write the failing test**

In `src/ranking/ordering.rs`, add the data types and a basic serialization test:

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A single user's ordering and vote allocations for a milestone.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserOrdering {
    /// Problem IDs in priority order (index 0 = highest priority).
    pub order: Vec<String>,
    /// Quadratic vote allocations: problem_id -> number of votes.
    /// Cost of K votes = K^2. Budget = max(100, 2*N) where N = problems in milestone.
    #[serde(default)]
    pub votes: HashMap<String, u32>,
    pub updated_at: DateTime<Utc>,
}

/// Aggregated ranking result for a single problem within a milestone.
#[derive(Debug, Clone)]
pub struct AggregatedRank {
    /// 1-indexed rank position (1 = highest priority).
    pub position: usize,
    /// Total aggregated score (Borda + QV boost).
    pub score: f64,
    /// Number of users who included this problem in their ordering.
    pub voter_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_user_ordering_roundtrip() {
        let ordering = UserOrdering {
            order: vec!["p1".into(), "p2".into(), "p3".into()],
            votes: HashMap::from([("p1".into(), 2), ("p3".into(), 1)]),
            updated_at: Utc::now(),
        };
        let json = serde_json::to_string(&ordering).unwrap();
        let loaded: UserOrdering = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded.order, ordering.order);
        assert_eq!(loaded.votes, ordering.votes);
    }

    #[test]
    fn test_user_ordering_empty_votes_default() {
        let json = r#"{"order":["p1","p2"],"updated_at":"2026-03-22T00:00:00Z"}"#;
        let loaded: UserOrdering = serde_json::from_str(json).unwrap();
        assert!(loaded.votes.is_empty());
    }
}
```

**Step 2: Update module exports**

In `src/ranking/mod.rs`, add:
```rust
pub mod ordering;
```

**Step 3: Run tests**

Run: `cargo test --lib ranking::ordering`
Expected: PASS

**Step 4: Commit**

```bash
git add src/ranking/ordering.rs src/ranking/mod.rs
git commit -m "feat(ranking): add UserOrdering and AggregatedRank types"
```

---

### Task 2: Implement ordering storage (load/save)

**Files:**
- Modify: `src/ranking/ordering.rs`
- Modify: `src/ranking/store.rs`

**Step 1: Write the failing test**

Add to `src/ranking/ordering.rs` tests:

```rust
#[test]
fn test_save_and_load_ordering() {
    let tmp = tempfile::TempDir::new().unwrap();
    let ordering = UserOrdering {
        order: vec!["p1".into(), "p2".into()],
        votes: HashMap::from([("p1".into(), 3)]),
        updated_at: Utc::now(),
    };
    super::save_user_ordering(tmp.path(), "m1", "alice", &ordering).unwrap();
    let loaded = super::load_user_ordering(tmp.path(), "m1", "alice").unwrap();
    assert!(loaded.is_some());
    let loaded = loaded.unwrap();
    assert_eq!(loaded.order, vec!["p1", "p2"]);
    assert_eq!(loaded.votes.get("p1"), Some(&3));
}

#[test]
fn test_load_nonexistent_ordering_returns_none() {
    let tmp = tempfile::TempDir::new().unwrap();
    let loaded = super::load_user_ordering(tmp.path(), "m1", "alice").unwrap();
    assert!(loaded.is_none());
}

#[test]
fn test_load_all_orderings() {
    let tmp = tempfile::TempDir::new().unwrap();
    let o1 = UserOrdering {
        order: vec!["p1".into(), "p2".into()],
        votes: HashMap::new(),
        updated_at: Utc::now(),
    };
    let o2 = UserOrdering {
        order: vec!["p2".into(), "p1".into()],
        votes: HashMap::new(),
        updated_at: Utc::now(),
    };
    super::save_user_ordering(tmp.path(), "m1", "alice", &o1).unwrap();
    super::save_user_ordering(tmp.path(), "m1", "bob", &o2).unwrap();
    let all = super::load_all_orderings(tmp.path(), "m1").unwrap();
    assert_eq!(all.len(), 2);
    assert!(all.contains_key("alice"));
    assert!(all.contains_key("bob"));
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test --lib ranking::ordering`
Expected: FAIL — functions don't exist yet

**Step 3: Implement storage functions**

Add to `src/ranking/ordering.rs`:

```rust
use crate::error::Result;
use std::fs;
use std::path::Path;

const RANKINGS_DIR: &str = "rankings";

/// Save a user's ordering for a milestone.
pub fn save_user_ordering(
    base: &Path,
    milestone_id: &str,
    user: &str,
    ordering: &UserOrdering,
) -> Result<()> {
    let dir = base.join(RANKINGS_DIR).join(milestone_id);
    fs::create_dir_all(&dir)?;
    let file_path = dir.join(format!("{}.json", crate::ranking::store::sanitize_user(user)));
    let json = serde_json::to_string_pretty(ordering)?;
    fs::write(file_path, json)?;
    Ok(())
}

/// Load a single user's ordering for a milestone. Returns None if file doesn't exist.
pub fn load_user_ordering(
    base: &Path,
    milestone_id: &str,
    user: &str,
) -> Result<Option<UserOrdering>> {
    let file_path = base
        .join(RANKINGS_DIR)
        .join(milestone_id)
        .join(format!("{}.json", crate::ranking::store::sanitize_user(user)));
    if !file_path.exists() {
        return Ok(None);
    }
    let contents = fs::read_to_string(&file_path)?;
    let ordering: UserOrdering = serde_json::from_str(&contents)?;
    Ok(Some(ordering))
}

/// Load all users' orderings for a milestone. Returns user_slug -> UserOrdering.
pub fn load_all_orderings(
    base: &Path,
    milestone_id: &str,
) -> Result<HashMap<String, UserOrdering>> {
    let dir = base.join(RANKINGS_DIR).join(milestone_id);
    let mut result = HashMap::new();
    if !dir.exists() {
        return Ok(result);
    }
    for entry in fs::read_dir(&dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let user_slug = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();
        let contents = fs::read_to_string(&path)?;
        let ordering: UserOrdering = serde_json::from_str(&contents)?;
        result.insert(user_slug, ordering);
    }
    Ok(result)
}
```

**Step 4: Run tests**

Run: `cargo test --lib ranking::ordering`
Expected: PASS

**Step 5: Commit**

```bash
git add src/ranking/ordering.rs
git commit -m "feat(ranking): implement ordering load/save storage"
```

---

### Task 3: Implement Borda count + QV aggregation

**Files:**
- Create: `src/ranking/borda.rs`
- Modify: `src/ranking/mod.rs`

**Step 1: Write the failing tests**

Create `src/ranking/borda.rs`:

```rust
use std::collections::HashMap;
use super::ordering::{AggregatedRank, UserOrdering};

/// Aggregate multiple user orderings into a global ranking using Borda count + QV boost.
///
/// - Borda: rank 1 gets N points, rank 2 gets N-1, ..., rank N gets 1.
/// - Owner weighting: if `owner_slug` matches a user, their Borda scores get 2x weight.
/// - QV boost: each vote K on an item adds K to the aggregated score.
/// - Budget: max(100, 2*problem_count) per user. Over-budget votes are silently capped.
/// - Ties broken by problem_id (lexicographic) for determinism.
pub fn aggregate_rankings(
    orderings: &HashMap<String, UserOrdering>,
    owner_slug: Option<&str>,
    problem_count: usize,
) -> Vec<(String, AggregatedRank)> {
    todo!()
}

/// Calculate the QV budget for a milestone with N problems.
pub fn qv_budget(problem_count: usize) -> u32 {
    (100u32).max(2 * problem_count as u32)
}

/// Calculate the total cost of K votes (K^2).
pub fn vote_cost(votes: u32) -> u32 {
    votes * votes
}

/// Calculate the total cost of all a user's vote allocations.
pub fn total_vote_cost(votes: &HashMap<String, u32>) -> u32 {
    votes.values().map(|&k| vote_cost(k)).sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn make_ordering(order: Vec<&str>, votes: Vec<(&str, u32)>) -> UserOrdering {
        UserOrdering {
            order: order.into_iter().map(String::from).collect(),
            votes: votes.into_iter().map(|(k, v)| (k.to_string(), v)).collect(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn test_single_user_borda() {
        let mut orderings = HashMap::new();
        orderings.insert("alice".to_string(), make_ordering(vec!["p1", "p2", "p3"], vec![]));
        let result = aggregate_rankings(&orderings, None, 3);
        // p1 gets 3 points, p2 gets 2, p3 gets 1
        assert_eq!(result[0].0, "p1");
        assert_eq!(result[0].1.position, 1);
        assert_eq!(result[1].0, "p2");
        assert_eq!(result[1].1.position, 2);
        assert_eq!(result[2].0, "p3");
        assert_eq!(result[2].1.position, 3);
    }

    #[test]
    fn test_two_users_equal_weight() {
        let mut orderings = HashMap::new();
        orderings.insert("alice".to_string(), make_ordering(vec!["p1", "p2", "p3"], vec![]));
        orderings.insert("bob".to_string(), make_ordering(vec!["p3", "p2", "p1"], vec![]));
        let result = aggregate_rankings(&orderings, None, 3);
        // p1: 3+1=4, p2: 2+2=4, p3: 1+3=4 — all tied, break by ID
        assert_eq!(result[0].0, "p1"); // lexicographic
        assert_eq!(result[1].0, "p2");
        assert_eq!(result[2].0, "p3");
    }

    #[test]
    fn test_owner_gets_2x_weight() {
        let mut orderings = HashMap::new();
        // Alice (owner): p2 first. Bob: p1 first.
        orderings.insert("alice".to_string(), make_ordering(vec!["p2", "p1"], vec![]));
        orderings.insert("bob".to_string(), make_ordering(vec!["p1", "p2"], vec![]));
        let result = aggregate_rankings(&orderings, Some("alice"), 2);
        // p1: alice=1*2 + bob=2 = 4. p2: alice=2*2 + bob=1 = 5. p2 wins.
        assert_eq!(result[0].0, "p2");
        assert_eq!(result[1].0, "p1");
    }

    #[test]
    fn test_qv_boost() {
        let mut orderings = HashMap::new();
        // p1 is ranked lower but has QV boost
        orderings.insert("alice".to_string(), make_ordering(vec!["p2", "p1"], vec![("p1", 3)]));
        let result = aggregate_rankings(&orderings, None, 2);
        // p2: borda=2. p1: borda=1 + qv=3 = 4. p1 wins.
        assert_eq!(result[0].0, "p1");
    }

    #[test]
    fn test_qv_budget() {
        assert_eq!(qv_budget(10), 100); // max(100, 20) = 100
        assert_eq!(qv_budget(60), 120); // max(100, 120) = 120
    }

    #[test]
    fn test_vote_cost() {
        assert_eq!(vote_cost(0), 0);
        assert_eq!(vote_cost(1), 1);
        assert_eq!(vote_cost(2), 4);
        assert_eq!(vote_cost(3), 9);
    }

    #[test]
    fn test_total_vote_cost() {
        let votes = HashMap::from([("p1".to_string(), 2u32), ("p2".to_string(), 3)]);
        assert_eq!(total_vote_cost(&votes), 4 + 9); // 13
    }

    #[test]
    fn test_voter_count() {
        let mut orderings = HashMap::new();
        orderings.insert("alice".to_string(), make_ordering(vec!["p1", "p2"], vec![]));
        orderings.insert("bob".to_string(), make_ordering(vec!["p1"], vec![]));
        let result = aggregate_rankings(&orderings, None, 2);
        let p1 = result.iter().find(|(id, _)| id == "p1").unwrap();
        assert_eq!(p1.1.voter_count, 2);
        let p2 = result.iter().find(|(id, _)| id == "p2").unwrap();
        assert_eq!(p2.1.voter_count, 1);
    }

    #[test]
    fn test_empty_orderings() {
        let orderings = HashMap::new();
        let result = aggregate_rankings(&orderings, None, 0);
        assert!(result.is_empty());
    }
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test --lib ranking::borda`
Expected: FAIL — `todo!()` panics

**Step 3: Implement aggregate_rankings**

Replace `todo!()` with:

```rust
pub fn aggregate_rankings(
    orderings: &HashMap<String, UserOrdering>,
    owner_slug: Option<&str>,
    problem_count: usize,
) -> Vec<(String, AggregatedRank)> {
    if orderings.is_empty() {
        return Vec::new();
    }

    let budget = qv_budget(problem_count);
    let mut scores: HashMap<String, (f64, usize)> = HashMap::new(); // (score, voter_count)

    for (user_slug, ordering) in orderings {
        let weight = if owner_slug.is_some_and(|o| o == user_slug) {
            2.0
        } else {
            1.0
        };

        let n = ordering.order.len();
        for (i, problem_id) in ordering.order.iter().enumerate() {
            let borda_points = (n - i) as f64 * weight;
            let entry = scores.entry(problem_id.clone()).or_insert((0.0, 0));
            entry.0 += borda_points;
            entry.1 += 1;
        }

        // Add QV boost (capped to budget)
        let mut remaining_budget = budget;
        for (problem_id, &vote_count) in &ordering.votes {
            let cost = vote_cost(vote_count);
            if cost <= remaining_budget {
                let entry = scores.entry(problem_id.clone()).or_insert((0.0, 0));
                entry.0 += vote_count as f64;
                remaining_budget -= cost;
            }
        }
    }

    let mut sorted: Vec<(String, (f64, usize))> = scores.into_iter().collect();
    sorted.sort_by(|a, b| {
        b.1 .0
            .partial_cmp(&a.1 .0)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.0.cmp(&b.0)) // tie-break by ID
    });

    sorted
        .into_iter()
        .enumerate()
        .map(|(i, (problem_id, (score, voter_count)))| {
            (
                problem_id,
                AggregatedRank {
                    position: i + 1,
                    score,
                    voter_count,
                },
            )
        })
        .collect()
}
```

**Step 4: Update mod.rs**

Add to `src/ranking/mod.rs`:
```rust
pub mod borda;
```

**Step 5: Run tests**

Run: `cargo test --lib ranking::borda`
Expected: PASS

**Step 6: Commit**

```bash
git add src/ranking/borda.rs src/ranking/mod.rs
git commit -m "feat(ranking): implement Borda count + quadratic voting aggregation"
```

---

### Task 4: Wire aggregation into ProjectData

**Files:**
- Modify: `src/tui/app/mod.rs`

**Step 1: Update imports**

Replace:
```rust
use crate::ranking::glicko2::{self, WeightedComparison};
use crate::ranking::store as ranking_store;
```
With:
```rust
use crate::ranking::borda;
use crate::ranking::ordering;
use crate::ranking::store as ranking_store;
```

**Step 2: Rewrite compute_rankings**

Replace the existing `compute_rankings` method in `ProjectData` (lines 148–203 of `src/tui/app/mod.rs`) with:

```rust
/// Compute aggregated rankings per milestone using Borda count + QV.
/// Returns milestone_id -> problem_id -> (rank_position, voter_count_str).
fn compute_rankings(
    store: &MetadataStore,
    milestones: &[Milestone],
) -> HashMap<String, HashMap<String, (usize, String)>> {
    let mut result = HashMap::new();
    let base = store.meta_path();

    for milestone in milestones {
        let orderings = match ordering::load_all_orderings(base, &milestone.id) {
            Ok(o) => o,
            Err(_) => continue,
        };
        if orderings.is_empty() {
            continue;
        }

        let owner_slug = milestone
            .assignee
            .as_deref()
            .map(ranking_store::sanitize_user);

        let problem_count = milestones
            .iter()
            .flat_map(|_| std::iter::empty::<()>()) // placeholder
            .count();
        // Actually count problems in this milestone — but we don't have problems here.
        // Use the max order length as a proxy.
        let problem_count = orderings.values().map(|o| o.order.len()).max().unwrap_or(0);

        let aggregated = borda::aggregate_rankings(
            &orderings,
            owner_slug.as_deref(),
            problem_count,
        );

        let mut milestone_rankings = HashMap::new();
        for (problem_id, rank) in &aggregated {
            milestone_rankings.insert(
                problem_id.clone(),
                (rank.position, format!("{}", rank.voter_count)),
            );
        }
        result.insert(milestone.id.clone(), milestone_rankings);
    }

    result
}
```

**Step 3: Remove unused imports**

Remove the old `WeightedComparison` import if no longer used elsewhere in the file. The `glicko2` module reference may still be needed temporarily for `InputMode::Ranking` — we'll clean that up in Task 8.

**Step 4: Run check**

Run: `cargo check`
Expected: PASS (or warnings about unused imports)

**Step 5: Run tests**

Run: `cargo test`
Expected: PASS

**Step 6: Commit**

```bash
git add src/tui/app/mod.rs
git commit -m "feat(ranking): wire Borda+QV aggregation into ProjectData"
```

---

### Task 5: Add personal ordering state to UiState

**Files:**
- Modify: `src/tui/app/mod.rs`

**Step 1: Add fields to UiState**

Add to `UiState` struct:
```rust
/// Whether to show personal ordering (true) or global aggregated view (false).
pub show_personal_ordering: bool,
/// The current user's personal orderings per milestone (loaded on startup).
/// milestone_id -> UserOrdering
pub personal_orderings: HashMap<String, crate::ranking::ordering::UserOrdering>,
/// Tier drilling state: stack of (milestone_id, start_index, end_index).
/// Empty = showing all items. Each entry narrows to a third of the parent range.
pub tier_drill: Vec<(String, usize, usize)>,
```

**Step 2: Initialize in UiState::new()**

Add defaults:
```rust
show_personal_ordering: true,
personal_orderings: HashMap::new(),
tier_drill: Vec::new(),
```

**Step 3: Load personal orderings in App::new()**

After `let data = ProjectData::load(&store)?;`, add loading the current user's orderings:

```rust
// Load personal orderings for the current user
let mut personal_orderings = HashMap::new();
for milestone in &data.milestones {
    if let Ok(Some(ordering)) = ordering::load_user_ordering(
        store.meta_path(),
        &milestone.id,
        &user,
    ) {
        personal_orderings.insert(milestone.id.clone(), ordering);
    }
}
ui.personal_orderings = personal_orderings;
```

**Step 4: Run check**

Run: `cargo check`
Expected: PASS

**Step 5: Commit**

```bash
git add src/tui/app/mod.rs
git commit -m "feat(tui): add personal ordering state to UiState"
```

---

### Task 6: Implement Shift+Up/Down reordering

**Files:**
- Modify: `src/tui/app/mod.rs` (key handler)
- Modify: `src/tui/app/actions.rs` (new action methods)

**Step 1: Add modifier detection in handle_normal_key**

In `handle_normal_key`, add Shift+Arrow handling before the existing arrow handling:

```rust
// Before existing KeyCode::Up/Down matches:
KeyCode::Up if key.modifiers.contains(KeyModifiers::SHIFT) => {
    self.move_problem_up()?;
}
KeyCode::Down if key.modifiers.contains(KeyModifiers::SHIFT) => {
    self.move_problem_down()?;
}
```

Note: The `if` guard on existing match arms needs to be added — the existing `KeyCode::Up => self.navigate_up()` should NOT trigger when Shift is held. Restructure the match so Shift+arrows are checked first.

**Step 2: Implement move_problem_up/down in actions.rs**

Add to `src/tui/app/actions.rs`:

```rust
/// Move the selected problem up in the user's personal ordering for its milestone.
pub(super) fn move_problem_up(&mut self) -> Result<()> {
    let (milestone_id, problem_id) = match self.selected_milestone_problem() {
        Some(x) => x,
        None => return Ok(()),
    };

    let ordering = self.ui.personal_orderings
        .entry(milestone_id.clone())
        .or_insert_with(|| {
            self.default_ordering_for_milestone(&milestone_id)
        });

    if let Some(pos) = ordering.order.iter().position(|id| *id == problem_id) {
        if pos > 0 {
            ordering.order.swap(pos, pos - 1);
            ordering.updated_at = chrono::Utc::now();
            ordering::save_user_ordering(
                self.store.meta_path(),
                &milestone_id,
                &self.user,
                ordering,
            )?;
            self.refresh_data()?;
            // Move cursor up too so it follows the item
            self.navigate_up();
        }
    }
    Ok(())
}

/// Move the selected problem down in the user's personal ordering.
pub(super) fn move_problem_down(&mut self) -> Result<()> {
    let (milestone_id, problem_id) = match self.selected_milestone_problem() {
        Some(x) => x,
        None => return Ok(()),
    };

    let ordering = self.ui.personal_orderings
        .entry(milestone_id.clone())
        .or_insert_with(|| {
            self.default_ordering_for_milestone(&milestone_id)
        });

    if let Some(pos) = ordering.order.iter().position(|id| *id == problem_id) {
        if pos + 1 < ordering.order.len() {
            ordering.order.swap(pos, pos + 1);
            ordering.updated_at = chrono::Utc::now();
            ordering::save_user_ordering(
                self.store.meta_path(),
                &milestone_id,
                &self.user,
                ordering,
            )?;
            self.refresh_data()?;
            // Move cursor down too
            self.navigate_down();
        }
    }
    Ok(())
}

/// Get (milestone_id, problem_id) if the selected tree item is a problem under a milestone.
fn selected_milestone_problem(&self) -> Option<(String, String)> {
    let item = self.cache.tree_items.get(self.ui.tree_index)?;
    let problem_id = match &item.node {
        super::super::tree::TreeNode::Problem { id, .. } => id.clone(),
        _ => return None,
    };

    // Find the milestone this problem belongs to
    let problem = self.data.problems.iter().find(|p| p.id == problem_id)?;
    let milestone_id = problem.milestone_id.clone()?;
    Some((milestone_id, problem_id))
}

/// Create a default ordering for a milestone from current problem order.
fn default_ordering_for_milestone(&self, milestone_id: &str) -> ordering::UserOrdering {
    let order: Vec<String> = self.data.problems
        .iter()
        .filter(|p| p.milestone_id.as_deref() == Some(milestone_id))
        .map(|p| p.id.clone())
        .collect();
    ordering::UserOrdering {
        order,
        votes: std::collections::HashMap::new(),
        updated_at: chrono::Utc::now(),
    }
}
```

**Step 3: Update tree building to sort by personal ordering**

In `src/tui/tree.rs`, modify `add_problems` to accept an optional ordering and sort problems accordingly. Or, sort problems before passing to `build_flat_tree_ranked` in `refresh_data`.

The simpler approach: in `App::refresh_data()`, when building tree items, pass the personal ordering to the tree builder so it sorts problems within milestones. Add an `order` parameter to `build_flat_tree_ranked`:

Actually, the simpler approach is to sort the `problems` slice before it reaches `add_problems`. In `build_flat_tree_ranked`, after filtering `milestone_problems`, sort them by the ordering if available.

Add a parameter `personal_order: Option<&[String]>` to `build_flat_tree_ranked` (or pass the full orderings map). Then in the milestone loop:

```rust
// Sort milestone_problems by personal ordering if available
if let Some(order) = personal_order_map.get(&milestone.id) {
    milestone_problems.sort_by_key(|p| {
        order.iter().position(|id| id == &p.id).unwrap_or(usize::MAX)
    });
}
```

**Step 4: Run check and test**

Run: `cargo check && cargo test`
Expected: PASS

**Step 5: Commit**

```bash
git add src/tui/app/mod.rs src/tui/app/actions.rs src/tui/tree.rs
git commit -m "feat(tui): add Shift+Up/Down for manual problem reordering"
```

---

### Task 7: Implement +/- quadratic voting

**Files:**
- Modify: `src/tui/app/mod.rs` (key handler)
- Modify: `src/tui/app/actions.rs` (vote methods)

**Step 1: Add key handlers**

In `handle_normal_key`:
```rust
KeyCode::Char('+') | KeyCode::Char('=') => self.add_vote()?,
KeyCode::Char('-') => self.remove_vote()?,
```

**Step 2: Implement add_vote/remove_vote in actions.rs**

```rust
/// Add a quadratic vote to the selected problem.
pub(super) fn add_vote(&mut self) -> Result<()> {
    let (milestone_id, problem_id) = match self.selected_milestone_problem() {
        Some(x) => x,
        None => return Ok(()),
    };

    let problem_count = self.data.problems
        .iter()
        .filter(|p| p.milestone_id.as_deref() == Some(&milestone_id))
        .count();
    let budget = borda::qv_budget(problem_count);

    let ordering = self.ui.personal_orderings
        .entry(milestone_id.clone())
        .or_insert_with(|| self.default_ordering_for_milestone(&milestone_id));

    let current_cost = borda::total_vote_cost(&ordering.votes);
    let current_votes = *ordering.votes.get(&problem_id).unwrap_or(&0);
    let next_cost = borda::vote_cost(current_votes + 1);
    let marginal = next_cost - borda::vote_cost(current_votes);

    if current_cost + marginal > budget {
        self.show_flash(&format!("No budget remaining ({}/{})", current_cost, budget));
        return Ok(());
    }

    *ordering.votes.entry(problem_id.clone()).or_insert(0) += 1;
    ordering.updated_at = chrono::Utc::now();

    ordering::save_user_ordering(
        self.store.meta_path(),
        &milestone_id,
        &self.user,
        ordering,
    )?;
    self.refresh_data()?;

    let new_votes = ordering.votes.get(&problem_id).copied().unwrap_or(0);
    let new_total = borda::total_vote_cost(&ordering.votes);
    self.show_flash(&format!("Vote added ({}★, budget {}/{})", new_votes, new_total, budget));
    Ok(())
}

/// Remove a quadratic vote from the selected problem.
pub(super) fn remove_vote(&mut self) -> Result<()> {
    let (milestone_id, problem_id) = match self.selected_milestone_problem() {
        Some(x) => x,
        None => return Ok(()),
    };

    let ordering = match self.ui.personal_orderings.get_mut(&milestone_id) {
        Some(o) => o,
        None => return Ok(()),
    };

    let current_votes = *ordering.votes.get(&problem_id).unwrap_or(&0);
    if current_votes == 0 {
        return Ok(());
    }

    if current_votes == 1 {
        ordering.votes.remove(&problem_id);
    } else {
        *ordering.votes.get_mut(&problem_id).unwrap() -= 1;
    }
    ordering.updated_at = chrono::Utc::now();

    ordering::save_user_ordering(
        self.store.meta_path(),
        &milestone_id,
        &self.user,
        ordering,
    )?;
    self.refresh_data()?;
    Ok(())
}
```

**Step 3: Run check and test**

Run: `cargo check && cargo test`
Expected: PASS

**Step 4: Commit**

```bash
git add src/tui/app/mod.rs src/tui/app/actions.rs
git commit -m "feat(tui): add +/- keys for quadratic voting"
```

---

### Task 8: Implement `r` toggle for personal/global view

**Files:**
- Modify: `src/tui/app/mod.rs` (key handler)
- Modify: `src/tui/app/actions.rs` (toggle method)
- Modify: `src/tui/ui.rs` (status bar display)

**Step 1: Add key handler**

In `handle_normal_key`, replace the old `r` binding:
```rust
// OLD: KeyCode::Char('r') => self.start_ranking()?,
// NEW:
KeyCode::Char('r') => self.toggle_ordering_view(),
```

**Step 2: Implement toggle**

In `actions.rs`:
```rust
pub(super) fn toggle_ordering_view(&mut self) {
    self.ui.show_personal_ordering = !self.ui.show_personal_ordering;
    self.refresh_data().ok();
    let view = if self.ui.show_personal_ordering { "Personal" } else { "Global" };
    self.show_flash(&format!("Showing {} ordering", view));
}
```

**Step 3: Update tree building**

Modify `refresh_data()` to pass either personal or global ordering to the tree builder depending on `show_personal_ordering`.

**Step 4: Add status bar indicator in ui.rs**

In the status bar area of `draw()`, add an indicator showing `[Personal | Budget: X/Y]` or `[Global | N voters]`.

**Step 5: Run check and test**

Run: `cargo check && cargo test`
Expected: PASS

**Step 6: Commit**

```bash
git add src/tui/app/mod.rs src/tui/app/actions.rs src/tui/ui.rs
git commit -m "feat(tui): add G key to toggle personal/global ordering view"
```

---

### Task 9: Implement tier drilling (Shift+Left/Right)

**Files:**
- Modify: `src/tui/app/mod.rs` (key handler)
- Modify: `src/tui/app/actions.rs` (drill methods)
- Modify: `src/tui/tree.rs` (filtered view)
- Modify: `src/tui/ui.rs` (breadcrumb display)

**Step 1: Add key handlers**

In `handle_normal_key`:
```rust
KeyCode::Left if key.modifiers.contains(KeyModifiers::SHIFT) => {
    self.tier_drill_out();
}
KeyCode::Right if key.modifiers.contains(KeyModifiers::SHIFT) => {
    self.tier_drill_in()?;
}
```

**Step 2: Implement tier drilling**

In `actions.rs`:

```rust
/// Drill into the tier (top/mid/bottom third) containing the selected problem.
pub(super) fn tier_drill_in(&mut self) -> Result<()> {
    let (milestone_id, problem_id) = match self.selected_milestone_problem() {
        Some(x) => x,
        None => return Ok(()),
    };

    // Get the current visible range for this milestone
    let (start, end) = if let Some(last) = self.ui.tier_drill.last() {
        if last.0 == milestone_id {
            (last.1, last.2)
        } else {
            // Different milestone, reset
            self.ui.tier_drill.clear();
            let count = self.milestone_problem_count(&milestone_id);
            (0, count)
        }
    } else {
        let count = self.milestone_problem_count(&milestone_id);
        (0, count)
    };

    let range_size = end - start;
    if range_size <= 3 {
        // Too small to drill further
        return Ok(());
    }

    // Find which third the selected problem is in
    let ordering = self.get_effective_ordering(&milestone_id);
    let pos_in_ordering = ordering.iter().position(|id| *id == problem_id).unwrap_or(0);
    let pos_in_range = pos_in_ordering.saturating_sub(start);

    let third = range_size / 3;
    let (new_start, new_end) = if pos_in_range < third {
        (start, start + third) // Top third
    } else if pos_in_range < 2 * third {
        (start + third, start + 2 * third) // Mid third
    } else {
        (start + 2 * third, end) // Bottom third
    };

    self.ui.tier_drill.push((milestone_id, new_start, new_end));
    self.refresh_data()?;
    Ok(())
}

/// Zoom out one tier level.
pub(super) fn tier_drill_out(&mut self) {
    if self.ui.tier_drill.pop().is_some() {
        self.refresh_data().ok();
    }
}

fn milestone_problem_count(&self, milestone_id: &str) -> usize {
    self.data.problems
        .iter()
        .filter(|p| p.milestone_id.as_deref() == Some(milestone_id))
        .count()
}

/// Get the effective ordering (personal or global) for a milestone.
fn get_effective_ordering(&self, milestone_id: &str) -> Vec<String> {
    if self.ui.show_personal_ordering {
        self.ui.personal_orderings
            .get(milestone_id)
            .map(|o| o.order.clone())
            .unwrap_or_default()
    } else {
        // From global rankings, sort by position
        self.data.rankings
            .get(milestone_id)
            .map(|m| {
                let mut items: Vec<_> = m.iter().collect();
                items.sort_by_key(|(_, (pos, _))| *pos);
                items.into_iter().map(|(id, _)| id.clone()).collect()
            })
            .unwrap_or_default()
    }
}
```

**Step 3: Filter tree items by tier drill**

In `build_flat_tree_ranked` or in `refresh_data`, when `tier_drill` is non-empty, filter `milestone_problems` to only show problems in the drill range.

**Step 4: Add breadcrumb in ui.rs**

Display the tier drill path (e.g., "Top > Mid") in the milestone header or status bar.

**Step 5: Run check and test**

Run: `cargo check && cargo test`
Expected: PASS

**Step 6: Commit**

```bash
git add src/tui/app/mod.rs src/tui/app/actions.rs src/tui/tree.rs src/tui/ui.rs
git commit -m "feat(tui): add Shift+Left/Right for recursive tier drilling"
```

---

### Task 10: Update tree display (rank prefix, vote stars)

**Files:**
- Modify: `src/tui/ui.rs` (tree rendering)
- Modify: `src/tui/tree.rs` (add votes field to TreeNode::Problem)

**Step 1: Add votes field to TreeNode::Problem**

```rust
Problem {
    id: String,
    title: String,
    status: ProblemStatus,
    priority: Priority,
    assignee: Option<String>,
    expanded: bool,
    rank: Option<usize>,
    votes: u32,  // NEW: user's QV vote count for this problem
}
```

**Step 2: Populate votes in add_problems**

Pass user's vote data through to the tree builder. In `add_problems`, look up votes from the personal ordering.

**Step 3: Render vote stars in ui.rs**

After the problem title, append stars: `"★".repeat(votes as usize)` in a yellow/gold color.

**Step 4: Update rank prefix**

The rank prefix `#N` is already rendered. Ensure it uses the correct ordering (personal vs global).

**Step 5: Run check and test**

Run: `cargo check && cargo test`
Expected: PASS

**Step 6: Commit**

```bash
git add src/tui/tree.rs src/tui/ui.rs
git commit -m "feat(tui): show rank numbers and vote stars in tree"
```

---

### Task 11: Update help overlay

**Files:**
- Modify: `src/tui/ui.rs`

**Step 1: Add new keybindings to help overlay**

Find the help overlay section and add entries for:
- `Shift+↑/↓` — Reorder problem
- `Shift+←/→` — Tier drill in/out
- `+/-` — Add/remove vote
- `G` — Toggle personal/global view

**Step 2: Run check**

Run: `cargo check`
Expected: PASS

**Step 3: Commit**

```bash
git add src/tui/ui.rs
git commit -m "feat(tui): update help overlay with ordering keybindings"
```

---

### Task 12: Update `jjj rank show` CLI command

**Files:**
- Modify: `src/commands/rank.rs`

**Step 1: Update show() to use Borda aggregation**

Replace the Glicko-2 rating display with Borda+QV aggregated scores. Show:
- Rank position
- Problem title
- Aggregated score
- Voter count

Keep the `--json` and `--by-user` flags working.

**Step 2: Update show_by_user()**

Instead of per-user Glicko ratings, show each user's ordering and vote allocations.

**Step 3: Run test**

Run: `cargo test --test ranking_workflow`
Expected: May need updates — see Task 14.

**Step 4: Commit**

```bash
git add src/commands/rank.rs
git commit -m "feat(cli): update rank show for Borda+QV aggregation"
```

---

### Task 13: Delete Glicko-2 and pairwise ranking code

**Files:**
- Delete: `src/ranking/glicko2.rs`
- Delete: `src/ranking/matchups.rs`
- Modify: `src/ranking/mod.rs` (remove old module exports)
- Modify: `src/tui/app/mod.rs` (remove InputMode::Ranking, RankingProblem)
- Modify: `src/tui/app/actions.rs` (remove start_ranking, handle_ranking_key, ranking_problem_from_id)
- Modify: `src/tui/ui.rs` (remove draw_ranking_overlay)
- Modify: `src/commands/rank.rs` (remove session subcommand)
- Modify: `src/cli.rs` (remove Session variant from RankAction, or keep as error)

**Step 1: Remove module exports**

In `src/ranking/mod.rs`, remove:
```rust
pub mod glicko2;
pub mod matchups;
```

**Step 2: Delete files**

```bash
rm src/ranking/glicko2.rs src/ranking/matchups.rs
```

**Step 3: Remove InputMode::Ranking**

In `src/tui/app/mod.rs`:
- Remove the `Ranking { .. }` variant from `InputMode`
- Remove `RankingProblem` struct
- Remove `handle_ranking_key` dispatch from `handle_key`

**Step 4: Remove ranking actions**

In `src/tui/app/actions.rs`:
- Remove `start_ranking()`, `handle_ranking_key()`, `ranking_problem_from_id()`
- Remove the 'r' key binding from `handle_normal_key`

**Step 5: Remove ranking overlay**

In `src/tui/ui.rs`:
- Remove `draw_ranking_overlay()` function
- Remove call site in `draw()` that renders the ranking comparison UI

**Step 6: Remove CLI session**

In `src/commands/rank.rs`:
- Remove `session()` function
- In `execute()`, either remove the `Session` dispatch or show an error

In `src/cli.rs`:
- Remove `Session` variant from `RankAction` enum, or keep it and show deprecation message

**Step 7: Fix all compilation errors**

Run `cargo check` repeatedly and fix all references to deleted code.

**Step 8: Run tests**

Run: `cargo test`
Expected: Some test failures from deleted types — fix in next task.

**Step 9: Commit**

```bash
git add -A
git commit -m "refactor: remove Glicko-2 pairwise ranking system"
```

---

### Task 14: Update tests

**Files:**
- Modify: `tests/ranking_workflow.rs`
- Modify: `src/ranking/store.rs` (tests reference Comparison from glicko2)

**Step 1: Update store.rs tests**

The `store.rs` tests import `Comparison` from `glicko2`. Either:
- Move `Comparison` to a shared location (e.g., `ordering.rs`)
- Or delete the old JSONL-based tests and add new JSON-based tests
- Or keep `Comparison` struct in store.rs for backward compat

Best approach: Keep `Comparison` in `store.rs` (it's just a data type) and keep the old storage functions for reading legacy JSONL files. This preserves backward compatibility.

**Step 2: Update ranking_workflow.rs**

Rewrite the integration test to:
1. Create two `UserOrdering` files
2. Aggregate via Borda count
3. Verify correct ranking

**Step 3: Run tests**

Run: `cargo test`
Expected: PASS

**Step 4: Commit**

```bash
git add -A
git commit -m "test: update ranking tests for Borda+QV system"
```

---

### Task 15: Update detail pane for ordering context

**Files:**
- Modify: `src/tui/detail.rs`

**Step 1: Show rank and votes in problem detail**

In `problem_lines()`, add a line showing:
- Current rank position (from rankings map)
- Vote allocation (from personal ordering)
- Budget remaining

**Step 2: Run check**

Run: `cargo check`
Expected: PASS

**Step 3: Commit**

```bash
git add src/tui/detail.rs
git commit -m "feat(tui): show rank and vote info in problem detail pane"
```

---

### Task 16: Final integration check

**Step 1: Full test suite**

Run: `cargo test`
Expected: All tests pass

**Step 2: Clippy**

Run: `cargo clippy -- -D warnings`
Expected: No warnings

**Step 3: Format**

Run: `cargo fmt`

**Step 4: Final commit if needed**

```bash
git add -A
git commit -m "chore: fix clippy warnings and formatting"
```
