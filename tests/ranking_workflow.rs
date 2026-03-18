use jjj::ranking::glicko2::{compute_ratings, Comparison, WeightedComparison};
use jjj::ranking::store::{append_comparison, load_comparisons, load_attributed_comparisons, sanitize_user};
use jjj::ranking::matchups::suggest_matchups;
use chrono::Utc;
use tempfile::TempDir;

#[test]
fn test_full_ranking_workflow() {
    let dir = TempDir::new().unwrap();
    let milestone_id = "m-test";

    // 1. Record comparisons from two users
    for (w, l) in &[("P-1", "P-2"), ("P-1", "P-3"), ("P-2", "P-3")] {
        append_comparison(dir.path(), milestone_id, "Alice <alice@test.com>", &Comparison {
            winner: w.to_string(),
            loser: l.to_string(),
            ts: Utc::now(),
        }).unwrap();
    }

    append_comparison(dir.path(), milestone_id, "Bob <bob@test.com>", &Comparison {
        winner: "P-3".into(),
        loser: "P-1".into(),
        ts: Utc::now(),
    }).unwrap();

    // 2. Load and verify
    let all = load_comparisons(dir.path(), milestone_id).unwrap();
    assert_eq!(all.len(), 4);

    let attributed = load_attributed_comparisons(dir.path(), milestone_id).unwrap();
    assert_eq!(attributed.len(), 4);

    // 3. Compute ratings (Alice is owner, weight=2)
    let alice_slug = sanitize_user("Alice <alice@test.com>");
    let weighted: Vec<WeightedComparison> = attributed
        .iter()
        .map(|(c, user)| WeightedComparison {
            winner: c.winner.clone(),
            loser: c.loser.clone(),
            weight: if *user == alice_slug { 2.0 } else { 1.0 },
        })
        .collect();
    let ratings = compute_ratings(&weighted);

    // P-1 should be highest (Alice ranked it first with 2x weight)
    assert!(ratings["P-1"].mu > ratings["P-3"].mu);

    // 4. Suggest next matchups for Bob
    let recent: Vec<(String, String)> = attributed
        .iter()
        .filter(|(_, u)| *u == sanitize_user("Bob <bob@test.com>"))
        .map(|(c, _)| (c.winner.clone(), c.loser.clone()))
        .collect();
    let matchups = suggest_matchups(&ratings, &recent, 3);
    assert!(!matchups.is_empty());

    // Bob only compared P-3 vs P-1, so P-2 should appear in suggestions
    let has_p2 = matchups.iter().any(|(a, b)| a == "P-2" || b == "P-2");
    assert!(has_p2, "Should suggest matchups involving the least-compared item");
}

#[test]
fn test_sanitize_user_roundtrip() {
    let slug = sanitize_user("Doug Fritz <doug@example.com>");
    assert_eq!(slug, "doug-fritz");

    let slug2 = sanitize_user("alice");
    assert_eq!(slug2, "alice");
}
