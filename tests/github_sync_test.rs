//! Hermetic GitHub sync tests.
//!
//! jjj reaches GitHub only by shelling out to `gh`, so putting a stub earlier on
//! `PATH` exercises every line of the real integration — argument construction,
//! JSON parsing, state reconciliation, event emission — with no network, no
//! credentials, and no shared repository.
//!
//! This replaces a suite that pointed at a live personal repository and gated
//! itself on `gh auth status`. On a CI runner that gate always failed, the tests
//! returned early, and the suite reported green having exercised none of this
//! code. The live suite still exists (`github_live_test.rs`) for pre-release
//! verification against the real API; this one is the version that runs on every
//! commit.
//!
//! The stub logs its argv to `FAKE_GH_LOG`, so tests can assert on *how* `gh`
//! was invoked — the half of the contract that stdout assertions cannot reach.

mod test_helpers;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use tempfile::TempDir;
use test_helpers::{jj_available, setup_test_repo};

/// Path to the checked-in stub directory.
fn fake_gh_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("fake-gh")
}

/// A repo wired to the stub, with the log and state dir the stub writes to.
struct GhRepo {
    repo: TempDir,
    log: PathBuf,
    state: PathBuf,
    /// Held so the bare remote outlives the repo pointing at it.
    _remote: Option<TempDir>,
}

/// A config with GitHub enabled and a fixed repo, so detection never depends on
/// the machine the test runs on.
const GITHUB_CONFIG: &str = "\
default_reviewers = []

[settings]

[github]
enabled = true
repo = \"testowner/testrepo\"
auto_push = false
sync_critiques = true
sync_lgtm = true
auto_close_on_solve = false
problem_label = \"jjj\"

[github.label_priority]

[sync]
";

impl GhRepo {
    fn new() -> Self {
        Self::build(false)
    }

    /// A repo that additionally has a bare git remote, which PR creation needs:
    /// jjj sets a bookmark on the solution's change and pushes it before asking
    /// `gh` to open the PR.
    fn with_remote() -> Self {
        Self::build(true)
    }

    fn build(with_remote: bool) -> Self {
        let repo = setup_test_repo();
        let log = repo.path().join("gh-invocations.log");
        let state = repo.path().join("gh-state");

        // The repo must be configured, or jjj auto-detects "no GitHub here" and
        // every command becomes a no-op that would pass any assertion. Rewrite
        // the whole file rather than appending: `jjj init` already writes a
        // `[github]` table, and a second one is a TOML parse error.
        let config_path = repo.path().join(".jj/jjj-meta/config.toml");
        fs::write(&config_path, GITHUB_CONFIG).expect("write github config");

        let remote = if with_remote {
            let remote = TempDir::new().expect("remote dir");
            Command::new("git")
                .args(["init", "-q", "--bare", "."])
                .current_dir(remote.path())
                .status()
                .expect("git init --bare");
            Command::new("git")
                .args(["remote", "add", "origin", &remote.path().to_string_lossy()])
                .current_dir(repo.path())
                .status()
                .expect("git remote add");
            Some(remote)
        } else {
            None
        };

        Self {
            repo,
            log,
            state,
            _remote: remote,
        }
    }

    /// Put a real jj change behind a solution, so PR creation has a branch to
    /// push.
    fn attach_change(&self, solution: &str) {
        Command::new("jj")
            .args(["new", "-m", "work"])
            .current_dir(self.repo.path())
            .output()
            .expect("jj new");
        fs::write(self.repo.path().join("work.rs"), "fn work() {}").expect("write file");
        self.jjj_ok(&["solution", "attach", solution]);
    }

    fn path(&self) -> &Path {
        self.repo.path()
    }

    /// Run jjj with the stub ahead of the real `gh` on PATH.
    fn jjj(&self, args: &[&str]) -> Output {
        let path = format!(
            "{}:{}",
            fake_gh_dir().display(),
            std::env::var("PATH").unwrap_or_default()
        );
        Command::new(test_helpers::jjj_binary())
            .args(args)
            .current_dir(self.repo.path())
            .env("PATH", path)
            .env("FAKE_GH_LOG", &self.log)
            .env("FAKE_GH_STATE", &self.state)
            .output()
            .expect("run jjj")
    }

    fn jjj_ok(&self, args: &[&str]) -> String {
        let out = self.jjj(args);
        assert!(
            out.status.success(),
            "jjj {} failed\nstdout: {}\nstderr: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr),
        );
        String::from_utf8_lossy(&out.stdout).into_owned()
    }

    /// Every `gh` invocation so far, one per line.
    fn gh_calls(&self) -> Vec<String> {
        fs::read_to_string(&self.log)
            .unwrap_or_default()
            .lines()
            .map(str::to_string)
            .collect()
    }

    /// Whether any invocation contains all of the given fragments.
    fn called_with(&self, fragments: &[&str]) -> bool {
        self.gh_calls()
            .iter()
            .any(|call| fragments.iter().all(|f| call.contains(f)))
    }
}

// =============================================================================
// Import
// =============================================================================

#[test]
fn importing_an_issue_creates_a_problem_with_its_metadata() {
    if !jj_available() {
        return;
    }
    let gh = GhRepo::new();

    let out = gh.jjj_ok(&["github", "import", "42"]);
    assert!(out.contains("Login is slow"), "import output: {out}");

    let listed = gh.jjj_ok(&["problem", "list"]);
    assert!(listed.contains("Login is slow when session expires"));

    // The "high" label must map onto priority — the mapping is the entire point
    // of importing rather than copy-pasting a title.
    let shown = gh.jjj_ok(&["problem", "list", "--json"]);
    assert!(
        shown.contains("\"priority\": \"high\""),
        "issue label should map to priority: {shown}"
    );
}

