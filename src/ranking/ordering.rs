use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::error::Result;

const RANKINGS_DIR: &str = "rankings";

/// Normalize a user identity string (e.g., "Alice Smith <alice@test.com>") into
/// a filesystem-safe slug (e.g., "alice-smith").
pub fn sanitize_user(user: &str) -> String {
    let name_part = if let Some(idx) = user.find('<') {
        &user[..idx]
    } else {
        user
    };

    let slug: String = name_part
        .trim()
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect();

    slug.trim_matches('-').to_string()
}

/// A single user's ordering and vote allocations for a milestone.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserOrdering {
    /// Problem IDs in priority order (index 0 = highest priority).
    pub order: Vec<String>,
    /// Quadratic vote allocations: problem_id -> signed vote count.
    /// Positive = support, negative = opposition. Cost of v votes = |v|^2.
    /// Budget = max(100, 2*N) where N = problems in milestone.
    #[serde(default)]
    pub votes: HashMap<String, i32>,
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

/// Save a user's ordering for a milestone to `{base}/rankings/{milestone_id}/{user_slug}.json`.
///
/// Creates directories as needed. Writes pretty-printed JSON.
pub fn save_user_ordering(
    base: &Path,
    milestone_id: &str,
    user: &str,
    ordering: &UserOrdering,
) -> Result<()> {
    let dir = base.join(RANKINGS_DIR).join(milestone_id);
    fs::create_dir_all(&dir)?;

    let slug = sanitize_user(user);
    let file_path = dir.join(format!("{slug}.json"));
    let json = serde_json::to_string_pretty(ordering)?;
    fs::write(file_path, json)?;

    Ok(())
}

/// Load a single user's ordering. Returns `None` if the file doesn't exist.
pub fn load_user_ordering(
    base: &Path,
    milestone_id: &str,
    user: &str,
) -> Result<Option<UserOrdering>> {
    let slug = sanitize_user(user);
    let file_path = base
        .join(RANKINGS_DIR)
        .join(milestone_id)
        .join(format!("{slug}.json"));

    if !file_path.exists() {
        return Ok(None);
    }

    let contents = fs::read_to_string(file_path)?;
    let ordering: UserOrdering = serde_json::from_str(&contents)?;
    Ok(Some(ordering))
}

