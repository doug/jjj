use std::collections::HashMap;

use super::ordering::{AggregatedRank, UserOrdering};

/// QV budget for a milestone with N problems.
pub fn qv_budget(problem_count: usize) -> u32 {
    100u32.max(2 * problem_count as u32)
}

/// Total cost of K votes = K^2.
pub fn vote_cost(votes: u32) -> u32 {
    votes * votes
}

/// Total cost of all a user's vote allocations.
pub fn total_vote_cost(votes: &HashMap<String, u32>) -> u32 {
    votes.values().map(|&k| vote_cost(k)).sum()
}

/// Aggregate multiple user orderings into a global ranking.
///
/// - **Borda**: rank 1 gets N points, rank 2 gets N-1, ..., rank N gets 1
///   (where N is the length of that user's ordering).
/// - **Owner weighting**: if `owner_slug` matches a user key, their Borda
///   scores are multiplied by 2.
/// - **QV boost**: each vote allocation of K on a problem adds K to the
///   aggregated score. Cost = K^2 drawn from the user's budget.
/// - **Budget**: `max(100, 2 * problem_count)` per user. If a user's total
///   vote cost exceeds the budget, their QV votes are silently skipped.
/// - **Ties** broken by problem_id (lexicographic ascending) for determinism.
///
/// Returns a `Vec` sorted by score descending, with 1-indexed positions.
pub fn aggregate_rankings(
    orderings: &HashMap<String, UserOrdering>,
    owner_slug: Option<&str>,
    problem_count: usize,
) -> Vec<(String, AggregatedRank)> {
    if orderings.is_empty() {
        return Vec::new();
    }

    let budget = qv_budget(problem_count);

    // Accumulate scores and voter counts per problem.
    let mut scores: HashMap<String, f64> = HashMap::new();
    let mut voter_counts: HashMap<String, usize> = HashMap::new();

    for (user, ordering) in orderings {
        let weight: f64 = if owner_slug == Some(user.as_str()) {
            2.0
        } else {
            1.0
        };

        let n = ordering.order.len();

        // Borda points
        for (i, problem_id) in ordering.order.iter().enumerate() {
            let borda_points = (n - i) as f64 * weight;
            *scores.entry(problem_id.clone()).or_insert(0.0) += borda_points;
            *voter_counts.entry(problem_id.clone()).or_insert(0) += 1;
        }

        // QV boost — only apply if user is within budget
        let cost = total_vote_cost(&ordering.votes);
        if cost <= budget {
            for (problem_id, &k) in &ordering.votes {
                *scores.entry(problem_id.clone()).or_insert(0.0) += k as f64;
                // Only count as a voter if not already counted via ordering
                if !ordering.order.contains(problem_id) {
                    *voter_counts.entry(problem_id.clone()).or_insert(0) += 1;
                }
            }
        }
    }

    // Collect, sort descending by score then ascending by ID for ties
    let mut results: Vec<(String, f64, usize)> = scores
        .into_iter()
        .map(|(id, score)| {
            let count = voter_counts.get(&id).copied().unwrap_or(0);
            (id, score, count)
        })
        .collect();

    results.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.0.cmp(&b.0))
    });

    // Assign 1-indexed positions
    results
        .into_iter()
        .enumerate()
        .map(|(i, (id, score, voter_count))| {
            (
                id,
                AggregatedRank {
                    position: i + 1,
                    score,
                    voter_count,
                },
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn make_ordering(order: Vec<&str>, votes: Vec<(&str, u32)>) -> UserOrdering {
        UserOrdering {
            order: order.into_iter().map(String::from).collect(),
            votes: votes
                .into_iter()
                .map(|(k, v)| (k.to_string(), v))
                .collect(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn test_single_user_borda() {
        let mut orderings = HashMap::new();
        orderings.insert(
            "alice".to_string(),
            make_ordering(vec!["p1", "p2", "p3"], vec![]),
        );

        let result = aggregate_rankings(&orderings, None, 3);

        assert_eq!(result.len(), 3);
        // p1 gets 3 points, p2 gets 2, p3 gets 1
        assert_eq!(result[0].0, "p1");
        assert_eq!(result[0].1.position, 1);
        assert!((result[0].1.score - 3.0).abs() < f64::EPSILON);

        assert_eq!(result[1].0, "p2");
        assert_eq!(result[1].1.position, 2);
        assert!((result[1].1.score - 2.0).abs() < f64::EPSILON);

        assert_eq!(result[2].0, "p3");
        assert_eq!(result[2].1.position, 3);
        assert!((result[2].1.score - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_two_users_equal_weight() {
        // Alice: p1 > p2 > p3 (scores: 3, 2, 1)
        // Bob:   p3 > p2 > p1 (scores: 3, 2, 1)
        // Combined: p1=4, p2=4, p3=4 — all tied, break by ID ascending
        let mut orderings = HashMap::new();
        orderings.insert(
            "alice".to_string(),
            make_ordering(vec!["p1", "p2", "p3"], vec![]),
        );
        orderings.insert(
            "bob".to_string(),
            make_ordering(vec!["p3", "p2", "p1"], vec![]),
        );

        let result = aggregate_rankings(&orderings, None, 3);

        assert_eq!(result.len(), 3);
        // All scores equal at 4.0, sorted by ID: p1, p2, p3
        assert_eq!(result[0].0, "p1");
        assert_eq!(result[1].0, "p2");
        assert_eq!(result[2].0, "p3");
        assert!((result[0].1.score - 4.0).abs() < f64::EPSILON);
        assert!((result[1].1.score - 4.0).abs() < f64::EPSILON);
        assert!((result[2].1.score - 4.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_owner_gets_2x_weight() {
        // Alice (owner): p2 > p1 (scores: 2*2=4, 2*1=2)
        // Bob:           p1 > p2 (scores: 2, 1)
        // Combined: p1 = 2+2 = 4, p2 = 4+1 = 5 => p2 wins
        let mut orderings = HashMap::new();
        orderings.insert(
            "alice".to_string(),
            make_ordering(vec!["p2", "p1"], vec![]),
        );
        orderings.insert(
            "bob".to_string(),
            make_ordering(vec!["p1", "p2"], vec![]),
        );

        let result = aggregate_rankings(&orderings, Some("alice"), 2);

        assert_eq!(result[0].0, "p2");
        assert!((result[0].1.score - 5.0).abs() < f64::EPSILON);
        assert_eq!(result[1].0, "p1");
        assert!((result[1].1.score - 4.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_qv_boost() {
        // Alice: p1 > p2 > p3 (Borda: 3, 2, 1)
        // Alice also votes 5 on p3 (cost=25, budget=100, within budget)
        // Scores: p1=3, p2=2, p3=1+5=6 => p3 should be first
        let mut orderings = HashMap::new();
        orderings.insert(
            "alice".to_string(),
            make_ordering(vec!["p1", "p2", "p3"], vec![("p3", 5)]),
        );

        let result = aggregate_rankings(&orderings, None, 3);

        assert_eq!(result[0].0, "p3");
        assert!((result[0].1.score - 6.0).abs() < f64::EPSILON);
        assert_eq!(result[0].1.position, 1);

        assert_eq!(result[1].0, "p1");
        assert!((result[1].1.score - 3.0).abs() < f64::EPSILON);

        assert_eq!(result[2].0, "p2");
        assert!((result[2].1.score - 2.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_qv_over_budget_silently_capped() {
        // Budget = max(100, 2*3) = 100
        // Alice votes 11 on p3 => cost = 121 > 100, votes silently skipped
        let mut orderings = HashMap::new();
        orderings.insert(
            "alice".to_string(),
            make_ordering(vec!["p1", "p2", "p3"], vec![("p3", 11)]),
        );

        let result = aggregate_rankings(&orderings, None, 3);

        // No QV boost applied, pure Borda
        assert_eq!(result[0].0, "p1");
        assert!((result[0].1.score - 3.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_qv_budget() {
        assert_eq!(qv_budget(0), 100);
        assert_eq!(qv_budget(10), 100);
        assert_eq!(qv_budget(49), 100);
        assert_eq!(qv_budget(50), 100);
        assert_eq!(qv_budget(51), 102);
        assert_eq!(qv_budget(100), 200);
        assert_eq!(qv_budget(1000), 2000);
    }

    #[test]
    fn test_vote_cost() {
        assert_eq!(vote_cost(0), 0);
        assert_eq!(vote_cost(1), 1);
        assert_eq!(vote_cost(2), 4);
        assert_eq!(vote_cost(3), 9);
        assert_eq!(vote_cost(10), 100);
    }

    #[test]
    fn test_total_vote_cost() {
        let votes: HashMap<String, u32> = vec![
            ("p1".to_string(), 3), // cost 9
            ("p2".to_string(), 2), // cost 4
            ("p3".to_string(), 1), // cost 1
        ]
        .into_iter()
        .collect();

        assert_eq!(total_vote_cost(&votes), 14);
    }

    #[test]
    fn test_total_vote_cost_empty() {
        let votes: HashMap<String, u32> = HashMap::new();
        assert_eq!(total_vote_cost(&votes), 0);
    }

    #[test]
    fn test_voter_count() {
        // Alice orders p1, p2; Bob orders p2, p3
        let mut orderings = HashMap::new();
        orderings.insert(
            "alice".to_string(),
            make_ordering(vec!["p1", "p2"], vec![]),
        );
        orderings.insert(
            "bob".to_string(),
            make_ordering(vec!["p2", "p3"], vec![]),
        );

        let result = aggregate_rankings(&orderings, None, 3);

        let counts: HashMap<String, usize> = result
            .iter()
            .map(|(id, rank)| (id.clone(), rank.voter_count))
            .collect();

        assert_eq!(counts["p1"], 1); // only alice
        assert_eq!(counts["p2"], 2); // alice + bob
        assert_eq!(counts["p3"], 1); // only bob
    }

    #[test]
    fn test_empty_orderings() {
        let orderings: HashMap<String, UserOrdering> = HashMap::new();
        let result = aggregate_rankings(&orderings, None, 5);
        assert!(result.is_empty());
    }
}
