use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::error::Result;

const RANKINGS_DIR: &str = "rankings";

/// Normalize a user identity string (e.g., "Alice Smith <alice@test.com>") into
/// a filesystem-safe slug (e.g., "alice-smith-a3f9").
///
/// The slug encodes both the display name *and* a short hash of the email so
/// two users named "Alex" with different emails don't collide. When no email
/// is present, only the name slug is used.
pub fn sanitize_user(user: &str) -> String {
    let (name_part, email_part) = if let Some(start) = user.find('<') {
        let after = &user[start + 1..];
        let email = after.split_once('>').map(|(e, _)| e).unwrap_or(after);
        (&user[..start], email)
    } else {
        (user, "")
    };

    let name_slug: String = name_part
        .trim()
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect();

    let name_slug = name_slug.trim_matches('-').to_string();
    let base = if name_slug.is_empty() {
        "anonymous".to_string()
    } else {
        name_slug
    };

    if email_part.trim().is_empty() {
        return base;
    }

    // Append a short hash of the email so identities with the same display
    // name but different emails don't share a slug. djb2 hash truncated to 4
    // hex chars is more than enough for casual collision avoidance and avoids
    // pulling in a real hash dependency.
    let hash = djb2_short(email_part.trim().to_lowercase().as_bytes());
    format!("{}-{:04x}", base, hash)
}

/// Simple djb2 hash, truncated to 16 bits. Not cryptographic — used only
/// to disambiguate user slugs with similar display names.
fn djb2_short(bytes: &[u8]) -> u16 {
    let mut h: u32 = 5381;
    for &b in bytes {
        h = h.wrapping_mul(33).wrapping_add(b as u32);
    }
    (h & 0xFFFF) as u16
}

/// Size of an authored gap *below* an item in a user's ordering.
///
/// A gap expresses *intensity* — how big the priority cliff is below an item —
/// collapsing the old ordering-vs-votes split into a single signal (see
/// `docs/design/latent-preference-ranking.md`). The absence of an entry means
/// the implicit **unit** gap, so an un-annotated ordering scores identically to
/// a plain ranked list. Sized gaps grow geometrically so an `XL` reads as a
/// "different league" cliff.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GapSize {
    S,
    M,
    L,
    XL,
}

impl GapSize {
    /// Cumulative-descent depth contributed by this gap. The implicit unit gap
    /// (no annotation) contributes `1.0`; sized gaps grow geometrically.
    pub fn depth(self) -> f64 {
        match self {
            GapSize::S => 2.0,
            GapSize::M => 4.0,
            GapSize::L => 8.0,
            GapSize::XL => 16.0,
        }
    }

    /// Short label for UI rendering.
    pub fn label(self) -> &'static str {
        match self {
            GapSize::S => "S",
            GapSize::M => "M",
            GapSize::L => "L",
            GapSize::XL => "XL",
        }
    }

    /// Cycle to the next size for the single-key gap toggle:
    /// `none → S → M → L → XL → none`.
    pub fn cycle(current: Option<GapSize>) -> Option<GapSize> {
        match current {
            None => Some(GapSize::S),
            Some(GapSize::S) => Some(GapSize::M),
            Some(GapSize::M) => Some(GapSize::L),
            Some(GapSize::L) => Some(GapSize::XL),
            Some(GapSize::XL) => None,
        }
    }
}

/// A single user's ordering for a milestone: a sorted list with optional
/// sized gaps expressing the priority cliffs between items.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserOrdering {
    /// Problem IDs in priority order (index 0 = highest priority).
    pub order: Vec<String>,
    /// Gap *below* the keyed item (absent = implicit unit gap). Keyed by
    /// problem id so a gap travels with its item across reorders and merges
    /// field-wise on `jj git fetch` rather than as a position-fragile array.
    #[serde(default)]
    pub gaps: HashMap<String, GapSize>,
    pub updated_at: DateTime<Utc>,
}

impl UserOrdering {
    /// Depth contributed by the gap below `id` — the authored sized gap if
    /// present, otherwise the implicit unit gap (`1.0`).
    pub fn gap_depth(&self, id: &str) -> f64 {
        self.gaps.get(id).map_or(1.0, |g| g.depth())
    }
}

