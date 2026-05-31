use std::collections::HashMap;

use super::ordering::{AggregatedRank, UserOrdering};

/// QV budget for a milestone with N problems.
pub fn qv_budget(problem_count: usize) -> u32 {
    100u32.max(2 * problem_count as u32)
}

/// Canonical count of problems belonging to a milestone — the denominator for
/// the QV budget. This MUST be computed identically at vote-entry time (TUI)
/// and at aggregation time (TUI cache + CLI `rank`); if the counts diverge, a
/// vote accepted within budget interactively can be silently dropped as
/// over-budget during aggregation. All problems with the milestone are counted
/// regardless of status, matching what the interactive vote path uses.
pub fn milestone_problem_count(problems: &[crate::models::Problem], milestone_id: &str) -> usize {
    problems
        .iter()
        .filter(|p| p.milestone_id.as_deref() == Some(milestone_id))
        .count()
}

/// Cost of a signed vote allocation: |v|^2.
///
/// Computed in u64 and saturated to `u32::MAX` so a hand-edited, corrupted, or
/// merged `rankings/*.json` carrying a huge magnitude can't overflow (which
/// wrapped to 0 in release — bypassing the budget — or panicked in debug). A
/// saturated cost stays far above any real budget, so the over-budget guard in
/// [`aggregate_rankings`] correctly skips it.
pub fn vote_cost(votes: i32) -> u32 {
    let a = votes.unsigned_abs() as u64;
    (a * a).min(u32::MAX as u64) as u32
}

/// Total cost of all a user's vote allocations: sum of |v|^2.
///
/// Summed in u64 and saturated so a pathological file can't overflow the sum.
pub fn total_vote_cost(votes: &HashMap<String, i32>) -> u32 {
    let sum: u64 = votes.values().map(|&v| vote_cost(v) as u64).sum();
    sum.min(u32::MAX as u64) as u32
}

