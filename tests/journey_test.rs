mod test_helpers;

use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone)]
enum BlockType {
    Jjj,
    JjjFail,
    JjjSetup,
    Shell,
    ShellSetup,
    ShellFail,
}

#[derive(Debug)]
enum Assertion {
    Contains(String),
    NotContains(String),
    Matches(String),
    Capture(String, String),
}

#[derive(Debug)]
struct JourneyBlock {
    command: String,
    lang: BlockType,
    assertions: Vec<Assertion>,
    line_number: usize,
}

fn extract_journey_blocks(content: &str) -> Vec<JourneyBlock> {
    let mut blocks = Vec::new();
    let mut in_block = false;
    let mut block_type: Option<BlockType> = None;
    let mut command_lines: Vec<String> = Vec::new();
    let mut assertions: Vec<Assertion> = Vec::new();
    let mut block_start = 0;

    for (i, line) in content.lines().enumerate() {
        let line_num = i + 1;
        let trimmed = line.trim();

        if !in_block {
            let bt = match trimmed {
                "```jjj" => Some(BlockType::Jjj),
                "```jjj:fail" => Some(BlockType::JjjFail),
                "```jjj:setup" => Some(BlockType::JjjSetup),
                "```shell" => Some(BlockType::Shell),
                "```shell:setup" => Some(BlockType::ShellSetup),
                "```shell:fail" => Some(BlockType::ShellFail),
                _ => None,
            };

            if let Some(bt) = bt {
                in_block = true;
                block_type = Some(bt);
                command_lines.clear();
                assertions.clear();
                block_start = line_num + 1;
            }
        } else if trimmed == "```" {
            in_block = false;
            if let Some(bt) = block_type.take() {
                let command = command_lines.join("\n");
                if !command.trim().is_empty() {
                    blocks.push(JourneyBlock {
                        command,
                        lang: bt,
                        assertions: std::mem::take(&mut assertions),
                        line_number: block_start,
                    });
                }
            }
        } else if trimmed.starts_with(">= ") {
            let rest = &trimmed[3..];
            if let Some(space_pos) = rest.find(' ') {
                let var = rest[..space_pos].to_string();
                let pattern = rest[space_pos + 1..].to_string();
                assertions.push(Assertion::Capture(var, pattern));
            }
        } else if trimmed.starts_with(">~ ") {
            assertions.push(Assertion::Matches(trimmed[3..].to_string()));
        } else if trimmed.starts_with(">! ") {
            assertions.push(Assertion::NotContains(trimmed[3..].to_string()));
        } else if trimmed.starts_with("> ") {
            assertions.push(Assertion::Contains(trimmed[2..].to_string()));
        } else {
            command_lines.push(line.to_string());
        }
    }

    blocks
}

fn split_shell_args(cmd: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;

    for ch in cmd.chars() {
        match ch {
            '"' => {
                in_quotes = !in_quotes;
            }
            ' ' | '\t' | '\n' if !in_quotes => {
                if !current.is_empty() {
                    args.push(current.clone());
                    current.clear();
                }
            }
            _ => {
                current.push(ch);
            }
        }
    }
    if !current.is_empty() {
        args.push(current);
    }
    args
}

fn expand_vars(text: &str, vars: &HashMap<String, String>) -> String {
    let mut result = text.to_string();
    // Sort by key length (longest first) to avoid partial matches
    let mut sorted: Vec<_> = vars.iter().collect();
    sorted.sort_by(|a, b| b.0.len().cmp(&a.0.len()));
    for (k, v) in sorted {
        result = result.replace(&format!("${}", k), v);
    }
    result
}

/// If $REPO/fake-bin exists, return a modified PATH with it prepended.
fn get_modified_path(dir: &Path) -> Option<String> {
    let fake_bin = dir.join("fake-bin");
    if fake_bin.exists() {
        let current = std::env::var("PATH").unwrap_or_default();
        Some(format!("{}:{}", fake_bin.display(), current))
    } else {
        None
    }
}

