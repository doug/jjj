use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// A testable code block extracted from a markdown file
struct TestBlock {
    start_line: usize,
    lines: Vec<String>,
}

/// Extract ```bash,test code blocks from a markdown file
fn extract_test_blocks(content: &str) -> Vec<TestBlock> {
    let mut blocks = Vec::new();
    let mut in_block = false;
    let mut current_lines = Vec::new();
    let mut block_start = 0;
    let mut line_num = 0;

    for line in content.lines() {
        line_num += 1;
        if line.trim().starts_with("```bash,test") {
            in_block = true;
            current_lines.clear();
            block_start = line_num + 1;
        } else if in_block && line.trim() == "```" {
            in_block = false;
            if !current_lines.is_empty() {
                blocks.push(TestBlock {
                    start_line: block_start,
                    lines: current_lines.clone(),
                });
            }
        } else if in_block {
            current_lines.push(line.to_string());
        }
    }
    blocks
}

fn jjj_binary() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("target");
    path.push("debug");
    path.push("jjj");
    path
}

fn setup_doc_test_repo() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();

    // Initialize jj repo
    let output = Command::new("jj")
        .args(["git", "init"])
        .current_dir(dir.path())
        .output()
        .expect("jj must be installed for doc tests");
    assert!(output.status.success(), "jj git init failed: {}",
        String::from_utf8_lossy(&output.stderr));

    // Configure user
    Command::new("jj")
        .args(["config", "set", "--repo", "user.name", "Test User"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    Command::new("jj")
        .args(["config", "set", "--repo", "user.email", "test@example.com"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    dir
}

fn run_doc_command(dir: &Path, cmd_line: &str) -> (bool, String, String) {
    let parts: Vec<&str> = cmd_line.split_whitespace().collect();
    if parts.is_empty() {
        return (true, String::new(), String::new());
    }

    // Only run jjj commands
    if parts[0] != "jjj" {
        return (true, String::new(), String::new());
    }

    let output = Command::new(jjj_binary())
        .args(&parts[1..])
        .current_dir(dir)
        .output()
        .expect("failed to run jjj command");

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    (output.status.success(), stdout, stderr)
}

#[test]
fn test_documentation_examples() {
    // Skip if jj not installed
    if which::which("jj").is_err() {
        eprintln!("Skipping doc tests: jj not found");
        return;
    }

    let docs_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("docs");
    if !docs_dir.exists() {
        eprintln!("Skipping doc tests: docs/ directory not found");
        return;
    }

    let mut failures: Vec<String> = Vec::new();
    let mut tested_files = 0;
    let mut tested_commands = 0;

    for entry in walkdir::WalkDir::new(&docs_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|ext| ext == "md").unwrap_or(false))
    {
        let path = entry.path();

        // Skip plan files (they contain spec examples, not runnable commands)
        if path.to_string_lossy().contains("/plans/") {
            continue;
        }

        let content = fs::read_to_string(path).unwrap();
        let blocks = extract_test_blocks(&content);

        if blocks.is_empty() {
            continue;
        }

        let rel_path = path.strip_prefix(&docs_dir).unwrap();
        let dir = setup_doc_test_repo();
        tested_files += 1;

        // Track the last command's stdout for expect checks
        let mut last_stdout = String::new();

        for block in &blocks {
            for (i, line) in block.lines.iter().enumerate() {
                let trimmed = line.trim();

                // Skip empty lines
                if trimmed.is_empty() {
                    continue;
                }

                // Handle expect assertions (check against previous command's stdout)
                if trimmed.starts_with("# expect:") {
                    let expected = trimmed.strip_prefix("# expect:").unwrap().trim().trim_matches('"');
                    if !last_stdout.contains(expected) {
                        failures.push(format!(
                            "{}:{} -- expected '{}' in output\nstdout: {}",
                            rel_path.display(),
                            block.start_line + i,
                            expected,
                            last_stdout.trim(),
                        ));
                    }
                    continue;
                }

                // Skip other comments
                if trimmed.starts_with('#') {
                    continue;
                }

                // Run the command
                let (success, stdout, stderr) = run_doc_command(dir.path(), trimmed);
                tested_commands += 1;

                if !success {
                    failures.push(format!(
                        "{}:{} -- command failed: {}\nstderr: {}",
                        rel_path.display(),
                        block.start_line + i,
                        trimmed,
                        stderr.trim(),
                    ));
                    break; // Stop this block on first failure
                }

                last_stdout = stdout;
            }
        }
    }

    eprintln!("Doc tests: {} files, {} commands tested", tested_files, tested_commands);

    if !failures.is_empty() {
        panic!(
            "\n{} documentation test(s) failed:\n\n{}",
            failures.len(),
            failures.join("\n\n"),
        );
    }
}
