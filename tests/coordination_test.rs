//! Coordination-layer integration tests: env-based pod/actor identity and the
//! `jjj whoami` command that surfaces the resolved identity for an agent swarm.
//!
//! Identity resolution is process-scoped (`JJJ_POD` / `JJJ_USER` env vars), so
//! these tests spawn the real binary with per-child env rather than mutating the
//! test process environment.

mod test_helpers;

use test_helpers::{run_jjj, run_jjj_env, run_jjj_success, setup_test_repo};

/// Overwrite a problem's body with a three-way conflict block (the shape
/// `merge_body` emits on a real divergent fetch) so `conflicts`/`resolve` have
/// something to act on without staging a full two-clone push/fetch.
fn inject_conflict(meta_problems: &std::path::Path) -> String {
    let entry = std::fs::read_dir(meta_problems)
        .expect("problems dir")
        .flatten()
        .find(|e| e.path().extension().and_then(|x| x.to_str()) == Some("md"))
        .expect("one problem file");
    let path = entry.path();
    let original = std::fs::read_to_string(&path).unwrap();
    let (front, _body) = original
        .split_once("---\n")
        .and_then(|(_, rest)| rest.split_once("\n---\n"))
        .map(|(fm, body)| (format!("---\n{fm}\n---\n"), body.to_string()))
        .expect("frontmatter split");
    let conflicted = format!(
        "{front}<<<<<<< local\nlocal body wins\n=======\nremote body wins\n>>>>>>> remote\n"
    );
    std::fs::write(&path, conflicted).unwrap();
    path.file_stem().unwrap().to_string_lossy().into_owned()
}

/// With no env override and no `.sync_state.json` pod, `whoami` reports the jj
/// user as the actor, no pod, and the bare `jjj` push bookmark.
#[test]
fn test_whoami_defaults_to_jj_user_no_pod() {
    if !test_helpers::jj_available() {
        return;
    }
    let repo = setup_test_repo();
    let out = run_jjj_env(repo.path(), &[], &["whoami", "--json"]);
    assert!(
        out.status.success(),
        "whoami failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("whoami --json should emit valid JSON");
    assert_eq!(v["actor"], "Test User", "actor should be the jj user name");
    assert!(v["pod"].is_null(), "no pod without an override");
    assert_eq!(v["push_bookmark"], "jjj", "pod-less pushes to bare jjj");
}

/// `JJJ_POD` steers both the actor identity and the per-pod push bookmark
/// (`jjj/{pod}`), with a namespaced pod sanitized to a single ref segment.
#[test]
fn test_whoami_pod_env_steers_actor_and_bookmark() {
    if !test_helpers::jj_available() {
        return;
    }
    let repo = setup_test_repo();
    let out = run_jjj_env(
        repo.path(),
        &[("JJJ_POD", "team/theory")],
        &["whoami", "--json"],
    );
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(
        v["actor"], "team/theory",
        "pod becomes the actor when no JJJ_USER"
    );
    assert_eq!(v["pod"], "team/theory");
    assert_eq!(
        v["push_bookmark"], "jjj/team-theory",
        "namespaced pod sanitizes to one ref segment"
    );
}

/// `JJJ_USER` takes precedence over the pod for the actor identity, while the
/// pod still governs the push bookmark (the two are independent knobs).
#[test]
fn test_whoami_user_env_overrides_actor() {
    if !test_helpers::jj_available() {
        return;
    }
    let repo = setup_test_repo();
    let out = run_jjj_env(
        repo.path(),
        &[("JJJ_USER", "alice"), ("JJJ_POD", "bob")],
        &["whoami", "--json"],
    );
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["actor"], "alice", "JJJ_USER wins for actor");
    assert_eq!(v["pod"], "bob");
    assert_eq!(
        v["push_bookmark"], "jjj/bob",
        "pod still drives the bookmark"
    );
}

/// The resolved identity flows into event authorship: a problem created under
/// `JJJ_USER` is attributed to that actor in the event log's `by` field.
#[test]
fn test_env_identity_flows_into_event_author() {
    if !test_helpers::jj_available() {
        return;
    }
    let repo = setup_test_repo();

    let out = run_jjj_env(
        repo.path(),
        &[("JJJ_USER", "swarm-agent-7")],
        &["problem", "new", "Coordinated work"],
    );
    assert!(
        out.status.success(),
        "problem new failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let out = run_jjj_env(repo.path(), &[], &["events", "--json"]);
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("swarm-agent-7"),
        "event `by` should carry the resolved JJJ_USER identity; got: {stdout}"
    );
}

/// A conflicted entity is discoverable via `jjj conflicts --json` and resolvable
/// non-interactively with `jjj resolve --theirs`, which strips the markers and
/// logs a `conflict_resolved` event.
#[test]
fn test_conflicts_listed_and_resolved_to_theirs() {
    if !test_helpers::jj_available() {
        return;
    }
    let repo = setup_test_repo();
    run_jjj_success(repo.path(), &["problem", "new", "Divergent problem"]);

    let problems_dir = repo.path().join(".jj/jjj-meta/problems");
    let id = inject_conflict(&problems_dir);

    // Discovery.
    let out = run_jjj(repo.path(), &["conflicts", "--json"]);
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v.as_array().unwrap().len(), 1, "exactly one conflict");
    assert_eq!(v[0]["entity_type"], "problem");
    assert_eq!(v[0]["id"], id);

    // Resolution to the remote side.
    let out = run_jjj(repo.path(), &["resolve", &id, "--theirs"]);
    assert!(
        out.status.success(),
        "resolve failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    // The markers are gone and the remote hunk survived.
    let body = std::fs::read_to_string(problems_dir.join(format!("{id}.md"))).unwrap();
    assert!(!body.contains("<<<<<<<"), "markers stripped");
    assert!(body.contains("remote body wins"), "kept --theirs hunk");
    assert!(!body.contains("local body wins"), "dropped local hunk");

    // No conflicts remain.
    let out = run_jjj_success(repo.path(), &["conflicts"]);
    assert!(out.contains("No unresolved conflicts"), "cleared: {out}");

    // Audit trail records the resolution.
    let out = run_jjj_success(
        repo.path(),
        &["events", "--event-type", "conflict_resolved", "--json"],
    );
    assert!(out.contains("conflict_resolved"), "event logged: {out}");
}

/// `jjj resolve` with neither `--ours` nor `--theirs` refuses rather than
/// guessing, and an unknown id is a clear error.
#[test]
fn test_resolve_requires_a_side_and_valid_id() {
    if !test_helpers::jj_available() {
        return;
    }
    let repo = setup_test_repo();
    run_jjj_success(repo.path(), &["problem", "new", "Divergent problem"]);
    let id = inject_conflict(&repo.path().join(".jj/jjj-meta/problems"));

    let out = run_jjj(repo.path(), &["resolve", &id]);
    assert!(!out.status.success(), "must refuse without a side");
    assert!(String::from_utf8_lossy(&out.stderr).contains("--ours"));

    let out = run_jjj(repo.path(), &["resolve", "000000nonexistent", "--ours"]);
    assert!(!out.status.success(), "unknown id must error");
}