fn run_jjj_block(
    jjj: &Path,
    dir: &Path,
    command: &str,
    env_path: Option<&str>,
) -> (bool, String) {
    let args = split_shell_args(command);
    let mut cmd = Command::new(jjj);
    cmd.args(&args).current_dir(dir);
    if let Some(path) = env_path {
        cmd.env("PATH", path);
    }
    let output = cmd.output().expect("failed to run jjj");
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    (output.status.success(), combined)
}

fn run_shell_block(dir: &Path, script: &str, env_path: Option<&str>) -> (bool, String) {
    let mut cmd = Command::new("sh");
    cmd.arg("-c").arg(script).current_dir(dir);
    if let Some(path) = env_path {
        cmd.env("PATH", path);
    }
    let output = cmd.output().expect("failed to run shell command");
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    (output.status.success(), combined)
}

fn truncate_output(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.trim().to_string()
    } else {
        format!("{}...(truncated)", &s[..max_len])
    }
}

fn setup_journey_repo() -> tempfile::TempDir {
    let dir = tempfile::TempDir::new().expect("create temp dir");

    // git init (matching UXR lib.sh colocated setup)
    let status = Command::new("git")
        .args(["init", "-q", "."])
        .current_dir(dir.path())
        .status()
        .expect("git must be installed");
    assert!(status.success(), "git init failed");

    Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(dir.path())
        .status()
        .unwrap();
    Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(dir.path())
        .status()
        .unwrap();

    Command::new("git")
        .args(["commit", "-q", "--allow-empty", "-m", "initial"])
        .current_dir(dir.path())
        .status()
        .unwrap();

    // jj git init --colocate (matching UXR lib.sh)
    Command::new("jj")
        .args(["git", "init", "--colocate"])
        .current_dir(dir.path())
        .stderr(std::process::Stdio::null())
        .output()
        .ok();

    Command::new("jj")
        .args(["config", "set", "--repo", "user.name", "Test User"])
        .current_dir(dir.path())
        .status()
        .ok();
    Command::new("jj")
        .args(["config", "set", "--repo", "user.email", "test@example.com"])
        .current_dir(dir.path())
        .status()
        .ok();

    dir
}

