//! Integration tests for push and fetch commands.
//!
//! These tests create a bare git repo as a "remote" to test actual push/fetch functionality.

mod test_helpers;

use std::process::Command;
use tempfile::TempDir;
use test_helpers::run_jjj;

/// Helper to run jj command
#[allow(dead_code)]
fn run_jj(dir: &std::path::Path, args: &[&str]) -> std::process::Output {
    Command::new("jj")
        .current_dir(dir)
        .args(args)
        .output()
        .expect("Failed to execute jj")
}

/// Create a bare git repo to use as a remote
fn create_bare_remote() -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temp dir for bare repo");

    let status = Command::new("git")
        .current_dir(temp_dir.path())
        .args(["init", "--bare"])
        .status()
        .expect("Failed to init bare repo");
    assert!(status.success(), "Failed to create bare git repo");

    temp_dir
}

/// Setup a jj repo with a remote configured
fn setup_repo_with_remote(remote_path: &std::path::Path) -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Init jj repo with git colocate
    let status = Command::new("jj")
        .current_dir(temp_dir.path())
        .args(["git", "init", "--colocate"])
        .status()
        .expect("Failed to run jj init");
    assert!(status.success(), "jj init failed");

    // Configure user
    Command::new("jj")
        .current_dir(temp_dir.path())
        .args(["config", "set", "--repo", "user.name", "Test User"])
        .status()
        .expect("Failed to set user.name");
    Command::new("jj")
        .current_dir(temp_dir.path())
        .args(["config", "set", "--repo", "user.email", "test@example.com"])
        .status()
        .expect("Failed to set user.email");

    // Add remote
    let remote_url = format!("file://{}", remote_path.display());
    let status = Command::new("jj")
        .current_dir(temp_dir.path())
        .args(["git", "remote", "add", "origin", &remote_url])
        .status()
        .expect("Failed to add remote");
    assert!(status.success(), "Failed to add git remote");

    temp_dir
}

