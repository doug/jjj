use std::collections::HashMap;

use super::ordering::{AggregatedRank, UserOrdering};

/// QV budget for a milestone with N problems.
pub fn qv_budget(problem_count: usize) -> u32 {
    100u32.max(2 * problem_count as u32)
}

/// Cost of a signed vote allocation: |v|^2.
pub fn vote_cost(votes: i32) -> u32 {
    votes.unsigned_abs() * votes.unsigned_abs()
}

/// Total cost of all a user's vote allocations: sum of |v|^2.
pub fn total_vote_cost(votes: &HashMap<String, i32>) -> u32 {
    votes.values().map(|&v| vote_cost(v)).sum()
}

/// Aggregate multiple user orderings into a global ranking.
///
/// - **Harmonic rank**: rank i (1-indexed) gets N/i points
///   (where N is the length of that user's ordering).
/// - **Squared votes**: each vote allocation of v on a problem adds
///   sign(v) × v² to the aggregated score.
/// - **Budget**: `max(100, 2 * problem_count)` per user. If a user's total
///   vote cost exceeds the budget, their QV votes are silently skipped.
/// - **Ties** broken by problem_id (lexicographic ascending) for determinism.
///
/// Returns a `Vec` sorted by score descending, with 1-indexed positions.
pub fn aggregate_rankings(
    orderings: &HashMap<String, UserOrdering>,
    problem_count: usize,
) -> Vec<(String, AggregatedRank)> {
    if orderings.is_empty() {
        return Vec::new();
    }

    let budget = qv_budget(problem_count);

    // Accumulate scores and voter counts per problem.
    let mut scores: HashMap<String, f64> = HashMap::new();
    let mut voter_counts: HashMap<String, usize> = HashMap::new();

    for ordering in orderings.values() {
        let n = ordering.order.len();

        // Harmonic rank points: N / rank (1-indexed)
        for (i, problem_id) in ordering.order.iter().enumerate() {
            let rank = (i + 1) as f64;
            let harmonic_points = n as f64 / rank;
            *scores.entry(problem_id.clone()).or_insert(0.0) += harmonic_points;
            *voter_counts.entry(problem_id.clone()).or_insert(0) += 1;
        }

        // QV boost — only apply if user is within budget
        let cost = total_vote_cost(&ordering.votes);
        if cost <= budget {
            for (problem_id, &v) in &ordering.votes {
                // sign(v) × v² = v × |v|
                let contribution = v as f64 * (v.unsigned_abs() as f64);
                *scores.entry(problem_id.clone()).or_insert(0.0) += contribution;
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

    fn make_ordering(order: Vec<&str>, votes: Vec<(&str, i32)>) -> UserOrdering {
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
    fn test_single_user_harmonic() {
        let mut orderings = HashMap::new();
        orderings.insert(
            "alice".to_string(),
            make_ordering(vec!["p1", "p2", "p3"], vec![]),
        );

        let result = aggregate_rankings(&orderings, 3);

        assert_eq!(result.len(), 3);
        // p1: 3/1 = 3.0, p2: 3/2 = 1.5, p3: 3/3 = 1.0
        assert_eq!(result[0].0, "p1");
        assert_eq!(result[0].1.position, 1);
        assert!((result[0].1.score - 3.0).abs() < f64::EPSILON);

        assert_eq!(result[1].0, "p2");
        assert_eq!(result[1].1.position, 2);
        assert!((result[1].1.score - 1.5).abs() < f64::EPSILON);

        assert_eq!(result[2].0, "p3");
        assert_eq!(result[2].1.position, 3);
        assert!((result[2].1.score - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_two_users_symmetric() {
        // Alice: p1 > p2 > p3 (harmonic: 3/1, 3/2, 3/3 = 3.0, 1.5, 1.0)
        // Bob:   p3 > p2 > p1 (harmonic: 3/1, 3/2, 3/3 = 3.0, 1.5, 1.0)
        // Combined: p1=4.0, p2=3.0, p3=4.0 — p1 and p3 tied, break by ID
        let mut orderings = HashMap::new();
        orderings.insert(
            "alice".to_string(),
            make_ordering(vec!["p1", "p2", "p3"], vec![]),
        );
        orderings.insert(
            "bob".to_string(),
            make_ordering(vec!["p3", "p2", "p1"], vec![]),
        );

        let result = aggregate_rankings(&orderings, 3);

        assert_eq!(result.len(), 3);
        // p1 and p3 both score 4.0, tied by ID: p1 first
        assert_eq!(result[0].0, "p1");
        assert!((result[0].1.score - 4.0).abs() < f64::EPSILON);
        assert_eq!(result[1].0, "p3");
        assert!((result[1].1.score - 4.0).abs() < f64::EPSILON);
        assert_eq!(result[2].0, "p2");
        assert!((result[2].1.score - 3.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_qv_positive_votes() {
        // Alice: p1 > p2 > p3 (harmonic: 3.0, 1.5, 1.0)
        // Alice also votes +3 on p3 (contribution: 3*3=9, cost=9, budget=100)
        // Scores: p1=3.0, p2=1.5, p3=1.0+9=10.0 => p3 first
        let mut orderings = HashMap::new();
        orderings.insert(
            "alice".to_string(),
            make_ordering(vec!["p1", "p2", "p3"], vec![("p3", 3)]),
        );

        let result = aggregate_rankings(&orderings, 3);

        assert_eq!(result[0].0, "p3");
        assert!((result[0].1.score - 10.0).abs() < f64::EPSILON);

        assert_eq!(result[1].0, "p1");
        assert!((result[1].1.score - 3.0).abs() < f64::EPSILON);

        assert_eq!(result[2].0, "p2");
        assert!((result[2].1.score - 1.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_qv_negative_votes() {
        // Alice: p1 > p2 (harmonic: 2/1=2.0, 2/2=1.0)
        // Alice votes -2 on p1 (contribution: -2*2=-4, cost=4, budget=100)
        // Scores: p1=2.0-4=-2.0, p2=1.0 => p2 first
        let mut orderings = HashMap::new();
        orderings.insert(
            "alice".to_string(),
            make_ordering(vec!["p1", "p2"], vec![("p1", -2)]),
        );

        let result = aggregate_rankings(&orderings, 2);

        assert_eq!(result[0].0, "p2");
        assert!((result[0].1.score - 1.0).abs() < f64::EPSILON);

        assert_eq!(result[1].0, "p1");
        assert!((result[1].1.score - (-2.0)).abs() < f64::EPSILON);
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

        let result = aggregate_rankings(&orderings, 3);

        // No QV boost applied, pure harmonic
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
        // Negative votes cost the same
        assert_eq!(vote_cost(-1), 1);
        assert_eq!(vote_cost(-3), 9);
        assert_eq!(vote_cost(-10), 100);
    }

    #[test]
    fn test_total_vote_cost() {
        let votes: HashMap<String, i32> = vec![
            ("p1".to_string(), 3),  // cost 9
            ("p2".to_string(), -2), // cost 4
            ("p3".to_string(), 1),  // cost 1
        ]
        .into_iter()
        .collect();

        assert_eq!(total_vote_cost(&votes), 14);
    }

    #[test]
    fn test_total_vote_cost_empty() {
        let votes: HashMap<String, i32> = HashMap::new();
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

        let result = aggregate_rankings(&orderings, 3);

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
        let result = aggregate_rankings(&orderings, 5);
        assert!(result.is_empty());
    }

    #[test]
    fn test_negative_budget_symmetric() {
        // +3 costs 9, -3 costs 9 — same budget
        let mut orderings = HashMap::new();
        orderings.insert(
            "alice".to_string(),
            make_ordering(vec!["p1", "p2"], vec![("p1", 3)]),
        );
        orderings.insert(
            "bob".to_string(),
            make_ordering(vec!["p1", "p2"], vec![("p1", -3)]),
        );

        let result = aggregate_rankings(&orderings, 2);

        // Alice: p1 harmonic=2.0+9=11.0, p2=1.0
        // Bob:   p1 harmonic=2.0-9=-7.0, p2=1.0
        // Combined: p1=4.0, p2=2.0
        let scores: HashMap<String, f64> = result
            .iter()
            .map(|(id, rank)| (id.clone(), rank.score))
            .collect();
        assert!((scores["p1"] - 4.0).abs() < f64::EPSILON);
        assert!((scores["p2"] - 2.0).abs() < f64::EPSILON);
    }
}
