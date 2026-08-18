mod test_helpers;

use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::process::Command;

/// Which program a block runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Runner {
    /// The jjj binary, with the block's words as argv.
    Jjj,
    /// `sh -c`, for setup and for asserting on files.
    Shell,
}

/// What the block's exit status is expected to be.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Expect {
    Success,
    Failure,
}

#[derive(Debug, Clone, Copy)]
struct BlockType {
    runner: Runner,
    expect: Expect,
    /// 0 = `$REPO`, 1 = `$REPO2`. A journey that names repo 2 must declare
    /// `mode: two-clone` in its frontmatter, or there is no second repo to run
    /// in and the journey fails loudly rather than silently using repo 1.
    repo: usize,
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
            let bt = trimmed.strip_prefix("```").and_then(parse_fence);

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
        } else if let Some(rest) = trimmed.strip_prefix(">= ") {
            if let Some(space_pos) = rest.find(' ') {
                let var = rest[..space_pos].to_string();
                let pattern = rest[space_pos + 1..].to_string();
                assertions.push(Assertion::Capture(var, pattern));
            }
        } else if let Some(rest) = trimmed.strip_prefix(">~ ") {
            assertions.push(Assertion::Matches(rest.to_string()));
        } else if let Some(rest) = trimmed.strip_prefix(">! ") {
            assertions.push(Assertion::NotContains(rest.to_string()));
        } else if let Some(rest) = trimmed.strip_prefix("> ") {
            assertions.push(Assertion::Contains(rest.to_string()));
        } else {
            command_lines.push(line.to_string());
        }
    }

    blocks
}

/// Parse a fence info string (everything after the opening backticks) into a
/// block type, or `None` if this fence is ordinary prose/code.
///
/// Grammar: `jjj|shell` then any order of `:2` (run in the second clone),
/// `:setup` (no assertions expected) and `:fail` (expect a non-zero exit).
fn parse_fence(info: &str) -> Option<BlockType> {
    let mut parts = info.trim().split(':');
    let runner = match parts.next()? {
        "jjj" => Runner::Jjj,
        "shell" => Runner::Shell,
        _ => return None,
    };
    let mut bt = BlockType {
        runner,
        expect: Expect::Success,
        repo: 0,
    };
    for modifier in parts {
        match modifier {
            "fail" => bt.expect = Expect::Failure,
            "setup" => {}
            "2" => bt.repo = 1,
            // An unrecognized modifier is almost certainly a typo in a journey.
            // Treating the fence as prose would silently skip the commands, so
            // refuse to claim the block at all and let the missing assertions
            // fail visibly.
            _ => return None,
        }
    }
    Some(bt)
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

fn run_jjj_block(jjj: &Path, dir: &Path, command: &str, env_path: Option<&str>) -> (bool, String) {
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

/// Create one colocated git+jj repo with a fixed identity.
///
/// `identity` names the actor so a two-clone journey can tell the two sides
/// apart in `jjj whoami` and in event authorship.
fn setup_journey_repo_as(identity: &str, email: &str) -> tempfile::TempDir {
    let dir = tempfile::TempDir::new().expect("create temp dir");

    // git init (matching UXR lib.sh colocated setup)
    let status = Command::new("git")
        .args(["init", "-q", "."])
        .current_dir(dir.path())
        .status()
        .expect("git must be installed");
    assert!(status.success(), "git init failed");

    Command::new("git")
        .args(["config", "user.name", identity])
        .current_dir(dir.path())
        .status()
        .unwrap();
    Command::new("git")
        .args(["config", "user.email", email])
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
        .args(["config", "set", "--repo", "user.name", identity])
        .current_dir(dir.path())
        .status()
        .ok();
    Command::new("jj")
        .args(["config", "set", "--repo", "user.email", email])
        .current_dir(dir.path())
        .status()
        .ok();

    dir
}

/// Everything a journey runs against: one repo, or two clones sharing a bare
/// remote when the journey declares `mode: two-clone`.
///
/// The bare remote is what makes a real multi-user journey possible — push and
/// fetch have somewhere to meet — and it is the piece the single-repo setup
/// could never provide.
struct JourneyEnv {
    repo: tempfile::TempDir,
    repo2: Option<tempfile::TempDir>,
    /// Held so the bare remote outlives the clones that point at it.
    _remote: Option<tempfile::TempDir>,
}

impl JourneyEnv {
    /// Working directory for a block, or `None` if it names a repo this
    /// journey did not declare.
    fn dir(&self, index: usize) -> Option<&Path> {
        match index {
            0 => Some(self.repo.path()),
            1 => self.repo2.as_ref().map(|d| d.path()),
            _ => None,
        }
    }
}

/// Whether a journey's frontmatter asks for two clones.
fn wants_two_clones(content: &str) -> bool {
    content
        .lines()
        .take_while(|l| !l.starts_with("## "))
        .any(|l| l.trim() == "mode: two-clone")
}

/// Point a colocated repo at a bare remote and pull its initial commit.
fn attach_remote(repo: &Path, remote: &Path) {
    let url = remote.to_string_lossy().into_owned();
    Command::new("git")
        .args(["remote", "add", "origin", &url])
        .current_dir(repo)
        .status()
        .ok();
    // jj discovers git remotes through the colocated .git, but the bookmark
    // tracking it needs only exists after a fetch.
    Command::new("jj")
        .args(["git", "fetch", "--remote", "origin"])
        .current_dir(repo)
        .output()
        .ok();
}

fn setup_journey_env(two_clone: bool) -> JourneyEnv {
    // Repo 1 keeps the historic identity: existing journeys assert against
    // "Test User", and a rename would silently change what `--assignee`
    // filters match.
    let repo = setup_journey_repo_as("Test User", "test@example.com");
    if !two_clone {
        return JourneyEnv {
            repo,
            repo2: None,
            _remote: None,
        };
    }

    let remote = tempfile::TempDir::new().expect("create remote dir");
    Command::new("git")
        .args(["init", "-q", "--bare", "."])
        .current_dir(remote.path())
        .status()
        .expect("git init --bare");

    attach_remote(repo.path(), remote.path());

    let repo2 = setup_journey_repo_as("Bob", "bob@example.com");
    attach_remote(repo2.path(), remote.path());

    JourneyEnv {
        repo,
        repo2: Some(repo2),
        _remote: Some(remote),
    }
}

fn run_journey(path: &Path) -> Vec<String> {
    let content = fs::read_to_string(path).unwrap();
    let blocks = extract_journey_blocks(&content);
    let mut failures = Vec::new();

    if blocks.is_empty() {
        return failures;
    }

    let env = setup_journey_env(wants_two_clones(&content));
    let jjj = test_helpers::jjj_binary();
    let mut vars: HashMap<String, String> = HashMap::new();
    vars.insert(
        "REPO".to_string(),
        env.repo.path().to_string_lossy().to_string(),
    );
    if let Some(second) = env.repo2.as_ref() {
        vars.insert(
            "REPO2".to_string(),
            second.path().to_string_lossy().to_string(),
        );
    }
    vars.insert("JJJ".to_string(), jjj.to_string_lossy().to_string());

    let rel_path = path.file_name().unwrap().to_string_lossy();

    for block in &blocks {
        let command = expand_vars(&block.command, &vars);

        let cwd = match env.dir(block.lang.repo) {
            Some(d) => d,
            None => {
                failures.push(format!(
                    "{}:{} -- block targets repo {} but the journey did not declare \
                     `mode: two-clone` in its frontmatter",
                    rel_path,
                    block.line_number,
                    block.lang.repo + 1,
                ));
                break;
            }
        };

        // Check for fake-bin PATH on each iteration (may be created mid-journey)
        let env_path = get_modified_path(cwd);
        let env_path_ref = env_path.as_deref();

        let (success, output) = match block.lang.runner {
            Runner::Jjj => run_jjj_block(&jjj, cwd, &command, env_path_ref),
            Runner::Shell => run_shell_block(cwd, &command, env_path_ref),
        };

        // Check exit code expectation
        let expect_success = block.lang.expect == Expect::Success;
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
                                let val = caps.get(1).unwrap_or_else(|| caps.get(0).unwrap());
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
                }
            }
        }
    }

    failures
}