/// Aggregated ranking result for a single problem within a milestone.
#[derive(Debug, Clone)]
pub struct AggregatedRank {
    /// 1-indexed rank position (1 = highest priority).
    pub position: usize,
    /// Total aggregated score (budget-normalized, gap-weighted ordering).
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
    // Atomic tmp+rename so a crash or concurrent writer can't leave a
    // truncated JSON file that fails to deserialize (dropping the ordering).
    crate::storage::atomic_write(&file_path, json.as_bytes())?;

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
    fn test_sanitize_user_disambiguates_same_name() {
        let a = sanitize_user("Alex <alex@a.com>");
        let b = sanitize_user("Alex <alex@b.com>");
        assert_ne!(a, b, "two Alexes with different emails must not collide");
        assert!(a.starts_with("alex-"));
        assert!(b.starts_with("alex-"));
    }

    #[test]
    fn test_sanitize_user_same_email_stable() {
        let a = sanitize_user("Alex Smith <alex@a.com>");
        let b = sanitize_user("Alex Smith <alex@a.com>");
        assert_eq!(a, b);
    }

    #[test]
    fn test_sanitize_user_no_email_falls_back_to_name_only() {
        let s = sanitize_user("alice");
        assert_eq!(s, "alice");
    }

    #[test]
    fn test_sanitize_user_empty_uses_anonymous() {
        let s = sanitize_user("");
        assert_eq!(s, "anonymous");
    }

    #[test]
    fn test_gap_cycle_wraps_through_sizes_and_back_to_none() {
        assert_eq!(GapSize::cycle(None), Some(GapSize::S));
        assert_eq!(GapSize::cycle(Some(GapSize::S)), Some(GapSize::M));
        assert_eq!(GapSize::cycle(Some(GapSize::M)), Some(GapSize::L));
        assert_eq!(GapSize::cycle(Some(GapSize::L)), Some(GapSize::XL));
        assert_eq!(GapSize::cycle(Some(GapSize::XL)), None);
    }

    #[test]
    fn test_gap_depth_defaults_to_unit() {
        let ordering = UserOrdering {
            order: vec!["p1".into(), "p2".into()],
            gaps: HashMap::from([("p1".to_string(), GapSize::L)]),
            updated_at: Utc::now(),
        };
        assert_eq!(ordering.gap_depth("p1"), 8.0); // L
        assert_eq!(ordering.gap_depth("p2"), 1.0); // unannotated → unit
        assert_eq!(ordering.gap_depth("missing"), 1.0);
    }

    #[test]
    fn test_user_ordering_roundtrip() {
        let ordering = UserOrdering {
            order: vec!["problem-1".to_string(), "problem-2".to_string()],
            gaps: HashMap::from([("problem-1".to_string(), GapSize::XL)]),
            updated_at: Utc::now(),
        };

        let json = serde_json::to_string(&ordering).unwrap();
        let deserialized: UserOrdering = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.order, ordering.order);
        assert_eq!(deserialized.gaps.len(), 1);
        assert_eq!(deserialized.gaps["problem-1"], GapSize::XL);
    }

    #[test]
    fn test_user_ordering_empty_gaps_default() {
        let json = r#"{
            "order": ["p1", "p2"],
            "updated_at": "2026-03-22T00:00:00Z"
        }"#;

        let ordering: UserOrdering = serde_json::from_str(json).unwrap();

        assert_eq!(ordering.order, vec!["p1", "p2"]);
        assert!(ordering.gaps.is_empty());
    }

    #[test]
    fn test_legacy_votes_field_is_ignored() {
        // Old files carried a `votes` map; the field is gone but must still
        // parse (serde ignores unknown fields) so existing rankings load.
        let json = r#"{
            "order": ["p1", "p2"],
            "votes": {"p1": 3, "p2": -1},
            "updated_at": "2026-03-22T00:00:00Z"
        }"#;

        let ordering: UserOrdering = serde_json::from_str(json).unwrap();
        assert_eq!(ordering.order, vec!["p1", "p2"]);
        assert!(ordering.gaps.is_empty());
    }

    #[test]
    fn test_save_and_load_ordering() {
        let tmp = TempDir::new().unwrap();
        let base = tmp.path();

        let ordering = UserOrdering {
            order: vec![
                "prob-a".to_string(),
                "prob-b".to_string(),
                "prob-c".to_string(),
            ],
            gaps: HashMap::from([("prob-b".to_string(), GapSize::M)]),
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
        assert_eq!(loaded.gaps.len(), 1);
        assert_eq!(loaded.gaps["prob-b"], GapSize::M);
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
            gaps: HashMap::new(),
            updated_at: Utc::now(),
        };

        let ordering_bob = UserOrdering {
            order: vec!["p2".to_string(), "p1".to_string()],
            gaps: HashMap::from([("p2".to_string(), GapSize::S)]),
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
        assert_eq!(all["bob"].gaps["p2"], GapSize::S);
    }
}
