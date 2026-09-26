//! A save must not delete fields this version does not recognise.
//!
//! jjj is distributed, so mixed versions across clones is the normal case rather
//! than the exception — and in a swarm the agents run a binary baked into an
//! image while the host gets rebuilt mid-run. Until this was fixed, an older
//! client that merely *touched* an entity silently deleted every frontmatter
//! field a newer one had written.
//!
//! The rule: data a reader does not understand must be retained verbatim if the
//! entity is written back. It applies to any format that round-trips through a
//! struct — whatever the struct does not name, it erases.

mod test_helpers;

use test_helpers::{jj_available, run_jjj_success, setup_test_repo};

/// Inject a field no version of jjj knows about, directly into the markdown.
fn inject(path: &std::path::Path, yaml: &str) {
    let s = std::fs::read_to_string(path).expect("read entity");
    let patched = s.replacen("status:", &format!("{yaml}\nstatus:"), 1);
    assert_ne!(
        s,
        patched,
        "injection anchor not found in {}",
        path.display()
    );
    std::fs::write(path, patched).expect("write entity");
}

fn entity_path(dir: &tempfile::TempDir, kind: &str, id: &str) -> std::path::PathBuf {
    dir.path()
        .join(".jj/jjj-meta")
        .join(kind)
        .join(format!("{id}.md"))
}

fn id_of(json: &str) -> String {
    let v: serde_json::Value = serde_json::from_str(json).expect("valid JSON");
    v["id"].as_str().expect("id").to_string()
}

#[test]
fn an_edit_preserves_fields_this_version_does_not_know() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();
    let pid = id_of(&run_jjj_success(
        &dir,
        &["problem", "new", "Keep my fields", "--json"],
    ));
    let path = entity_path(&dir, "problems", &pid);

    inject(
        &path,
        "severity_score: 7\nfuture_field: keep-me\nnested_thing:\n  a: 1\n  b: [x, y]",
    );

    run_jjj_success(&dir, &["problem", "edit", &pid, "--title", "Edited"]);

    let after = std::fs::read_to_string(&path).expect("read back");
    assert!(
        after.contains("title: Edited"),
        "the edit did not apply: {after}"
    );
    for field in ["severity_score", "future_field", "nested_thing"] {
        assert!(
            after.contains(field),
            "'{field}' was deleted by a client that does not know it:\n{after}"
        );
    }
    // Nested structure, not just scalars.
    assert!(after.contains("a: 1"), "nested value lost:\n{after}");

    // And it survives a second, independent mutation.
    run_jjj_success(&dir, &["problem", "edit", &pid, "--status", "in_progress"]);
    let again = std::fs::read_to_string(&path).expect("read back");
    assert!(
        again.contains("future_field") && again.contains("nested_thing"),
        "lost on the second save:\n{again}"
    );
}

/// The cache must carry them too, or `CACHE_FAITHFUL` is a lie.
///
/// Reads are DB-primary, so if the cache drops these fields then any
/// list-then-save path erases exactly what this feature exists to keep. This is
/// also the test that catches a column-index slip: reading the neighbouring
/// column as JSON fails silently and yields an empty map.
#[test]
fn the_cache_round_trips_unknown_fields_for_every_entity_type() {
    if !jj_available() {
        return;
    }
    let dir = setup_test_repo();

    // One of each kind, since each has its own column list and decoder.
    let pid = id_of(&run_jjj_success(
        &dir,
        &["problem", "new", "Parent", "--json"],
    ));
    let sid = id_of(&run_jjj_success(
        &dir,
        &["solution", "new", "Approach", "--problem", &pid, "--json"],
    ));
    let cid = id_of(&run_jjj_success(
        &dir,
        &["critique", "new", &sid, "An objection", "--json"],
    ));
    let fid = id_of(&run_jjj_success(
        &dir,
        &["finding", "new", &pid, "A measurement", "--json"],
    ));
    run_jjj_success(&dir, &["milestone", "new", "A cycle"]);
    let mid = {
        let v: serde_json::Value =
            serde_json::from_str(&run_jjj_success(&dir, &["milestone", "list", "--json"]))
                .expect("valid JSON");
        v[0]["id"].as_str().expect("id").to_string()
    };

    for (kind, id) in [
        ("problems", &pid),
        ("solutions", &sid),
        ("critiques", &cid),
        ("findings", &fid),
        ("milestones", &mid),
    ] {
        inject(&entity_path(&dir, kind, id), "future_field: keep-me");
    }

    // Force everything through the cache.
    run_jjj_success(&dir, &["db", "rebuild"]);

    for (kind, listing) in [
        ("problems", vec!["problem", "list", "--json"]),
        ("solutions", vec!["solution", "list", "--json"]),
        ("critiques", vec!["critique", "list", "--json"]),
        ("findings", vec!["finding", "list", "--json"]),
        ("milestones", vec!["milestone", "list", "--json"]),
    ] {
        let out = run_jjj_success(&dir, &listing);
        let v: serde_json::Value = serde_json::from_str(&out).expect("valid JSON");
        let first = &v[0];
        assert_eq!(
            first["future_field"], "keep-me",
            "{kind}: the cache dropped the unknown field, so a DB-primary read \
             cannot reconstruct the markdown (check the column index in row_to_*):\n{out}"
        );
    }
}