/// Aggregate multiple user orderings into a global ranking.
///
/// Two signals combine, deliberately on the **same scale** so each voter has
/// roughly equal baseline influence and votes act as a bounded megaphone:
///
/// - **Normalized harmonic ordering**: each voter's ordering points sum to the
///   QV budget `B`, distributed by harmonic weight — rank `i` gets
///   `B · (1/i) / H_n` where `H_n = Σ 1/k` over the voter's `n` ranked items.
///   This gives every voter the *same* total ordering weight regardless of how
///   many items they ranked (no length bias), while concentrating influence at
///   the top (so refining the top tier matters most). The top item gets
///   `≈ 0.34·B`.
/// - **Quadratic votes**: each allocation `v` adds `sign(v)·v²` to the score,
///   gated by the budget `B = max(100, 2·problem_count)` (a voter whose total
///   `Σv²` exceeds `B` has their votes skipped). A maxed vote contributes
///   `≈ B` — about 3× the top ordinal slot — so only a real budget spend
///   overrides a clear ranking, and a negative vote (the only sub-zero signal,
///   since ordering points are all ≥0) sinks an item below the whole list.
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
    let budget_f = budget as f64;

    // Accumulate scores and voter counts per problem.
    let mut scores: HashMap<String, f64> = HashMap::new();
    let mut voter_counts: HashMap<String, usize> = HashMap::new();

    for ordering in orderings.values() {
        let n = ordering.order.len();

        if n > 0 {
            // Harmonic weights 1/i sum to H_n; scale so each voter's ordering
            // points total exactly `budget` (equal per-voter influence).
            let h_n: f64 = (1..=n).map(|k| 1.0 / k as f64).sum();
            for (i, problem_id) in ordering.order.iter().enumerate() {
                let rank = (i + 1) as f64;
                let points = budget_f * (1.0 / rank) / h_n;
                *scores.entry(problem_id.clone()).or_insert(0.0) += points;
                *voter_counts.entry(problem_id.clone()).or_insert(0) += 1;
            }
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
            votes: votes.into_iter().map(|(k, v)| (k.to_string(), v)).collect(),
            updated_at: Utc::now(),
        }
    }

    /// Sum of all aggregated scores (one voter, no votes → equals their total
    /// ordering influence).
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
        // A voter's ordering points total the QV budget (equal influence).
        assert!((total_score(&result) - 100.0).abs() < 1e-9);
        // Harmonic shape: strictly descending, top dominant.
        assert!(result[0].1.score > result[1].1.score);
        assert!(result[1].1.score > result[2].1.score);
        assert!(result[0].1.score > 50.0, "top item should get ~0.55·budget");
    }

    #[test]
    fn test_equal_influence_regardless_of_ordering_length() {
        // The whole point of option 1: a voter who ranks 3 items and a voter
        // who ranks 10 contribute the SAME total ordering weight (= budget).
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
        // By symmetry p1 and p3 tie (each is one voter's top + one's bottom),
        // p2 (both middles) is lower; tie broken by ID → p1 first. Total = 2·budget.
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
    fn test_small_vote_does_not_override_clear_ranking() {
        // A modest +3 vote (cost 9) must NOT lift the bottom item over a clear
        // #1 — votes are a budgeted megaphone, not a cheap override.
        let mut orderings = HashMap::new();
        orderings.insert(
            "alice".to_string(),
            make_ordering(vec!["p1", "p2", "p3"], vec![("p3", 3)]),
        );

        let result = aggregate_rankings(&orderings, 3); // budget 100

        assert_eq!(result[0].0, "p1", "a +3 vote shouldn't beat the #1 ranking");
        // But the vote did lift p3 — it now beats nobody above it only because
        // the spend was small; its score rose by exactly the QV contribution (9).
        let p3 = result.iter().find(|(id, _)| id == "p3").unwrap().1.score;
        assert!(
            p3 > 18.0,
            "p3 got its ordering points (~18) plus the +9 vote"
        );
    }

    #[test]
    fn test_large_vote_overrides_ranking() {
        // Spending real budget (+7, cost 49) on the bottom item lifts it above
        // the #1 — exactly "this one is a cut above, I'll spend to pin it."
        let mut orderings = HashMap::new();
        orderings.insert(
            "alice".to_string(),
            make_ordering(vec!["p1", "p2", "p3"], vec![("p3", 7)]),
        );

        let result = aggregate_rankings(&orderings, 3);

        assert_eq!(
            result[0].0, "p3",
            "a +7 vote (cost 49) should override the ranking"
        );
    }

    #[test]
    fn test_strong_negative_vote_sinks_below_pack() {
        // A negative vote is the only way to drive a score below the pack
        // ("anything but this"). A strong -8 on the #1 item buries it last.
        let mut orderings = HashMap::new();
        orderings.insert(
            "alice".to_string(),
            make_ordering(vec!["p1", "p2", "p3"], vec![("p1", -8)]),
        );

        let result = aggregate_rankings(&orderings, 3);

        assert_eq!(
            result.last().unwrap().0,
            "p1",
            "strong -vote sinks p1 to the bottom"
        );
        assert!(
            result.last().unwrap().1.score < 0.0,
            "and drives its score negative"
        );
    }

    #[test]
    fn test_qv_over_budget_silently_capped() {
        // Budget = max(100, 2*3) = 100. Alice votes 11 on p3 => cost 121 > 100,
        // so her votes are skipped entirely — only the ordering counts.
        let mut orderings = HashMap::new();
        orderings.insert(
            "alice".to_string(),
            make_ordering(vec!["p1", "p2", "p3"], vec![("p3", 11)]),
        );

        let result = aggregate_rankings(&orderings, 3);

        // Pure ordering: the over-budget vote had no effect.
        assert_eq!(
            result.iter().map(|(id, _)| id.as_str()).collect::<Vec<_>>(),
            vec!["p1", "p2", "p3"]
        );
        assert!((total_score(&result) - 100.0).abs() < 1e-9);
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
    fn test_vote_cost_saturates_instead_of_overflowing() {
        // |v| >= 65536 overflows u32 squaring: must saturate, never wrap to 0
        // (which would bypass the QV budget) or panic in debug.
        assert_eq!(vote_cost(65536), u32::MAX);
        assert_eq!(vote_cost(i32::MAX), u32::MAX);
        assert_eq!(vote_cost(i32::MIN), u32::MAX);
        // A huge vote must cost MORE than any real budget so it is skipped.
        let votes: HashMap<String, i32> = std::iter::once(("p1".to_string(), 200_000)).collect();
        assert!(total_vote_cost(&votes) > qv_budget(10_000));
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

    #[test]
    fn test_opposing_votes_cancel_leaving_ordering() {
        // Alice +3 on p1, Bob -3 on p1 — equal and opposite (cost 9 each).
        // Their QV contributions cancel, so the result is pure (doubled)
        // ordering: both rank p1 > p2, so p1 stays on top.
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

        // Compare against the same two voters with NO votes — must be identical
        // (the +9/-9 cancel exactly).
        let mut no_votes = HashMap::new();
        no_votes.insert("alice".to_string(), make_ordering(vec!["p1", "p2"], vec![]));
        no_votes.insert("bob".to_string(), make_ordering(vec!["p1", "p2"], vec![]));
        let baseline = aggregate_rankings(&no_votes, 2);

        let s: HashMap<&str, f64> = result
            .iter()
            .map(|(id, r)| (id.as_str(), r.score))
            .collect();
        let b: HashMap<&str, f64> = baseline
            .iter()
            .map(|(id, r)| (id.as_str(), r.score))
            .collect();
        assert!((s["p1"] - b["p1"]).abs() < 1e-9);
        assert!((s["p2"] - b["p2"]).abs() < 1e-9);
        assert_eq!(result[0].0, "p1");
        // Two voters → total is 2·budget.
        assert!((total_score(&result) - 200.0).abs() < 1e-9);
    }
}
