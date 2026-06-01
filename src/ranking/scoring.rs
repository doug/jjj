use std::collections::HashMap;

use super::ordering::{AggregatedRank, UserOrdering};

/// Per-user normalization budget for a milestone with N problems.
///
/// Each user's ordering points are scaled to sum to this budget, giving every
/// user equal baseline influence regardless of how many items they ranked.
pub fn ordering_budget(problem_count: usize) -> u32 {
    100u32.max(2 * problem_count as u32)
}

/// Canonical count of problems belonging to a milestone — the denominator for
/// the normalization budget. Computed identically wherever a budget is needed
/// (TUI + CLI `rank`). All problems with the milestone are counted regardless
/// of status, matching what the interactive path uses.
pub fn milestone_problem_count(problems: &[crate::models::Problem], milestone_id: &str) -> usize {
    problems
        .iter()
        .filter(|p| p.milestone_id.as_deref() == Some(milestone_id))
        .count()
}

/// Aggregate multiple user orderings into a global ranking.
///
/// Each user contributes a single signal — a **budget-normalized, gap-weighted
/// ordering**:
///
/// - Walking the list top-to-bottom, item `k` sits at cumulative descent
///   `depth_k = Σ_{j<k} gap_below(j)`, where each gap defaults to the unit gap
///   `1.0` and an authored S/M/L/XL gap stretches the descent geometrically
///   (2/4/8/16). The item's raw weight is the harmonic `1 / (1 + depth_k)`.
/// - Weights are scaled so each user's points sum to the budget
///   `B = max(100, 2·problem_count)`, giving every user equal total influence
///   (no length bias) while concentrating weight at the top.
/// - With no authored gaps, `depth_k = k`, so the weights are `1, 1/2, …, 1/n`
///   — *identical* to plain harmonic ordering. Gaps are therefore strictly
///   backward-compatible: an un-annotated list scores exactly as before. A
///   large gap creates a real cliff (e.g. an `XL` above the bottom item
///   expresses "must not win" without any negative-vote channel).
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

    let budget = ordering_budget(problem_count);
    let budget_f = budget as f64;

    // Accumulate scores and voter counts per problem.
    let mut scores: HashMap<String, f64> = HashMap::new();
    let mut voter_counts: HashMap<String, usize> = HashMap::new();

    for ordering in orderings.values() {
        let n = ordering.order.len();
        if n == 0 {
            continue;
        }

        // Harmonic weight at each item's cumulative gap depth. The gap *below*
        // an item is applied after weighting it, pushing the next item lower.
        let mut depth = 0.0f64;
        let mut weights: Vec<f64> = Vec::with_capacity(n);
        for id in &ordering.order {
            weights.push(1.0 / (1.0 + depth));
            depth += ordering.gap_depth(id);
        }

        // Scale so this user's points sum to exactly `budget` (equal influence).
        let total_weight: f64 = weights.iter().sum();
        for (id, w) in ordering.order.iter().zip(weights.iter()) {
            *scores.entry(id.clone()).or_insert(0.0) += budget_f * w / total_weight;
            *voter_counts.entry(id.clone()).or_insert(0) += 1;
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
    use crate::ranking::ordering::GapSize;
    use chrono::Utc;

    fn make_ordering(order: Vec<&str>, gaps: Vec<(&str, GapSize)>) -> UserOrdering {
        UserOrdering {
            order: order.into_iter().map(String::from).collect(),
            gaps: gaps.into_iter().map(|(k, g)| (k.to_string(), g)).collect(),
            updated_at: Utc::now(),
        }
    }

    /// Sum of all aggregated scores (one voter → equals their total influence).
    fn total_score(result: &[(String, AggregatedRank)]) -> f64 {
        result.iter().map(|(_, r)| r.score).sum()
    }

    #[test]
    fn test_single_user_ordering_sums_to_budget_and_descends() {
        let mut orderings = HashMap::new();
        orderings.insert(
            "alice".to_string(),
            make_ordering(vec!["p1", "p2", "p3"], vec![]),
        );

        let result = aggregate_rankings(&orderings, 3); // budget = 100

        assert_eq!(
            result.iter().map(|(id, _)| id.as_str()).collect::<Vec<_>>(),
            vec!["p1", "p2", "p3"]
        );
        // A voter's ordering points total the budget (equal influence).
        assert!((total_score(&result) - 100.0).abs() < 1e-9);
        // Harmonic shape: strictly descending, top dominant.
        assert!(result[0].1.score > result[1].1.score);
        assert!(result[1].1.score > result[2].1.score);
        assert!(result[0].1.score > 50.0, "top item should get ~0.55·budget");
    }

    #[test]
    fn test_no_gaps_matches_plain_harmonic_exactly() {
        // The backward-compat guarantee: an un-annotated list reproduces the
        // exact harmonic weights 1, 1/2, …, 1/n scaled to the budget.
        let mut orderings = HashMap::new();
        orderings.insert(
            "a".to_string(),
            make_ordering(vec!["p1", "p2", "p3", "p4"], vec![]),
        );
        let result = aggregate_rankings(&orderings, 4); // budget 100
        let by_id: HashMap<&str, f64> = result
            .iter()
            .map(|(id, r)| (id.as_str(), r.score))
            .collect();

        let h: f64 = (1..=4).map(|k| 1.0 / k as f64).sum();
        for (k, id) in ["p1", "p2", "p3", "p4"].iter().enumerate() {
            let expected = 100.0 * (1.0 / (k as f64 + 1.0)) / h;
            assert!(
                (by_id[id] - expected).abs() < 1e-9,
                "score mismatch for {id}"
            );
        }
    }

    #[test]
    fn test_equal_influence_regardless_of_ordering_length() {
        // A voter who ranks 3 items and one who ranks 10 contribute the SAME
        // total ordering weight (= budget).
        let mut short = HashMap::new();
        short.insert(
            "a".to_string(),
            make_ordering(vec!["p1", "p2", "p3"], vec![]),
        );
        let mut long = HashMap::new();
        long.insert(
            "a".to_string(),
            make_ordering(
                vec!["p1", "p2", "p3", "p4", "p5", "p6", "p7", "p8", "p9", "p10"],
                vec![],
            ),
        );
        // problem_count 20 → budget 100 for both.
        let short_total = total_score(&aggregate_rankings(&short, 20));
        let long_total = total_score(&aggregate_rankings(&long, 20));
        assert!((short_total - 100.0).abs() < 1e-9);
        assert!((long_total - 100.0).abs() < 1e-9);
        assert!(
            (short_total - long_total).abs() < 1e-9,
            "length must not change total influence"
        );
    }

    #[test]
    fn test_two_users_symmetric() {
        // Alice: p1 > p2 > p3 ; Bob: p3 > p2 > p1 (mirror).
        // By symmetry p1 and p3 tie; p2 (both middles) is lower; tie broken by
        // ID → p1 first. Total = 2·budget.
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
        assert!((total_score(&result) - 200.0).abs() < 1e-9);
        assert_eq!(result[0].0, "p1");
        assert_eq!(result[1].0, "p3");
        assert!(
            (result[0].1.score - result[1].1.score).abs() < 1e-9,
            "p1 and p3 tie"
        );
        assert_eq!(result[2].0, "p2");
        assert!(
            result[1].1.score > result[2].1.score,
            "tied tops beat both-middles"
        );
    }

    #[test]
    fn test_small_gap_does_not_override_clear_ranking() {
        // A modest S gap below the #1 item nudges the others down but does not
        // reorder them relative to each other or unseat the top.
        let mut orderings = HashMap::new();
        orderings.insert(
            "alice".to_string(),
            make_ordering(vec!["p1", "p2", "p3"], vec![("p1", GapSize::S)]),
        );

        let result = aggregate_rankings(&orderings, 3);

        assert_eq!(
            result.iter().map(|(id, _)| id.as_str()).collect::<Vec<_>>(),
            vec!["p1", "p2", "p3"]
        );
        assert!(result[0].1.score > result[1].1.score);
    }

    #[test]
    fn test_large_gap_creates_a_cliff() {
        // An XL gap below the top item pushes p2/p3 far down and nearly level
        // with each other (a "different league" cliff after p1).
        let mut orderings = HashMap::new();
        orderings.insert(
            "alice".to_string(),
            make_ordering(vec!["p1", "p2", "p3"], vec![("p1", GapSize::XL)]),
        );

        let result = aggregate_rankings(&orderings, 3);

        assert_eq!(result[0].0, "p1");
        let p1 = result.iter().find(|(id, _)| id == "p1").unwrap().1.score;
        let p2 = result.iter().find(|(id, _)| id == "p2").unwrap().1.score;
        let p3 = result.iter().find(|(id, _)| id == "p3").unwrap().1.score;
        // p1 dominates; the cliff makes p2 and p3 nearly equal far below it.
        assert!(p1 > 5.0 * p2, "XL gap should make p1 dominate the rest");
        assert!(
            (p2 - p3).abs() < p2 * 0.1,
            "below the cliff items are close"
        );
    }

    #[test]
    fn test_gap_below_pack_buries_bottom_item() {
        // An XL gap below p2 sinks p3 ("must not win"), with no negative channel.
        let mut orderings = HashMap::new();
        orderings.insert(
            "alice".to_string(),
            make_ordering(vec!["p1", "p2", "p3"], vec![("p2", GapSize::XL)]),
        );

        let result = aggregate_rankings(&orderings, 3);

        assert_eq!(result.last().unwrap().0, "p3");
        let p2 = result.iter().find(|(id, _)| id == "p2").unwrap().1.score;
        let p3 = result.iter().find(|(id, _)| id == "p3").unwrap().1.score;
        assert!(p2 > 4.0 * p3, "the cliff below p2 buries p3 far down");
    }

    #[test]
    fn test_ordering_budget() {
        assert_eq!(ordering_budget(0), 100);
        assert_eq!(ordering_budget(10), 100);
        assert_eq!(ordering_budget(50), 100);
        assert_eq!(ordering_budget(51), 102);
        assert_eq!(ordering_budget(100), 200);
        assert_eq!(ordering_budget(1000), 2000);
    }

    #[test]
    fn test_voter_count() {
        // Alice orders p1, p2; Bob orders p2, p3
        let mut orderings = HashMap::new();
        orderings.insert("alice".to_string(), make_ordering(vec!["p1", "p2"], vec![]));
        orderings.insert("bob".to_string(), make_ordering(vec!["p2", "p3"], vec![]));

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
}
