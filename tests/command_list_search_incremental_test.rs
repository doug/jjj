//! `problem`/`solution`/`critique` `list --search` populates the FTS index via
//! a markdown reload. Regression coverage for switching that reload from the
//! full O(corpus) `load_from_markdown` to the content-hash-based
//! `load_from_markdown_incremental` already used by push/fetch: a file edited
//! directly on disk (not through jjj) must still be picked up on the very
//! next `--search`, proving the swap didn't trade correctness for speed.

mod test_helpers;

use std::fs;
use test_helpers::{jj_available, run_jjj_success, setup_test_repo};

#[test]
fn search_after_swap_to_incremental_reload_sees_a_direct_file_edit() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    let created = run_jjj_success(
        &dir,
        &["problem", "new", "Alphaneedle keyword", "--force", "--json"],
    );
    let json: serde_json::Value = serde_json::from_str(&created).expect("parse problem new json");
    let id = json["id"].as_str().expect("id field").to_string();

    // First --search populates the content-hash cache used by the incremental
    // reload.
    let found = run_jjj_success(
        &dir,
        &["problem", "list", "--search", "Alphaneedle", "--json"],
    );
    assert!(
        found.contains(&id),
        "expected freshly created problem to be found by search: {found}"
    );

    // Edit the entity file directly on disk, bypassing jjj entirely -- the
    // same class of external write (git merge, hand edit) the incremental
    // reload's doc comment says must never be trusted away.
    let path = dir
        .path()
        .join(".jj/jjj-meta/problems")
        .join(format!("{id}.md"));
    let original = fs::read_to_string(&path).expect("read problem file");
    let mutated = original.replace("Alphaneedle keyword", "Bravoneedle keyword");
    assert_ne!(original, mutated, "replacement did not match file contents");
    fs::write(&path, mutated).expect("write mutated problem file");

    // The new content must be searchable immediately...
    let found_new = run_jjj_success(
        &dir,
        &["problem", "list", "--search", "Bravoneedle", "--json"],
    );
    assert!(
        found_new.contains(&id),
        "direct edit was not picked up by the next search -- incremental reload skipped a changed file: {found_new}"
    );

    // ...and the old title must no longer match.
    let stale = run_jjj_success(
        &dir,
        &["problem", "list", "--search", "Alphaneedle", "--json"],
    );
    assert!(
        !stale.contains(&id),
        "stale keyword still matched after the file was edited on disk: {stale}"
    );
}