fn run_journey(path: &Path) -> Vec<String> {
    let content = fs::read_to_string(path).unwrap();
    let blocks = extract_journey_blocks(&content);
    let mut failures = Vec::new();

    if blocks.is_empty() {
        return failures;
    }

    let dir = setup_journey_repo();
    let jjj = test_helpers::jjj_binary();
    let mut vars: HashMap<String, String> = HashMap::new();
    vars.insert(
        "REPO".to_string(),
        dir.path().to_string_lossy().to_string(),
    );
    vars.insert(
        "JJJ".to_string(),
        jjj.to_string_lossy().to_string(),
    );

    let rel_path = path.file_name().unwrap().to_string_lossy();

    for block in &blocks {
        let command = expand_vars(&block.command, &vars);

        // Check for fake-bin PATH on each iteration (may be created mid-journey)
        let env_path = get_modified_path(dir.path());
        let env_path_ref = env_path.as_deref();

        let (success, output) = match &block.lang {
            BlockType::Jjj | BlockType::JjjFail | BlockType::JjjSetup => {
                run_jjj_block(&jjj, dir.path(), &command, env_path_ref)
            }
            BlockType::Shell | BlockType::ShellSetup | BlockType::ShellFail => {
                run_shell_block(dir.path(), &command, env_path_ref)
            }
        };

        // Check exit code expectation
        let expect_success =
            !matches!(&block.lang, BlockType::JjjFail | BlockType::ShellFail);
        let exit_ok = success == expect_success;

        if !exit_ok {
            let expected = if expect_success { "success" } else { "failure" };
            let got = if success { "success" } else { "failure" };
            failures.push(format!(
                "{}:{} -- expected {} but got {}\n  command: {}\n  output: {}",
                rel_path,
                block.line_number,
                expected,
                got,
                command.lines().next().unwrap_or(""),
                truncate_output(&output, 500),
            ));
            break; // stop journey on exit code mismatch
        }

        // Check assertions (expand vars in assertion text)
        for assertion in &block.assertions {
            match assertion {
                Assertion::Contains(text) => {
                    let expanded = expand_vars(text, &vars);
                    if !output.contains(expanded.as_str()) {
                        failures.push(format!(
                            "{}:{} -- output should contain '{}'\n  command: {}\n  output: {}",
                            rel_path,
                            block.line_number,
                            expanded,
                            command.lines().next().unwrap_or(""),
                            truncate_output(&output, 500),
                        ));
                    }
                }
                Assertion::NotContains(text) => {
                    let expanded = expand_vars(text, &vars);
                    if output.contains(expanded.as_str()) {
                        failures.push(format!(
                            "{}:{} -- output should NOT contain '{}'\n  command: {}\n  output: {}",
                            rel_path,
                            block.line_number,
                            expanded,
                            command.lines().next().unwrap_or(""),
                            truncate_output(&output, 500),
                        ));
                    }
                }
                Assertion::Matches(pattern) => {
                    let expanded = expand_vars(pattern, &vars);
                    match Regex::new(&expanded) {
                        Ok(re) => {
                            if !re.is_match(&output) {
                                failures.push(format!(
                                    "{}:{} -- output should match /{}/\n  command: {}\n  output: {}",
                                    rel_path,
                                    block.line_number,
                                    expanded,
                                    command.lines().next().unwrap_or(""),
                                    truncate_output(&output, 500),
                                ));
                            }
                        }
                        Err(e) => {
                            failures.push(format!(
                                "{}:{} -- invalid regex '{}': {}",
                                rel_path, block.line_number, expanded, e,
                            ));
                        }
                    }
                }
                Assertion::Capture(var, pattern) => {
                    let expanded = expand_vars(pattern, &vars);
                    match Regex::new(&expanded) {
                        Ok(re) => {
                            if let Some(caps) = re.captures(&output) {
                                let val =
                                    caps.get(1).unwrap_or_else(|| caps.get(0).unwrap());
                                vars.insert(var.clone(), val.as_str().to_string());
                            } else {
                                failures.push(format!(
                                    "{}:{} -- capture ${} failed, pattern /{}/ not found\n  command: {}\n  output: {}",
                                    rel_path,
                                    block.line_number,
                                    var,
                                    expanded,
                                    command.lines().next().unwrap_or(""),
                                    truncate_output(&output, 500),
                                ));
                            }
                        }
                        Err(e) => {
                            failures.push(format!(
                                "{}:{} -- invalid regex '{}': {}",
                                rel_path, block.line_number, expanded, e,
                            ));
                        }
                    }
                },
            }
        }
    }

    failures
}

#[test]
fn journey_tests() {
    if !test_helpers::jj_available() {
        eprintln!("Skipping journey tests: jj not found");
        return;
    }

    let journeys_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("journeys");
    if !journeys_dir.exists() {
        eprintln!("Skipping journey tests: journeys/ directory not found");
        return;
    }

    let mut entries: Vec<PathBuf> = fs::read_dir(&journeys_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|e| e == "md").unwrap_or(false))
        .collect();
    entries.sort();

    let mut all_failures: Vec<String> = Vec::new();
    let mut tested = 0;

    for path in &entries {
        let name = path.file_name().unwrap().to_string_lossy();
        eprintln!("Running journey: {}", name);
        let failures = run_journey(path);
        if failures.is_empty() {
            eprintln!("  PASS");
        } else {
            eprintln!("  FAIL ({} failures)", failures.len());
            all_failures.extend(failures);
        }
        tested += 1;
    }

    eprintln!("Journey tests: {} files tested", tested);

    if !all_failures.is_empty() {
        panic!(
            "\n{} journey test failure(s):\n\n{}",
            all_failures.len(),
            all_failures.join("\n\n"),
        );
    }

    assert!(tested > 0, "No journey files found in journeys/");
}
