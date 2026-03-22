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
    use std::collections::HashMap;

    #[test]
    fn test_user_ordering_roundtrip() {
        let mut votes = HashMap::new();
        votes.insert("problem-1".to_string(), 3);
        votes.insert("problem-2".to_string(), 1);

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
        assert_eq!(deserialized.votes["problem-2"], 1);
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
}
