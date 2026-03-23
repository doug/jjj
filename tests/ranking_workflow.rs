use chrono::Utc;
use jjj::ranking::store::{
    append_comparison, load_attributed_comparisons, load_comparisons, sanitize_user, Comparison,
};
use tempfile::TempDir;

#[test]
fn test_comparison_store_workflow() {
    let dir = TempDir::new().unwrap();
    let milestone_id = "m-test";

    // 1. Record comparisons from two users
    for (w, l) in &[("P-1", "P-2"), ("P-1", "P-3"), ("P-2", "P-3")] {
        append_comparison(
            dir.path(),
            milestone_id,
            "Alice <alice@test.com>",
            &Comparison {
                winner: w.to_string(),
                loser: l.to_string(),
                ts: Utc::now(),
            },
        )
        .unwrap();
    }

    append_comparison(
        dir.path(),
        milestone_id,
        "Bob <bob@test.com>",
        &Comparison {
            winner: "P-3".into(),
            loser: "P-1".into(),
            ts: Utc::now(),
        },
    )
    .unwrap();

    // 2. Load and verify
    let all = load_comparisons(dir.path(), milestone_id).unwrap();
    assert_eq!(all.len(), 4);

    let attributed = load_attributed_comparisons(dir.path(), milestone_id).unwrap();
    assert_eq!(attributed.len(), 4);

    // 3. Verify user attribution
    let alice_slug = sanitize_user("Alice <alice@test.com>");
    let bob_slug = sanitize_user("Bob <bob@test.com>");
    let alice_count = attributed.iter().filter(|(_, u)| *u == alice_slug).count();
    let bob_count = attributed.iter().filter(|(_, u)| *u == bob_slug).count();
    assert_eq!(alice_count, 3);
    assert_eq!(bob_count, 1);
}

#[test]
fn test_sanitize_user_roundtrip() {
    let slug = sanitize_user("Doug Fritz <doug@example.com>");
    assert_eq!(slug, "doug-fritz");

    let slug2 = sanitize_user("alice");
    assert_eq!(slug2, "alice");
}
