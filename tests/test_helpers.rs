use std::process::{Command, Output};
use tempfile::TempDir;

/// Creates an isolated jj repo with jjj initialized for testing
pub fn setup_test_repo() -> TempDir {
    let dir = TempDir::new().expect("Failed to create temp dir");

    let output = Command::new("jj")
        .args(["git", "init"])
        .current_dir(dir.path())
        .output()
        .expect("Failed to run jj git init");
    assert!(output.status.success(), "jj git init failed: {:?}", output);

    // Configure user for jj
    Command::new("jj")
        .current_dir(dir.path())
        .args(["config", "set", "--user", "user.name", "Test User"])
        .status()
        .expect("Failed to set user name");

    Command::new("jj")
        .current_dir(dir.path())
        .args(["config", "set", "--user", "user.email", "test@example.com"])
        .status()
        .expect("Failed to set user email");

    let output = Command::new(env!("CARGO_BIN_EXE_jjj"))
        .arg("init")
        .current_dir(dir.path())
        .output()
        .expect("Failed to run jjj init");
    assert!(output.status.success(), "jjj init failed: {:?}", output);

    dir
}

pub fn run_jjj(dir: &TempDir, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_jjj"))
        .args(args)
        .current_dir(dir.path())
        .output()
        .expect("Failed to run jjj command")
}

pub fn run_jjj_success(dir: &TempDir, args: &[&str]) -> String {
    let output = run_jjj(dir, args);
    assert!(
        output.status.success(),
        "Command failed: jjj {}\nstdout: {}\nstderr: {}",
        args.join(" "),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).to_string()
}

/// Check if jj is available; tests should skip if not
pub fn jj_available() -> bool {
    which::which("jj").is_ok()
}