#[test]
fn test_push_to_bare_remote() {
    if !test_helpers::jj_available() {
        return;
    }

    // 1. Create bare remote
    let remote_dir = create_bare_remote();

    // 2. Setup repo with remote
    let repo_dir = setup_repo_with_remote(remote_dir.path());
    let dir = repo_dir.path();

    // 3. Initialize jjj
    let output = run_jjj(dir, &["init"]);
    assert!(
        output.status.success(),
        "jjj init failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // 4. Create some entities
    let output = run_jjj(dir, &["problem", "new", "Test Problem for Push"]);
    assert!(
        output.status.success(),
        "problem new failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let output = run_jjj(
        dir,
        &[
            "solution",
            "new",
            "Test Solution",
            "--problem",
            "Test Problem for Push",
        ],
    );
    assert!(
        output.status.success(),
        "solution new failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // 5. Push to remote
    let output = run_jjj(dir, &["push", "--remote", "origin"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    println!("Push stdout: {}", stdout);
    println!("Push stderr: {}", stderr);

    assert!(output.status.success(), "jjj push failed: {}", stderr);
    assert!(
        stdout.contains("Pushing jjj"),
        "Should mention pushing jjj bookmark"
    );

    // 6. Verify the jjj bookmark exists on the remote
    let output = Command::new("git")
        .current_dir(remote_dir.path())
        .args(["branch", "-a"])
        .output()
        .expect("Failed to list branches");
    let branches = String::from_utf8_lossy(&output.stdout);
    assert!(
        branches.contains("jjj"),
        "Remote should have jjj branch. Got: {}",
        branches
    );
}

#[test]
fn test_fetch_from_remote() {
    if !test_helpers::jj_available() {
        return;
    }

    // 1. Create bare remote
    let remote_dir = create_bare_remote();

    // 2. Setup first repo (Alice) and push data
    let alice_dir = setup_repo_with_remote(remote_dir.path());

    run_jjj(alice_dir.path(), &["init"]);
    run_jjj(alice_dir.path(), &["problem", "new", "Shared Problem"]);
    run_jjj(
        alice_dir.path(),
        &[
            "solution",
            "new",
            "Alice Solution",
            "--problem",
            "Shared Problem",
        ],
    );

    let output = run_jjj(alice_dir.path(), &["push", "--remote", "origin"]);
    assert!(
        output.status.success(),
        "Alice push failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // 3. Setup second repo (Bob) pointing to same remote
    let bob_dir = setup_repo_with_remote(remote_dir.path());

    // 4. Initialize jjj for Bob (but don't create any entities yet)
    run_jjj(bob_dir.path(), &["init"]);

    // 5. Fetch from remote
    let output = run_jjj(bob_dir.path(), &["fetch", "--remote", "origin"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    println!("Fetch stdout: {}", stdout);
    println!("Fetch stderr: {}", stderr);

    assert!(output.status.success(), "jjj fetch failed: {}", stderr);

    // 6. Verify Bob can see the problem and solution
    let output = run_jjj(bob_dir.path(), &["problem", "list"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Shared Problem"),
        "Bob should see Shared Problem after fetch. Got: {}",
        stdout
    );

    let output = run_jjj(bob_dir.path(), &["solution", "list"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Alice Solution"),
        "Bob should see Alice Solution after fetch. Got: {}",
        stdout
    );
}

#[test]
fn test_push_fetch_roundtrip() {
    if !test_helpers::jj_available() {
        return;
    }

    // 1. Create bare remote
    let remote_dir = create_bare_remote();

    // 2. Setup Alice's repo
    let alice_dir = setup_repo_with_remote(remote_dir.path());
    run_jjj(alice_dir.path(), &["init"]);

    // 3. Alice creates a problem and pushes
    run_jjj(alice_dir.path(), &["problem", "new", "Auth timeout bug"]);
    let output = run_jjj(alice_dir.path(), &["push", "--remote", "origin"]);
    assert!(output.status.success(), "Alice initial push failed");

    // 4. Bob clones and fetches
    let bob_dir = setup_repo_with_remote(remote_dir.path());
    run_jjj(bob_dir.path(), &["init"]);
    let output = run_jjj(bob_dir.path(), &["fetch", "--remote", "origin"]);
    assert!(output.status.success(), "Bob fetch failed");

    // 5. Bob adds a solution and pushes
    let output = run_jjj(
        bob_dir.path(),
        &[
            "solution",
            "new",
            "Token refresh fix",
            "--problem",
            "Auth timeout bug",
            "--force",
        ],
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    println!("Bob solution new stdout: {}", stdout);
    println!("Bob solution new stderr: {}", stderr);
    assert!(
        output.status.success(),
        "Bob solution new failed: {}",
        stderr
    );

    let output = run_jjj(bob_dir.path(), &["push", "--remote", "origin"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    println!("Bob push stdout: {}", stdout);
    println!("Bob push stderr: {}", stderr);
    assert!(output.status.success(), "Bob push failed: {}", stderr);

    // 6. Alice fetches Bob's changes
    let output = run_jjj(alice_dir.path(), &["fetch", "--remote", "origin"]);
    assert!(
        output.status.success(),
        "Alice fetch failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // 7. Verify Alice sees Bob's solution
    let output = run_jjj(alice_dir.path(), &["solution", "list"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Token refresh fix"),
        "Alice should see Bob's solution after fetch. Got: {}",
        stdout
    );
}

#[test]
fn test_push_dry_run() {
    if !test_helpers::jj_available() {
        return;
    }

    // 1. Create bare remote
    let remote_dir = create_bare_remote();

    // 2. Setup repo
    let repo_dir = setup_repo_with_remote(remote_dir.path());
    let dir = repo_dir.path();

    run_jjj(dir, &["init"]);
    run_jjj(dir, &["problem", "new", "Dry Run Test"]);

    // 3. Push with --dry-run
    let output = run_jjj(dir, &["push", "--remote", "origin", "--dry-run"]);
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success(), "Dry run should succeed");
    assert!(
        stdout.contains("Would push"),
        "Should indicate what would be pushed. Got: {}",
        stdout
    );

    // 4. Verify nothing was actually pushed
    let output = Command::new("git")
        .current_dir(remote_dir.path())
        .args(["branch", "-a"])
        .output()
        .expect("Failed to list branches");
    let branches = String::from_utf8_lossy(&output.stdout);
    assert!(
        !branches.contains("jjj"),
        "Dry run should not push. Remote branches: {}",
        branches
    );
}

#[test]
fn test_push_validates_before_pushing() {
    if !test_helpers::jj_available() {
        return;
    }

    // 1. Create bare remote
    let remote_dir = create_bare_remote();

    // 2. Setup repo
    let repo_dir = setup_repo_with_remote(remote_dir.path());
    let dir = repo_dir.path();

    let output = run_jjj(dir, &["init"]);
    assert!(
        output.status.success(),
        "init failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let output = run_jjj(dir, &["problem", "new", "Validation Test"]);
    assert!(
        output.status.success(),
        "problem new failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // 3. Push should validate and succeed
    let output = run_jjj(dir, &["push", "--remote", "origin"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    println!("Push stdout: {}", stdout);
    println!("Push stderr: {}", stderr);

    assert!(output.status.success(), "Push failed: {}", stderr);
    assert!(
        stdout.contains("Validating") || stdout.contains("checks passed"),
        "Should show validation. Got: {}",
        stdout
    );
}

/// Residual-race insurance: two clones misconfigured with the SAME pod id
/// contend on `jjj/dup`. The loser's push is rejected non-fast-forward; the
/// bounded backoff loop must re-fetch (three-way merging the winner's content
/// into local), rebuild the merge commit, and re-push to success — with NO data
/// loss on either side. Exercises `push_meta_bookmark_with_retry` +
/// `track_meta_bookmarks`.
#[test]
fn test_push_retry_recovers_from_same_pod_contention() {
    if !test_helpers::jj_available() {
        return;
    }

    let remote_dir = create_bare_remote();

    // Two clones, both (mis)configured as pod "dup".
    let c1 = setup_repo_with_remote(remote_dir.path());
    run_jjj(c1.path(), &["init"]);
    set_pod(c1.path(), "dup");
    let c2 = setup_repo_with_remote(remote_dir.path());
    run_jjj(c2.path(), &["init"]);
    set_pod(c2.path(), "dup");

    // c1 wins the race: creates P1 and pushes jjj/dup first.
    run_jjj(c1.path(), &["problem", "new", "P1 from c1", "--force"]);
    let out = run_jjj(c1.path(), &["push", "--remote", "origin"]);
    assert!(
        out.status.success(),
        "c1 push: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    // c2 creates P2 WITHOUT fetching, then pushes to the same jjj/dup → the
    // first attempt is rejected; the retry loop must reconcile and succeed.
    run_jjj(c2.path(), &["problem", "new", "P2 from c2", "--force"]);
    let out = run_jjj(c2.path(), &["push", "--remote", "origin", "--no-prompt"]);
    assert!(
        out.status.success(),
        "c2 push must recover via retry loop. stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    // c2's re-fetch must have merged c1's P1 into its own working set.
    let list = run_jjj(c2.path(), &["problem", "list"]);
    let stdout = String::from_utf8_lossy(&list.stdout);
    assert!(
        stdout.contains("P1 from c1") && stdout.contains("P2 from c2"),
        "both problems must survive the contended push on c2. Got:\n{stdout}"
    );

    // And c1, fetching afterwards, sees both — nothing was lost on the wire.
    run_jjj(c1.path(), &["fetch", "--remote", "origin"]);
    let list = run_jjj(c1.path(), &["problem", "list"]);
    let stdout = String::from_utf8_lossy(&list.stdout);
    assert!(
        stdout.contains("P1 from c1") && stdout.contains("P2 from c2"),
        "c1 must receive c2's merged push. Got:\n{stdout}"
    );
}

/// Force a pod identity into a clone's local sync state so it pushes to its own
/// single-writer bookmark `jjj/{pod}` (parallel per-pod branches). Written
/// before the first push, mirroring what the future `jjj sync` identity wiring
/// will persist.
fn set_pod(dir: &std::path::Path, pod: &str) {
    let meta = dir.join(".jj").join("jjj-meta");
    std::fs::create_dir_all(&meta).expect("create meta dir");
    std::fs::write(
        meta.join(".sync_state.json"),
        format!(r#"{{"version":1,"last_synced_rev":null,"pod":"{}"}}"#, pod),
    )
    .expect("write sync state");
}

/// The single problem markdown file in a clone (entity UUID is generated, so we
/// locate it rather than hard-code it).
fn only_problem_file(dir: &std::path::Path) -> std::path::PathBuf {
    let pdir = dir.join(".jj").join("jjj-meta").join("problems");
    std::fs::read_dir(&pdir)
        .expect("read problems dir")
        .flatten()
        .map(|e| e.path())
        .find(|p| p.extension().and_then(|x| x.to_str()) == Some("md"))
        .expect("a problem markdown file")
}

/// Edit one frontmatter scalar in place and bump `updated_at`, simulating a
/// local edit to a field.
fn edit_problem_scalar(dir: &std::path::Path, from: &str, to: &str, updated_at: &str) {
    let path = only_problem_file(dir);
    let content = std::fs::read_to_string(&path).expect("read problem");
    let content = content.replace(from, to);
    // Replace whatever updated_at line is present with the new timestamp.
    let content = content
        .lines()
        .map(|l| {
            if l.starts_with("updated_at:") {
                format!("updated_at: '{}'", updated_at)
            } else {
                l.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(&path, format!("{}\n", content)).expect("write problem");
}

/// The M1 keystone gate (design "Correctness parity" / audit P5): two pods on
/// **parallel per-pod branches** concurrently edit the *same* entity's
/// *different* scalar fields. The merge base must be the true common ancestor
/// `GCA(last_synced_rev, other_head)`, NOT `last_synced_rev` itself — the naive
/// reading reconstructs a base containing the fetching pod's own unpushed edit
/// and silently reverts it (audit 0.1). This test fails loudly on that
/// regression: both edits must survive.
#[test]
fn test_two_writer_parallel_branches_no_data_loss() {
    if !test_helpers::jj_available() {
        return;
    }

    let remote_dir = create_bare_remote();

    // Alice (pod=alice) creates the shared problem and pushes to jjj/alice.
    let alice = setup_repo_with_remote(remote_dir.path());
    run_jjj(alice.path(), &["init"]);
    set_pod(alice.path(), "alice");
    run_jjj(alice.path(), &["problem", "new", "Auth bug", "--force"]);
    let out = run_jjj(alice.path(), &["push", "--remote", "origin"]);
    assert!(
        out.status.success(),
        "alice initial push: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    // Bob (pod=bob) fetches the problem, then pushes to jjj/bob (descends
    // jjj/alice, so the branches share a real ancestor).
    let bob = setup_repo_with_remote(remote_dir.path());
    run_jjj(bob.path(), &["init"]);
    set_pod(bob.path(), "bob");
    let out = run_jjj(bob.path(), &["fetch", "--remote", "origin"]);
    assert!(
        out.status.success(),
        "bob fetch: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let out = run_jjj(bob.path(), &["push", "--remote", "origin"]);
    assert!(
        out.status.success(),
        "bob initial push: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    // Alice fetches Bob's (empty) branch so both clones know both bookmarks.
    run_jjj(alice.path(), &["fetch", "--remote", "origin"]);

    // CONCURRENT divergent edits to the SAME problem, NO fetch between them:
    //   Alice flips status  open -> in_progress
    //   Bob   changes title Auth bug -> Auth timeout
    edit_problem_scalar(
        alice.path(),
        "status: open",
        "status: in_progress",
        "2026-05-02T00:00:00Z",
    );
    edit_problem_scalar(
        bob.path(),
        "title: Auth bug",
        "title: Auth timeout",
        "2026-05-03T00:00:00Z",
    );

    // Both push to their own bookmarks — now jjj/alice and jjj/bob are PARALLEL
    // (each descends the shared {alice1, bob1}, neither descends the other).
    let out = run_jjj(alice.path(), &["push", "--remote", "origin"]);
    assert!(
        out.status.success(),
        "alice divergent push: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let out = run_jjj(bob.path(), &["push", "--remote", "origin"]);
    assert!(
        out.status.success(),
        "bob divergent push: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    // Alice fetches Bob's parallel branch. GCA(jjj/alice2, jjj/bob2) is the
    // shared merge of the first round — so the base for the diff has the
    // ORIGINAL field values, and the three-way merge keeps BOTH edits.
    let out = run_jjj(alice.path(), &["fetch", "--remote", "origin"]);
    assert!(
        out.status.success(),
        "alice final fetch: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let merged = std::fs::read_to_string(only_problem_file(alice.path())).expect("read merged");
    assert!(
        merged.contains("status: in_progress"),
        "DATA LOSS: Alice's own status edit was reverted by the fetch:\n{merged}"
    );
    assert!(
        merged.contains("title: Auth timeout"),
        "Bob's title edit was not merged in:\n{merged}"
    );
    assert!(
        !merged.contains("<<<<<<<"),
        "edits to different scalar fields must not conflict:\n{merged}"
    );

    // Second fetch with nothing new on the remote is a no-op (idempotent merge).
    let out = run_jjj(alice.path(), &["fetch", "--remote", "origin"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "alice no-op fetch: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        stdout.contains("No new jjj changes"),
        "a redundant fetch should report no changes, got:\n{stdout}"
    );
}

/// Regression for the rankings-never-synced bug (audit 0.3): per-user ranking
/// files (`rankings/{milestone}/{user}.json`) must travel through push/fetch so
/// global Borda+QV aggregation can see every collaborator. We write a ranking
/// file as the TUI's save_user_ordering would, push as Alice, and assert Bob
/// receives it on fetch.
#[test]
fn test_rankings_sync_roundtrip() {
    if !test_helpers::jj_available() {
        return;
    }

    let remote_dir = create_bare_remote();

    // Alice: init and stage a ranking file.
    let alice_dir = setup_repo_with_remote(remote_dir.path());
    run_jjj(alice_dir.path(), &["init"]);

    let rankings_rel = std::path::Path::new(".jj")
        .join("jjj-meta")
        .join("rankings")
        .join("m-test");
    let alice_rankings = alice_dir.path().join(&rankings_rel);
    std::fs::create_dir_all(&alice_rankings).expect("create rankings dir");
    let ordering = r#"{"order":["p1","p2"],"votes":{"p1":2},"updated_at":"2026-05-01T00:00:00Z"}"#;
    std::fs::write(alice_rankings.join("alice.json"), ordering).expect("write ranking");

    let output = run_jjj(alice_dir.path(), &["push", "--remote", "origin"]);
    assert!(
        output.status.success(),
        "Alice push failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Bob: init and fetch — the ranking file should arrive.
    let bob_dir = setup_repo_with_remote(remote_dir.path());
    run_jjj(bob_dir.path(), &["init"]);
    let output = run_jjj(bob_dir.path(), &["fetch", "--remote", "origin"]);
    assert!(
        output.status.success(),
        "Bob fetch failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let bob_ranking = bob_dir.path().join(&rankings_rel).join("alice.json");
    assert!(
        bob_ranking.exists(),
        "Bob should have received Alice's ranking file at {}",
        bob_ranking.display()
    );
    let got = std::fs::read_to_string(&bob_ranking).expect("read bob ranking");
    assert!(
        got.contains("\"p1\"") && got.contains("2026-05-01T00:00:00Z"),
        "ranking content should round-trip, got: {}",
        got
    );
}

/// Regression for the dump_to_markdown hazard (audit 3.6): markdown is the
/// source of truth. If the cache is dirty (an interrupted bulk load), the next
/// `Database::open` rebuilds it to EMPTY — and the old code unconditionally
/// dumped that empty DB back over the markdown during push, wiping it (and
/// pushing the wipe). Push must instead load markdown→DB, leaving files intact.
#[test]
fn test_push_does_not_wipe_markdown_when_cache_is_dirty() {
    if !test_helpers::jj_available() {
        return;
    }

    let remote_dir = create_bare_remote();
    let repo = setup_repo_with_remote(remote_dir.path());
    let path = repo.path();

    run_jjj(path, &["init"]);
    run_jjj(path, &["problem", "new", "Important problem", "--force"]);

    // Simulate an interrupted load: mark the cache dirty so push's
    // Database::open rebuilds it to empty.
    let db_path = path.join(".jj").join("jjj.db");
    {
        let db = jjj::db::Database::open(&db_path).expect("open db");
        jjj::db::set_dirty(&db, true).expect("set dirty");
    }

    let out = run_jjj(path, &["push", "--remote", "origin"]);
    assert!(
        out.status.success(),
        "push failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    // The markdown entity must still exist — not wiped by an empty-DB dump.
    let list = run_jjj(path, &["problem", "list"]);
    let stdout = String::from_utf8_lossy(&list.stdout);
    assert!(
        stdout.contains("Important problem"),
        "markdown was wiped by push after a dirty-cache rebuild! list output: {}",
        stdout
    );
    let problems_dir = path.join(".jj").join("jjj-meta").join("problems");
    let md_count = std::fs::read_dir(&problems_dir)
        .map(|d| {
            d.filter_map(|e| e.ok())
                .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some("md"))
                .count()
        })
        .unwrap_or(0);
    assert!(
        md_count >= 1,
        "problem markdown file should survive the push"
    );
}

/// Replace the markdown body (everything after the closing `---`) of the single
/// problem file, bumping `updated_at` so scalar LWW is deterministic.
fn edit_problem_body(dir: &std::path::Path, new_body: &str, updated_at: &str) {
    let path = only_problem_file(dir);
    let content = std::fs::read_to_string(&path).expect("read problem");
    // Frontmatter is the block between the opening `---` and the next `\n---`.
    let rest = content.strip_prefix("---").expect("leading ---");
    let end = rest.find("\n---").expect("frontmatter close");
    let front = rest[..end]
        .lines()
        .map(|l| {
            if l.starts_with("updated_at:") {
                format!("updated_at: '{}'", updated_at)
            } else {
                l.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    // `front` begins with the empty segment before the first newline, so joining
    // reproduces the leading newline; add the closing fence + body explicitly.
    std::fs::write(&path, format!("---{}\n---\n\n{}\n", front, new_body)).expect("write problem");
}

/// Bring `n` writers to a shared ancestor, then fan out: the merge of ALL heads
/// (`jj new head1 head2 head3 ...`) is only exercised with 2 heads elsewhere.
/// Three pods each add a DISTINCT problem on parallel branches; a fourth, empty
/// clone must cold-start and receive all three — proving `meta_head_commits`
/// unions 3+ refs and the per-head base loop merges them all without loss.
#[test]
fn test_three_writer_fanout_merges_all() {
    if !test_helpers::jj_available() {
        return;
    }
    let remote_dir = create_bare_remote();

    // Seed: alice creates the shared problem and pushes jjj/alice.
    let alice = setup_repo_with_remote(remote_dir.path());
    run_jjj(alice.path(), &["init"]);
    set_pod(alice.path(), "alice");
    run_jjj(alice.path(), &["problem", "new", "Shared root", "--force"]);
    assert!(
        run_jjj(alice.path(), &["push", "--remote", "origin"])
            .status
            .success(),
        "alice seed push"
    );

    // bob and carol each fetch the shared root, add their own problem, and push
    // to their own single-writer bookmark — three parallel heads on the remote.
    for (pod, title) in [("bob", "Bob problem"), ("carol", "Carol problem")] {
        let clone = setup_repo_with_remote(remote_dir.path());
        run_jjj(clone.path(), &["init"]);
        set_pod(clone.path(), pod);
        assert!(
            run_jjj(clone.path(), &["fetch", "--remote", "origin"])
                .status
                .success(),
            "{pod} fetch"
        );
        run_jjj(clone.path(), &["problem", "new", title, "--force"]);
        assert!(
            run_jjj(clone.path(), &["push", "--remote", "origin"])
                .status
                .success(),
            "{pod} push"
        );
    }

    // A fresh fourth clone cold-starts and must see ALL three problems.
    let dave = setup_repo_with_remote(remote_dir.path());
    run_jjj(dave.path(), &["init"]);
    assert!(
        run_jjj(dave.path(), &["fetch", "--remote", "origin"])
            .status
            .success(),
        "dave cold fetch"
    );
    let list = run_jjj(dave.path(), &["problem", "list"]);
    let stdout = String::from_utf8_lossy(&list.stdout);
    for expect in ["Shared root", "Bob problem", "Carol problem"] {
        assert!(
            stdout.contains(expect),
            "3-writer fanout lost {expect:?}. Got:\n{stdout}"
        );
    }
}

/// Genuine same-field divergence: two pods edit the SAME problem BODY to
/// different text. The three-way merge cannot auto-resolve, so conflict markers
/// land in the file — and the next push MUST be blocked by validation (audit
/// 0.4) so `<<<<<<<` text is never propagated to other clones.
#[test]
fn test_body_conflict_blocks_push() {
    if !test_helpers::jj_available() {
        return;
    }
    let remote_dir = create_bare_remote();

    let alice = setup_repo_with_remote(remote_dir.path());
    run_jjj(alice.path(), &["init"]);
    set_pod(alice.path(), "alice");
    run_jjj(alice.path(), &["problem", "new", "Shared", "--force"]);
    assert!(
        run_jjj(alice.path(), &["push", "--remote", "origin"])
            .status
            .success(),
        "alice seed push"
    );

    let bob = setup_repo_with_remote(remote_dir.path());
    run_jjj(bob.path(), &["init"]);
    set_pod(bob.path(), "bob");
    assert!(
        run_jjj(bob.path(), &["fetch", "--remote", "origin"])
            .status
            .success(),
        "bob fetch"
    );
    assert!(
        run_jjj(bob.path(), &["push", "--remote", "origin"])
            .status
            .success(),
        "bob seed push"
    );
    run_jjj(alice.path(), &["fetch", "--remote", "origin"]);

    // Divergent BODY edits, no fetch between — a true conflict.
    edit_problem_body(
        alice.path(),
        "Alice's analysis of root cause.",
        "2026-05-02T00:00:00Z",
    );
    edit_problem_body(
        bob.path(),
        "Bob's completely different take.",
        "2026-05-03T00:00:00Z",
    );
    assert!(
        run_jjj(alice.path(), &["push", "--remote", "origin"])
            .status
            .success(),
        "alice body push"
    );
    assert!(
        run_jjj(bob.path(), &["push", "--remote", "origin"])
            .status
            .success(),
        "bob body push"
    );

    // Alice fetches bob's parallel branch → conflict markers in the body.
    run_jjj(alice.path(), &["fetch", "--remote", "origin"]);
    let merged = std::fs::read_to_string(only_problem_file(alice.path())).expect("read merged");
    assert!(
        merged.contains("<<<<<<<"),
        "same-body divergence must produce conflict markers. Got:\n{merged}"
    );

    // The conflicted markdown must NOT be pushable — validation blocks it.
    let out = run_jjj(alice.path(), &["push", "--remote", "origin"]);
    assert!(
        !out.status.success(),
        "push of conflict-marked markdown must be blocked. stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        combined.to_lowercase().contains("conflict"),
        "blocked push should explain the conflict. Got:\n{combined}"
    );
}

/// Delete/edit conflict: the remote deletes an entity while we edited it locally
/// since the shared base. The fetch must KEEP our copy and surface the conflict,
/// never silently drop the local edit.
#[test]
fn test_delete_edit_conflict_keeps_local_copy() {
    if !test_helpers::jj_available() {
        return;
    }
    let remote_dir = create_bare_remote();

    let alice = setup_repo_with_remote(remote_dir.path());
    run_jjj(alice.path(), &["init"]);
    set_pod(alice.path(), "alice");
    run_jjj(alice.path(), &["problem", "new", "Contested", "--force"]);
    assert!(
        run_jjj(alice.path(), &["push", "--remote", "origin"])
            .status
            .success(),
        "alice seed push"
    );

    let bob = setup_repo_with_remote(remote_dir.path());
    run_jjj(bob.path(), &["init"]);
    set_pod(bob.path(), "bob");
    assert!(
        run_jjj(bob.path(), &["fetch", "--remote", "origin"])
            .status
            .success(),
        "bob fetch"
    );
    assert!(
        run_jjj(bob.path(), &["push", "--remote", "origin"])
            .status
            .success(),
        "bob seed push"
    );
    run_jjj(alice.path(), &["fetch", "--remote", "origin"]);

    // alice deletes the file; bob edits a scalar. Both push their branches.
    std::fs::remove_file(only_problem_file(alice.path())).expect("delete problem file");
    assert!(
        run_jjj(alice.path(), &["push", "--remote", "origin"])
            .status
            .success(),
        "alice delete push"
    );
    edit_problem_scalar(
        bob.path(),
        "status: open",
        "status: in_progress",
        "2026-05-04T00:00:00Z",
    );
    assert!(
        run_jjj(bob.path(), &["push", "--remote", "origin"])
            .status
            .success(),
        "bob edit push"
    );

    // bob fetches alice's deletion → delete/edit conflict → keep bob's edited copy.
    let out = run_jjj(bob.path(), &["fetch", "--remote", "origin"]);
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let list = run_jjj(bob.path(), &["problem", "list"]);
    let stdout = String::from_utf8_lossy(&list.stdout);
    assert!(
        stdout.contains("Contested"),
        "delete/edit conflict must KEEP the locally-edited entity. Got:\n{stdout}"
    );
    assert!(
        combined.to_lowercase().contains("delete") || combined.to_lowercase().contains("conflict"),
        "fetch should report the delete/edit conflict. Got:\n{combined}"
    );
}

/// Pillar 2 read parity: reads served from the clean DB (`list<T>`) must return
/// the same entities as the FS walk (`list_fs<T>`, used when the DB is dirty).
/// A regression here means the DB-primary path shows a different world than the
/// canonical markdown.
#[test]
fn test_db_primary_and_fs_reads_agree() {
    if !test_helpers::jj_available() {
        return;
    }
    let repo = test_helpers::setup_test_repo();
    let path = repo.path();

    run_jjj(path, &["problem", "new", "Alpha problem", "--force"]);
    run_jjj(path, &["problem", "new", "Beta problem", "--force"]);
    run_jjj(
        path,
        &[
            "solution",
            "new",
            "Fix alpha",
            "--problem",
            "Alpha problem",
            "--force",
        ],
    );

    // DB-primary read (cache is clean after the writes' synchronous upserts).
    let db_list = String::from_utf8_lossy(&run_jjj(path, &["problem", "list"]).stdout).into_owned();

    // Force the FS-walk fallback by marking the cache dirty.
    let db_path = path.join(".jj").join("jjj.db");
    {
        let db = jjj::db::Database::open(&db_path).expect("open db");
        jjj::db::set_dirty(&db, true).expect("set dirty");
    }
    let fs_list = String::from_utf8_lossy(&run_jjj(path, &["problem", "list"]).stdout).into_owned();

    for expect in ["Alpha problem", "Beta problem"] {
        assert!(
            db_list.contains(expect) && fs_list.contains(expect),
            "DB-primary and FS reads must agree on {expect:?}.\nDB:\n{db_list}\nFS:\n{fs_list}"
        );
    }
}

/// Scale smoke: a cold-start fetch of a mid-sized corpus must complete and adopt
/// every entity. Catches O(n²) surprises in the delta loop / DB rebuild that a
/// handful-of-entities test cannot. Kept modest so it runs in normal CI; the
/// full 25K/100K validation lives in `tools/bench/`.
#[test]
fn test_scale_cold_start_fetch_smoke() {
    if !test_helpers::jj_available() {
        return;
    }
    const N: usize = 400;
    let remote_dir = create_bare_remote();

    // Alice: write N problem markdown files directly into the meta tree, then push.
    let alice = setup_repo_with_remote(remote_dir.path());
    run_jjj(alice.path(), &["init"]);
    let problems = alice.path().join(".jj").join("jjj-meta").join("problems");
    std::fs::create_dir_all(&problems).expect("create problems dir");
    for i in 0..N {
        // UUID7-shaped ids so listing/sorting behaves as in production.
        let id = format!("01957d3e-a8b2-7def-8c3a-{:012x}", i);
        let md = format!(
            "---\nid: '{id}'\ntitle: Scale problem {i}\nstatus: open\npriority: medium\ncreated_at: '2026-05-01T00:00:00Z'\nupdated_at: '2026-05-01T00:00:00Z'\n---\n\nGenerated scale-smoke problem number {i}.\n"
        );
        std::fs::write(problems.join(format!("{id}.md")), md).expect("write problem");
    }
    // Rebuild alice's DB from the freshly-written markdown, then push.
    assert!(
        run_jjj(alice.path(), &["db", "rebuild"]).status.success(),
        "alice db rebuild"
    );
    assert!(
        run_jjj(alice.path(), &["push", "--remote", "origin"])
            .status
            .success(),
        "alice scale push"
    );

    // Bob cold-starts and must adopt all N.
    let bob = setup_repo_with_remote(remote_dir.path());
    run_jjj(bob.path(), &["init"]);
    let out = run_jjj(bob.path(), &["fetch", "--remote", "origin"]);
    assert!(
        out.status.success(),
        "bob cold fetch of {N} entities: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    // Spot-check a few across the range rather than parsing the whole list.
    let list =
        String::from_utf8_lossy(&run_jjj(bob.path(), &["problem", "list"]).stdout).into_owned();
    for i in [0usize, N / 2, N - 1] {
        assert!(
            list.contains(&format!("Scale problem {i}")),
            "cold-start scale fetch dropped 'Scale problem {i}'"
        );
    }
}