/// Run one journey file by name, failing the test with every assertion that
/// did not hold.
///
/// Each journey gets its own `#[test]` (see the `journeys!` block below) so a
/// failure names the journey that broke, the suite runs them in parallel, and
/// one broken journey no longer hides the other seventeen.
fn run_journey_file(file_name: &str) {
    if !test_helpers::jj_available() {
        eprintln!("Skipping journey {}: jj not found", file_name);
        return;
    }

    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("journeys")
        .join(file_name);
    assert!(path.exists(), "journey file not found: {}", path.display());

    let failures = run_journey(&path);
    assert!(
        failures.is_empty(),
        "\n{} failure(s) in {}:\n\n{}",
        failures.len(),
        file_name,
        failures.join("\n\n"),
    );
}

/// Declare one `#[test]` per journey file.
///
/// The literal list is deliberate rather than a directory walk: it makes the
/// test names visible in `cargo test` output and in CI logs. Drift is caught by
/// `every_journey_file_has_a_test` below, which fails if the two ever disagree.
macro_rules! journeys {
    ($($name:ident => $file:literal),* $(,)?) => {
        $(
            #[test]
            fn $name() {
                run_journey_file($file);
            }
        )*

        /// Every journey file declared above, for the completeness guard.
        const DECLARED_JOURNEYS: &[&str] = &[$($file),*];
    };
}

journeys! {
    solo_quickstart        => "01-solo-quickstart.md",
    team_workflow          => "02-team-workflow.md",
    new_contributor        => "03-new-contributor.md",
    conflict_resolution    => "04-conflict-resolution.md",
    error_recovery         => "05-error-recovery.md",
    end_to_end_showcase    => "06-end-to-end-showcase.md",
    solution_lifecycle     => "07-solution-lifecycle.md",
    critique_validate      => "08-critique-validate.md",
    events_audit           => "09-events-audit.md",
    status_and_filtering   => "10-status-and-filtering.md",
    milestone_advanced     => "11-milestone-advanced.md",
    github_sync            => "12-github-sync.md",
    problem_reopen         => "13-problem-reopen.md",
    milestone_status       => "14-milestone-status.md",
    assignee_workflow      => "15-assignee-workflow.md",
    problem_graph          => "16-problem-graph.md",
    solution_diff          => "17-solution-diff.md",
    automation_rules       => "18-automation-rules.md",
    ranking                => "19-ranking.md",
    coordination           => "20-coordination.md",
    two_clone_sync         => "21-two-clone-sync.md",
    triage                 => "22-triage.md",
}

/// Guard against a journey file being added without a test to run it — the
/// failure mode that a directory walk avoids and a literal list invites.
#[test]
fn every_journey_file_has_a_test() {
    let journeys_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("journeys");
    let mut on_disk: Vec<String> = fs::read_dir(&journeys_dir)
        .expect("journeys/ must exist")
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(".md"))
        .collect();
    on_disk.sort();

    let mut declared: Vec<String> = DECLARED_JOURNEYS.iter().map(|s| s.to_string()).collect();
    declared.sort();

    assert_eq!(
        on_disk, declared,
        "journeys/ and the `journeys!` list in this file have drifted — add the \
         missing `#[test]` (or delete the stale entry)"
    );
}
