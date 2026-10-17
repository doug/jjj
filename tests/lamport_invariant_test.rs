//! The Lamport clock must be stamped on every substantive write — and on no
//! other write.
//!
//! Concurrent edits to one entity are ordered by comparing Lamport clocks
//! (`storage::merge::pick_side`), so an edit that fails to move the clock is
//! invisible to the merge and gets decided by a wall-clock timestamp instead —
//! which is the failure the clock was introduced to remove. `MetadataStore::save`
//! stamps centrally, precisely so that sixty-odd save sites cannot drift apart.
//!
//! Central stamping makes the invariant true today; it does not keep it true. A
//! command that writes markdown by another route, or a future entity kind whose
//! save path skips the seam, would silently rejoin the broken majority, and no
//! existing test would notice: everything would still round-trip, list and show
//! correctly. Only a concurrent merge would be wrong, on whichever machine had
//! the faster clock.
//!
//! So this enumerates all five entity kinds and pins both halves of the
//! contract. The inverse half matters as much as the forward one: over-stamping
//! is also a defect, because a bookkeeping write that bumps the clock would let
//! a clone that merely re-saved an entity outrank one that genuinely edited it,
//! and `save` deliberately writes nothing at all when content is unchanged.
//!
//! The idea is borrowed from beads, which found that 26 of its 34 write paths
//! had silently stopped stamping their equivalent token, and responded with a
//! completeness test plus an inverse guard rather than a fix at each site.

mod test_helpers;

use test_helpers::{jj_available, run_jjj_success, setup_test_repo};

/// Read an entity's persisted clock straight from its markdown.
///
/// Deliberately not via `--json`: the clock is omitted from output when zero
/// (`skip_serializing_if`), and it is the *stored* value that decides a merge.
fn stored_clock(dir: &std::path::Path, subdir: &str, id: &str) -> u64 {
    let path = dir
        .join(".jj/jjj-meta")
        .join(subdir)
        .join(format!("{id}.md"));
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    text.lines()
        .find_map(|l| l.strip_prefix("lamport:"))
        .map(|v| {
            v.trim()
                .parse()
                .unwrap_or_else(|e| panic!("unparseable lamport in {}: {e}", path.display()))
        })
        // Absent means zero — what every entity written before clocks existed
        // looks like, and what a freshly created one looks like too.
        .unwrap_or(0)
}

fn id_of(json: &str) -> String {
    let v: serde_json::Value = serde_json::from_str(json).expect("valid JSON");
    v["id"].as_str().expect("id field").to_string()
}

fn first_id(json: &str) -> String {
    let v: serde_json::Value = serde_json::from_str(json).expect("valid JSON");
    v[0]["id"].as_str().expect("id field").to_string()
}

#[test]
fn every_entity_kind_stamps_the_clock_on_a_substantive_edit() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();
    let p = dir.path();

    // One of each kind. Five kinds, five rows below — if a sixth is ever added
    // and not listed here, that is the gap this test exists to make visible.
    let pid = id_of(&run_jjj_success(
        p,
        &["problem", "new", "A problem", "--json"],
    ));
    let sid = id_of(&run_jjj_success(
        p,
        &["solution", "new", "A solution", "--problem", &pid, "--json"],
    ));
    let cid = id_of(&run_jjj_success(
        p,
        &["critique", "new", &sid, "A critique", "--json"],
    ));
    let fid = id_of(&run_jjj_success(
        p,
        &["finding", "new", &pid, "A finding", "--json"],
    ));
    run_jjj_success(p, &["milestone", "new", "A milestone"]);
    let mid = first_id(&run_jjj_success(p, &["milestone", "list", "--json"]));

    let kinds: [(&str, &str, &str); 5] = [
        ("problem", "problems", &pid),
        ("solution", "solutions", &sid),
        ("critique", "critiques", &cid),
        ("finding", "findings", &fid),
        ("milestone", "milestones", &mid),
    ];

    for (kind, subdir, id) in kinds {
        let before = stored_clock(p, subdir, id);

        // Forward half: a real change must move the clock. The clock is a
        // per-clone counter, so it can jump by more than one — only the
        // direction is guaranteed.
        run_jjj_success(p, &[kind, "edit", id, "--title", "Edited once"]);
        let after = stored_clock(p, subdir, id);
        assert!(
            after > before,
            "{kind}: a substantive edit left the Lamport clock at {before}, \
             so a concurrent merge of this entity would be decided by wall clock"
        );

        // A second, different change must move it again — proving the first
        // bump was the edit and not a one-off on the entity's first save.
        run_jjj_success(p, &[kind, "edit", id, "--title", "Edited twice"]);
        let again = stored_clock(p, subdir, id);
        assert!(
            again > after,
            "{kind}: the clock moved once ({before} → {after}) and then stopped at {again}"
        );

        // Inverse half: re-applying the title it already has changes nothing, so
        // `save` must write nothing and the clock must stand still. A bump here
        // would let a clone that did no work outrank one that did.
        run_jjj_success(p, &[kind, "edit", id, "--title", "Edited twice"]);
        let noop = stored_clock(p, subdir, id);
        assert_eq!(
            noop, again,
            "{kind}: a no-op edit bumped the Lamport clock {again} → {noop}, \
             which would outrank a clone that genuinely edited the entity"
        );
    }
}

#[test]
fn a_body_edit_stamps_the_clock_too() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();
    let p = dir.path();

    // `--body` on `edit` reaches the same central save as `--title`, but it is
    // the newest write path and the one whose whole purpose is to replace the
    // payload a merge would otherwise have to reconcile. If any edit needs its
    // clock moved, it is this one.
    let pid = id_of(&run_jjj_success(
        p,
        &[
            "problem",
            "new",
            "Decode is slow",
            "--body",
            "First measurement.",
            "--json",
        ],
    ));
    let before = stored_clock(p, "problems", &pid);

    run_jjj_success(
        p,
        &["problem", "edit", &pid, "--body", "Corrected measurement."],
    );
    let after = stored_clock(p, "problems", &pid);
    assert!(
        after > before,
        "a body edit left the clock at {before}; the corrected body would lose \
         a merge against the figure it corrects"
    );

    run_jjj_success(
        p,
        &["problem", "edit", &pid, "--body", "Corrected measurement."],
    );
    let noop = stored_clock(p, "problems", &pid);
    assert_eq!(
        noop, after,
        "re-writing an identical body bumped the clock {after} → {noop}"
    );
}
