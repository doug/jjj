#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use tempfile::TempDir;

/// Get the path to the jjj binary built by cargo.
pub fn jjj_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_jjj"))
}

/// Creates an isolated jj repo with jjj initialized for testing
pub fn setup_test_repo() -> TempDir {
    let dir = TempDir::new().expect("Failed to create temp dir");

    let output = Command::new("jj")
        .args(["git", "init"])
        .current_dir(dir.path())
        .output()
        .expect("Failed to run jj git init");
    assert!(output.status.success(), "jj git init failed: {:?}", output);

    // Configure user for jj (use --repo to avoid polluting global config)
    Command::new("jj")
        .current_dir(dir.path())
        .args(["config", "set", "--repo", "user.name", "Test User"])
        .status()
        .expect("Failed to set user name");

    Command::new("jj")
        .current_dir(dir.path())
        .args(["config", "set", "--repo", "user.email", "test@example.com"])
        .status()
        .expect("Failed to set user email");

    let output = Command::new(jjj_binary())
        .arg("init")
        .current_dir(dir.path())
        .output()
        .expect("Failed to run jjj init");
    assert!(output.status.success(), "jjj init failed: {:?}", output);

    dir
}

pub fn run_jjj(dir: impl AsRef<Path>, args: &[&str]) -> Output {
    Command::new(jjj_binary())
        .args(args)
        .current_dir(dir.as_ref())
        .output()
        .expect("Failed to run jjj command")
}

pub fn run_jjj_success(dir: impl AsRef<Path>, args: &[&str]) -> String {
    let output = run_jjj(dir.as_ref(), args);
    assert!(
        output.status.success(),
        "Command failed: jjj {}\nstdout: {}\nstderr: {}",
        args.join(" "),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

/// Check if jj is available; tests should skip if not.
///
/// Locally, a missing `jj` returns `false` so the suite still runs on a machine
/// without jujutsu installed. In CI this silent-skip is a trap: the sync/data-loss
/// integration tests would report "passed" having executed nothing. Set
/// `JJJ_REQUIRE_JJ=1` (the CI does) to turn a missing `jj` into a hard failure so
/// those guards can never masquerade as green.
pub fn jj_available() -> bool {
    if jjj::jj::find_executable("jj").is_some() {
        return true;
    }
    if std::env::var_os("JJJ_REQUIRE_JJ").is_some() {
        panic!(
            "jj binary not found but JJJ_REQUIRE_JJ is set: the jj-backed \
             integration tests must run here, not silently skip. Install jujutsu \
             (jj) on this runner."
        );
    }
    false
}