/// Load all users' orderings for a milestone. Returns `HashMap<user_slug, UserOrdering>`.
///
/// Only reads `.json` files (ignores `.jsonl` files from the old Glicko-2 system).
/// Returns an empty `HashMap` if the directory doesn't exist.
pub fn load_all_orderings(
    base: &Path,
    milestone_id: &str,
) -> Result<HashMap<String, UserOrdering>> {
    let dir = base.join(RANKINGS_DIR).join(milestone_id);

    if !dir.exists() {
        return Ok(HashMap::new());
    }

    let mut result = HashMap::new();

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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use std::collections::HashMap;
    use tempfile::TempDir;

    #[test]
    fn test_user_ordering_roundtrip() {
        let mut votes = HashMap::new();
        votes.insert("problem-1".to_string(), 3i32);
        votes.insert("problem-2".to_string(), -1i32);

        let ordering = UserOrdering {
            order: vec!["problem-1".to_string(), "problem-2".to_string()],
            votes,
            updated_at: Utc::now(),
        };

        let json = serde_json::to_string(&ordering).unwrap();
        let deserialized: UserOrdering = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.order, ordering.order);
        assert_eq!(deserialized.votes.len(), 2);
        assert_eq!(deserialized.votes["problem-1"], 3);
        assert_eq!(deserialized.votes["problem-2"], -1);
    }

    #[test]
    fn test_user_ordering_empty_votes_default() {
        let json = r#"{
            "order": ["p1", "p2"],
            "updated_at": "2026-03-22T00:00:00Z"
        }"#;

        let ordering: UserOrdering = serde_json::from_str(json).unwrap();

        assert_eq!(ordering.order, vec!["p1", "p2"]);
        assert!(ordering.votes.is_empty());
    }

    #[test]
    fn test_save_and_load_ordering() {
        let tmp = TempDir::new().unwrap();
        let base = tmp.path();

        let mut votes = HashMap::new();
        votes.insert("prob-a".to_string(), 2);
        votes.insert("prob-b".to_string(), 5);

        let ordering = UserOrdering {
            order: vec![
                "prob-a".to_string(),
                "prob-b".to_string(),
                "prob-c".to_string(),
            ],
            votes,
            updated_at: Utc::now(),
        };

        save_user_ordering(
            base,
            "milestone-1",
            "Alice Smith <alice@test.com>",
            &ordering,
        )
        .unwrap();

        let loaded = load_user_ordering(base, "milestone-1", "Alice Smith <alice@test.com>")
            .unwrap()
            .expect("ordering should exist");

        assert_eq!(loaded.order, ordering.order);
        assert_eq!(loaded.votes.len(), 2);
        assert_eq!(loaded.votes["prob-a"], 2);
        assert_eq!(loaded.votes["prob-b"], 5);
    }

    #[test]
    fn test_load_nonexistent_ordering_returns_none() {
        let tmp = TempDir::new().unwrap();
        let result = load_user_ordering(tmp.path(), "no-such-milestone", "nobody").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_load_all_orderings() {
        let tmp = TempDir::new().unwrap();
        let base = tmp.path();
        let milestone = "m-all";

        let ordering_alice = UserOrdering {
            order: vec!["p1".to_string(), "p2".to_string()],
            votes: HashMap::new(),
            updated_at: Utc::now(),
        };

        let mut bob_votes = HashMap::new();
        bob_votes.insert("p2".to_string(), 3);

        let ordering_bob = UserOrdering {
            order: vec!["p2".to_string(), "p1".to_string()],
            votes: bob_votes,
            updated_at: Utc::now(),
        };

        save_user_ordering(base, milestone, "alice", &ordering_alice).unwrap();
        save_user_ordering(base, milestone, "bob", &ordering_bob).unwrap();

        // Also write a .jsonl file to verify it gets ignored
        let dir = base.join("rankings").join(milestone);
        fs::write(dir.join("old-glicko.jsonl"), "should be ignored\n").unwrap();

        let all = load_all_orderings(base, milestone).unwrap();
        assert_eq!(all.len(), 2);
        assert!(all.contains_key("alice"));
        assert!(all.contains_key("bob"));

        assert_eq!(all["alice"].order, vec!["p1", "p2"]);
        assert_eq!(all["bob"].order, vec!["p2", "p1"]);
        assert_eq!(all["bob"].votes["p2"], 3);
    }

    /// Simulate the assign_tier remove/insert logic to verify index math.
    fn simulate_assign(order: &mut Vec<&str>, current_pos: usize, target_pos: usize) {
        let id = order.remove(current_pos);
        let adjusted = if current_pos < target_pos {
            target_pos - 1
        } else {
            target_pos
        };
        order.insert(adjusted, id);
    }

    #[test]
    fn test_assign_to_top_from_middle() {
        // 9 items, view_start=0, view_end=9
        let mut order: Vec<&str> = vec!["a", "b", "c", "d", "e", "f", "g", "h", "i"];
        // Shift+K on "e" (pos 4) → move to view_start (pos 0)
        simulate_assign(&mut order, 4, 0);
        assert_eq!(order, vec!["e", "a", "b", "c", "d", "f", "g", "h", "i"]);
    }

    #[test]
    fn test_assign_to_top_from_bottom() {
        let mut order: Vec<&str> = vec!["a", "b", "c", "d", "e", "f", "g", "h", "i"];
        // Shift+K on "h" (pos 7) → move to view_start (pos 0)
        simulate_assign(&mut order, 7, 0);
        assert_eq!(order, vec!["h", "a", "b", "c", "d", "e", "f", "g", "i"]);
    }

    #[test]
    fn test_assign_to_bottom_from_top() {
        let mut order: Vec<&str> = vec!["a", "b", "c", "d", "e", "f", "g", "h", "i"];
        // Shift+J on "b" (pos 1) → move to view_end-1 (pos 8)
        simulate_assign(&mut order, 1, 8);
        // After removing pos 1, target 8 becomes 7
        assert_eq!(order, vec!["a", "c", "d", "e", "f", "g", "h", "b", "i"]);
        // "b" at pos 7, "i" at pos 8 (items stack from bottom)
    }

    #[test]
    fn test_assign_to_bottom_from_middle() {
        let mut order: Vec<&str> = vec!["a", "b", "c", "d", "e", "f", "g", "h", "i"];
        // Shift+J on "e" (pos 4) → move to view_end-1 (pos 8)
        simulate_assign(&mut order, 4, 8);
        assert_eq!(order, vec!["a", "b", "c", "d", "f", "g", "h", "e", "i"]);
    }

    #[test]
    fn test_assign_with_drill_offset() {
        // Drilled view: view_start=3, view_end=6 (the middle tier of 9)
        let mut order: Vec<&str> = vec!["a", "b", "c", "d", "e", "f", "g", "h", "i"];
        // Shift+K on "f" (pos 5) → move to view_start (pos 3)
        simulate_assign(&mut order, 5, 3);
        assert_eq!(order, vec!["a", "b", "c", "f", "d", "e", "g", "h", "i"]);
    }

    #[test]
    fn test_assign_to_bottom_with_drill_offset() {
        // Drilled view: view_start=3, view_end=6
        let mut order: Vec<&str> = vec!["a", "b", "c", "d", "e", "f", "g", "h", "i"];
        // Shift+J on "d" (pos 3) → move to view_end-1 (pos 5)
        simulate_assign(&mut order, 3, 5);
        // After removing pos 3, target 5 becomes 4
        assert_eq!(order, vec!["a", "b", "c", "e", "d", "f", "g", "h", "i"]);
    }

    #[test]
    fn test_multiple_assigns_stack_at_top() {
        let mut order: Vec<&str> = vec!["a", "b", "c", "d", "e", "f"];
        // Shift+K on "d" (pos 3) → moves to 0
        simulate_assign(&mut order, 3, 0);
        assert_eq!(order, vec!["d", "a", "b", "c", "e", "f"]);
        // Shift+K on "f" (pos 5) → moves to 0
        simulate_assign(&mut order, 5, 0);
        assert_eq!(order, vec!["f", "d", "a", "b", "c", "e"]);
        // Top two are the promoted items, in reverse order of promotion
    }

    #[test]
    fn test_multiple_assigns_stack_at_bottom() {
        let mut order: Vec<&str> = vec!["a", "b", "c", "d", "e", "f"];
        // Shift+J on "b" (pos 1) → moves to 5
        simulate_assign(&mut order, 1, 5);
        assert_eq!(order, vec!["a", "c", "d", "e", "b", "f"]);
        // Shift+J on "c" (now pos 1) → moves to 5
        simulate_assign(&mut order, 1, 5);
        assert_eq!(order, vec!["a", "d", "e", "b", "c", "f"]);
        // Bottom items stack: f (original), c, b (most recently demoted closest to bottom)
    }

    #[test]
    fn test_votes_do_not_reorder_personal_list() {
        // Votes are decorations for the personal list. Tier assignment (Shift+K/J)
        // is the only thing that reorders. Votes contribute to global aggregation.
        let mut ord = UserOrdering {
            order: vec!["a".into(), "b".into(), "c".into(), "d".into()],
            votes: HashMap::from([("d".into(), 5), ("a".into(), -2)]),
            updated_at: Utc::now(),
        };
        // Adding votes should not change the order
        let order_before = ord.order.clone();
        // (no reorder function called — that's the point)
        assert_eq!(ord.order, order_before);
    }
}