#[test]
fn importing_the_same_issue_twice_is_idempotent() {
    if !jj_available() {
        return;
    }
    let gh = GhRepo::new();

    gh.jjj_ok(&["github", "import", "42"]);
    let before = gh.jjj_ok(&["problem", "list", "--json"]);

    let second = gh.jjj(&["github", "import", "42"]);
    let after = gh.jjj_ok(&["problem", "list", "--json"]);

    let count = |json: &str| json.matches("\"id\"").count();
    assert_eq!(
        count(&before),
        count(&after),
        "re-importing created a duplicate problem\nsecond run: {}{}",
        String::from_utf8_lossy(&second.stdout),
        String::from_utf8_lossy(&second.stderr),
    );
}

#[test]
fn importing_a_closed_issue_does_not_reopen_it() {
    if !jj_available() {
        return;
    }
    let gh = GhRepo::new();

    // Issue 44 is CLOSED upstream. If the import drops that state, the problem
    // lands Open, and the next push "reconciles" by reopening the issue —
    // mutating the remote against the maintainer's intent.
    gh.jjj_ok(&["github", "import", "44"]);
    gh.jjj(&["github", "push"]);

    assert!(
        !gh.called_with(&["issue reopen"]),
        "importing a closed issue caused a reopen: {:?}",
        gh.gh_calls()
    );
}

// =============================================================================
// Pull requests
// =============================================================================

#[test]
fn pr_create_passes_the_requested_base_branch() {
    if !jj_available() {
        return;
    }
    let gh = GhRepo::with_remote();
    gh.jjj_ok(&["problem", "new", "Session timeout", "--force"]);
    gh.jjj_ok(&[
        "solution",
        "new",
        "Add a TTL",
        "--problem",
        "Session timeout",
        "--force",
    ]);
    gh.attach_change("Add a TTL");

    gh.jjj(&["github", "pr", "Add a TTL", "--base", "develop"]);

    // Asserting on stdout would pass even if `--base` were dropped on the floor;
    // only the recorded argv shows whether it reached `gh`.
    assert!(
        gh.called_with(&["pr create", "--base", "develop"]),
        "--base was not passed through to gh: {:?}",
        gh.gh_calls()
    );
}

#[test]
fn merging_a_pr_reports_through_to_status() {
    if !jj_available() {
        return;
    }
    let gh = GhRepo::with_remote();
    gh.jjj_ok(&["problem", "new", "Session timeout", "--force"]);
    gh.jjj_ok(&[
        "solution",
        "new",
        "Add a TTL",
        "--problem",
        "Session timeout",
        "--force",
    ]);
    gh.attach_change("Add a TTL");
    gh.jjj(&["github", "pr", "Add a TTL"]);

    let merged = gh.jjj(&["github", "merge", "Add a TTL"]);
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&merged.stdout),
        String::from_utf8_lossy(&merged.stderr)
    );

    assert!(
        gh.called_with(&["pr merge"]) || combined.contains("no linked PR"),
        "merge neither called gh nor explained why: {combined}\n{:?}",
        gh.gh_calls()
    );
}

// =============================================================================
// Local-first defaults
// =============================================================================

#[test]
fn creating_a_problem_locally_does_not_touch_github() {
    if !jj_available() {
        return;
    }
    let gh = GhRepo::new();

    gh.jjj_ok(&["problem", "new", "Purely local work", "--force"]);

    // `auto_push` is off by default and must stay that way: a tracker that
    // files a public issue the moment you jot a note is not offline-first.
    assert!(
        !gh.called_with(&["issue create"]),
        "creating a problem opened a GitHub issue without auto_push: {:?}",
        gh.gh_calls()
    );
}

#[test]
fn github_status_reports_links_without_mutating_anything() {
    if !jj_available() {
        return;
    }
    let gh = GhRepo::new();
    gh.jjj_ok(&["github", "import", "42"]);

    let before = gh.gh_calls().len();
    gh.jjj_ok(&["github", "status"]);
    let mutating: Vec<_> = gh
        .gh_calls()
        .into_iter()
        .skip(before)
        .filter(|c| {
            c.contains("issue create")
                || c.contains("issue close")
                || c.contains("issue reopen")
                || c.contains("pr create")
                || c.contains("pr merge")
        })
        .collect();

    assert!(
        mutating.is_empty(),
        "`github status` performed writes: {mutating:?}"
    );
}

#[test]
fn dry_run_never_calls_a_mutating_gh_command() {
    if !jj_available() {
        return;
    }
    let gh = GhRepo::new();
    gh.jjj_ok(&["problem", "new", "Session timeout", "--force"]);

    gh.jjj(&["github", "--dry-run", "close", "Session timeout"]);

    assert!(
        !gh.called_with(&["issue close"]),
        "--dry-run closed an issue for real: {:?}",
        gh.gh_calls()
    );
}

// =============================================================================
// The stub itself
// =============================================================================

#[test]
fn the_stub_fails_loudly_on_an_unhandled_command() {
    // A stub that returns empty output for anything it does not recognize turns
    // a new call site into a silently-passing test. This one must exit non-zero.
    let out = Command::new(fake_gh_dir().join("gh"))
        .args(["release", "create", "v9.9.9"])
        .output()
        .expect("run stub");

    assert!(!out.status.success(), "unhandled commands must fail");
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("unhandled"),
        "the failure should name the problem"
    );
}
