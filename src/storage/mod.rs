use crate::error::{JjjError, Result};
use crate::jj::JjClient;
use crate::models::{Event, ProblemStatus, ProjectConfig, SolutionStatus};
use std::cell::RefCell;
use std::fs;
use std::path::PathBuf;

mod critiques;
mod events;
mod milestones;
mod problems;
mod solutions;

/// Write `content` to `path` atomically by writing to a uniquely-named `.tmp`
/// sibling first, then renaming. The temp name includes the process ID and
/// sub-second nanoseconds so concurrent writers cannot clobber each other's
/// temp file.
pub(super) fn atomic_write(path: &std::path::Path, content: &[u8]) -> std::io::Result<()> {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    let tmp = path.with_extension(format!("md.{}.{}.tmp", std::process::id(), nanos));
    std::fs::write(&tmp, content)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

pub(super) const META_BOOKMARK: &str = "jjj";
pub(super) const CONFIG_FILE: &str = "config.toml";
pub(super) const PROBLEMS_DIR: &str = "problems";
pub(super) const SOLUTIONS_DIR: &str = "solutions";
pub(super) const CRITIQUES_DIR: &str = "critiques";
pub(super) const MILESTONES_DIR: &str = "milestones";

/// The core storage abstraction for jjj metadata.
///
/// Manages reading/writing Problems, Solutions, Critiques, and Milestones from
/// an orphaned `jjj` bookmark stored in `.jj/jjj-meta/`. Each write goes through
/// [`with_metadata`](MetadataStore::with_metadata), which atomically commits the
/// change with an event appended to the commit message.
///
/// The metadata lives entirely outside the working copy — operations here never
/// touch the user's working changes.
pub struct MetadataStore {
    /// Path to the metadata directory (checked out from jjj bookmark)
    meta_path: PathBuf,

    /// JJ client for interacting with the repository
    pub jj_client: JjClient,

    /// JJ client for the metadata workspace
    pub meta_client: JjClient,

    /// Events to embed in the next commit description
    pending_events: RefCell<Vec<Event>>,
}

// =============================================================================
// Markdown Frontmatter Parsing
// =============================================================================

/// Parse YAML frontmatter from markdown content
fn parse_frontmatter<T: serde::de::DeserializeOwned>(content: &str) -> Result<(T, String)> {
    let content = content.trim();

    // Check for frontmatter delimiter
    if !content.starts_with("---") {
        return Err(JjjError::FrontmatterParse {
            entity_type: String::new(),
            entity_id: String::new(),
            message: "File must start with YAML frontmatter (---)".to_string(),
        });
    }

    // Find the closing delimiter
    let rest = &content[3..];
    let end_pos = rest
        .find("\n---")
        .ok_or_else(|| JjjError::FrontmatterParse {
            entity_type: String::new(),
            entity_id: String::new(),
            message: "Missing closing frontmatter delimiter".to_string(),
        })?;

    let yaml_str = &rest[..end_pos].trim();
    let body = rest[end_pos + 4..].trim().to_string();

    let frontmatter: T = serde_yml::from_str(yaml_str).map_err(|e| JjjError::FrontmatterParse {
        entity_type: String::new(),
        entity_id: String::new(),
        message: e.to_string(),
    })?;

    Ok((frontmatter, body))
}

/// Add entity context to a FrontmatterParse error
fn add_frontmatter_context(err: JjjError, entity_type: &str, entity_id: &str) -> JjjError {
    match err {
        JjjError::FrontmatterParse { message, .. } => JjjError::FrontmatterParse {
            entity_type: entity_type.to_string(),
            entity_id: entity_id.to_string(),
            message,
        },
        other => other,
    }
}

/// Serialize entity to markdown with YAML frontmatter
fn to_markdown<T: serde::Serialize>(frontmatter: &T, body: &str) -> Result<String> {
    let yaml = serde_yml::to_string(frontmatter)?;
    Ok(format!("---\n{}---\n\n{}", yaml, body))
}

impl MetadataStore {
    /// Create a new metadata store
    pub fn new(jj_client: JjClient) -> Result<Self> {
        let repo_root = jj_client.repo_root().to_path_buf();
        let meta_path = repo_root.join(".jj").join("jjj-meta");

        let meta_client = JjClient::with_root(meta_path.clone())?;

        Ok(Self {
            meta_path,
            jj_client,
            meta_client,
            pending_events: RefCell::new(Vec::new()),
        })
    }

    /// Get the path to the metadata directory
    pub fn meta_path(&self) -> &std::path::Path {
        &self.meta_path
    }

    /// Initialize the metadata store (create jjj bookmark)
    pub fn init(&self) -> Result<()> {
        // Check if already initialized
        if self.jj_client.bookmark_exists(META_BOOKMARK)? {
            return Err(crate::error::JjjError::Validation(
                "jjj is already initialized".to_string(),
            ));
        }

        // Create the meta workspace directly at root() so we never touch the
        // user's main working copy (fixes #3: init clobbers working copy).
        let meta_path_str = self
            .meta_path
            .to_str()
            .ok_or_else(|| JjjError::PathError(self.meta_path.clone()))?;
        self.jj_client
            .execute(&["workspace", "add", meta_path_str, "-r", "root()"])?;

        // Describe the new workspace's change from inside the meta workspace
        // so all operations stay in that workspace.
        self.meta_client.describe("Initialize jjj metadata")?;
        let change_id = self.meta_client.current_change_id()?;

        // Create the bookmark from the main workspace using --ignore-working-copy
        // to avoid stale-workspace errors from the workspace add above.
        self.jj_client.execute(&[
            "--ignore-working-copy",
            "bookmark",
            "create",
            META_BOOKMARK,
            "-r",
            &change_id,
        ])?;

        // Create initial structure
        fs::create_dir_all(self.meta_path.join(PROBLEMS_DIR))?;
        fs::create_dir_all(self.meta_path.join(SOLUTIONS_DIR))?;
        fs::create_dir_all(self.meta_path.join(CRITIQUES_DIR))?;
        fs::create_dir_all(self.meta_path.join(MILESTONES_DIR))?;

        // Create default config
        let default_config = ProjectConfig::default();
        self.save_config(&default_config)?;

        // Commit the initial structure
        self.commit_changes("Initialize jjj structure")?;

        Ok(())
    }

    /// Ensure the metadata workspace exists and is checked out from the `jjj` bookmark.
    ///
    /// Creates a new `jj workspace` pointing at `.jj/jjj-meta/` if the directory
    /// does not already exist. If the `jjj` bookmark itself does not yet exist,
    /// auto-initializes the metadata store so any command works on a fresh repo
    /// without requiring an explicit `jjj init`.
    pub(super) fn ensure_meta_checkout(&self) -> Result<()> {
        if !self.meta_path.exists() {
            let meta_path_str = self
                .meta_path
                .to_str()
                .ok_or_else(|| JjjError::PathError(self.meta_path.clone()))?;

            // Auto-initialize if the jjj bookmark has never been created.
            // Create workspace at root() so we never touch the user's main
            // working copy.
            if !self.jj_client.bookmark_exists(META_BOOKMARK)? {
                self.jj_client
                    .execute(&["workspace", "add", meta_path_str, "-r", "root()"])?;
                self.meta_client.describe("Initialize jjj metadata")?;
                let change_id = self.meta_client.current_change_id()?;
                self.jj_client.execute(&[
                    "--ignore-working-copy",
                    "bookmark",
                    "create",
                    META_BOOKMARK,
                    "-r",
                    &change_id,
                ])?;
            } else {
                self.jj_client.execute(&[
                    "workspace",
                    "add",
                    meta_path_str,
                    "-r",
                    META_BOOKMARK,
                ])?;
            }
            // Create required subdirectories inside the newly-checked-out workspace.
            fs::create_dir_all(self.meta_path.join(PROBLEMS_DIR))?;
            fs::create_dir_all(self.meta_path.join(SOLUTIONS_DIR))?;
            fs::create_dir_all(self.meta_path.join(CRITIQUES_DIR))?;
            fs::create_dir_all(self.meta_path.join(MILESTONES_DIR))?;
        }
        Ok(())
    }

    // =========================================================================
    // Config Operations
    // =========================================================================

    /// Load project configuration
    pub fn load_config(&self) -> Result<ProjectConfig> {
        self.ensure_meta_checkout()?;

        let config_path = self.meta_path.join(CONFIG_FILE);
        if !config_path.exists() {
            return Ok(ProjectConfig::default());
        }

        let content = fs::read_to_string(config_path)?;
        let config: ProjectConfig = toml::from_str(&content)?;
        Ok(config)
    }

    /// Save project configuration
    pub fn save_config(&self, config: &ProjectConfig) -> Result<()> {
        self.ensure_meta_checkout()?;

        let config_path = self.meta_path.join(CONFIG_FILE);
        let content = toml::to_string_pretty(config)?;
        fs::write(config_path, content)?;

        Ok(())
    }

    // =========================================================================
    // High-Level Operations
    // =========================================================================

    /// Check whether a problem can transition to `Solved` status.
    ///
    /// A problem is solvable if:
    /// 1. It has at least one `Approved` solution, **or**
    /// 2. All of its direct subproblems are `Solved`.
    ///
    /// Returns `(can_solve, reason)` where `reason` is non-empty when `can_solve`
    /// is `false` (explaining the blocker) or when it is `true` via subproblem path
    /// (confirming all subproblems are solved). Returns an error if the problem
    /// cannot be found.
    pub fn can_solve_problem(&self, problem_id: &str) -> Result<(bool, String)> {
        let problem = self.load_problem(problem_id)?;

        // Check if already solved
        if problem.status == ProblemStatus::Solved {
            return Ok((false, "Problem is already solved".to_string()));
        }

        // Check for approved solutions
        let solutions = self.list_solutions_for_problem(problem_id)?;
        let has_approved = solutions
            .iter()
            .any(|s| s.status == SolutionStatus::Approved);

        if has_approved {
            return Ok((true, String::new()));
        }

        // Check if all subproblems are solved
        let subproblems = self.list_subproblems(problem_id)?;
        if !subproblems.is_empty() {
            let all_solved = subproblems
                .iter()
                .all(|p| p.status == ProblemStatus::Solved);
            if all_solved {
                return Ok((true, "All subproblems are solved".to_string()));
            }
            return Ok((
                false,
                "Not all subproblems are solved and no approved solution exists".to_string(),
            ));
        }

        Ok((false, "No approved solution exists".to_string()))
    }

    /// Determine whether a solution is eligible for `Approved` status.
    ///
    /// A solution can be approved if:
    /// 1. It is not already in a finalized state (`Approved` or `Withdrawn`), **and**
    /// 2. It has no `Valid` critiques (validated critiques block approval).
    ///
    /// Open critiques do not block approval but produce a warning in the returned
    /// message. Returns `(can_approve, message)` where `message` may describe
    /// blockers or warnings.
    pub fn can_approve_solution(&self, solution_id: &str) -> Result<(bool, String)> {
        let solution = self.load_solution(solution_id)?;

        // Check if already finalized
        if solution.is_finalized() {
            return Ok((false, format!("Solution is already {:?}", solution.status)));
        }

        // Check for valid critiques
        if self.has_valid_critiques(solution_id)? {
            return Ok((
                false,
                "Solution has valid critiques that block approval".to_string(),
            ));
        }

        // Check for open critiques (warning but not blocking)
        let open_critiques = self.list_open_critiques_for_solution(solution_id)?;
        if !open_critiques.is_empty() {
            return Ok((
                true,
                format!(
                    "Warning: {} open critique(s) remain unaddressed",
                    open_critiques.len()
                ),
            ));
        }

        Ok((true, String::new()))
    }

    // =========================================================================
    // Commit Operations
    // =========================================================================

    /// Commit changes to the metadata
    pub fn commit_changes(&self, message: &str) -> Result<()> {
        // Embed all pending events as `jjj: <json>` lines in the commit
        // description. The commit history is the sole authoritative event
        // store; there is no separate events.jsonl file to update.
        let event_suffix = {
            let mut pending = self.pending_events.borrow_mut();
            if pending.is_empty() {
                String::new()
            } else {
                let lines: String = pending
                    .drain(..)
                    .filter_map(|e| e.to_commit_suffix().ok())
                    .collect::<Vec<_>>()
                    .join("\n");
                format!("\n\n{}", lines)
            }
        };

        let full_message = format!("{}{}", message, event_suffix);

        // The meta workspace may be stale from a previous bookmark set in the
        // main workspace (bookmark set advances the shared op log). Bring it
        // current before describing, which needs a non-stale working copy
        // to snapshot the metadata files we just wrote to disk.
        let _ = self.meta_client.execute(&["workspace", "update-stale"]);

        // Describe FIRST so jj snapshots the working-copy files into the
        // current change and attaches our message. Then `jj new` creates an
        // empty child, leaving the described change as @- with all content.
        // (The old order — new then describe — caused content to land in the
        // parent while the description went to an empty child: issue #4.)
        self.meta_client.describe(&full_message)?;
        self.meta_client.execute(&["new"])?;

        // The commit we care about is now @- (the parent of the fresh empty
        // change). Read its commit-id for the bookmark update. We use
        // commit_id rather than change_id because change IDs can become
        // divergent across workspaces, causing bookmark set to fail.
        let meta_change = self
            .meta_client
            .execute(&["log", "--no-graph", "-r", "@-", "-T", "commit_id"])?
            .trim()
            .to_string();
        self.jj_client.execute(&[
            "--ignore-working-copy",
            "bookmark",
            "set",
            META_BOOKMARK,
            "-r",
            &meta_change,
            "--allow-backwards",
        ])?;

        Ok(())
    }

    /// Execute an operation on the metadata store and atomically commit the result.
    ///
    /// This is the primary mechanism for all metadata writes. The `operation`
    /// closure runs first; if it succeeds, changes are committed to the `jjj`
    /// bookmark with `message` as the commit description. Any events queued
    /// via [`set_pending_event`](MetadataStore::set_pending_event) are
    /// embedded as `jjj: <json>` lines in the commit description.
    ///
    /// The commit history is the sole authoritative event store — there is no
    /// separate cache file. Events are replayed from `::@` on demand.
    ///
    /// If `operation` returns an error, no commit is made.
    pub fn with_metadata<F, R>(&self, message: &str, operation: F) -> Result<R>
    where
        F: FnOnce() -> Result<R>,
    {
        let result = operation()?;
        self.commit_changes(message)?;
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Confidence, Priority, ProblemFrontmatter};
    use chrono::Utc;

    #[test]
    fn test_parse_frontmatter() {
        let content = r#"---
id: p1
title: Test Problem
status: open
priority: medium
created_at: 2024-01-15T10:30:00Z
updated_at: 2024-01-15T10:30:00Z
---

## Description

This is a test problem.

## Context

Some context here.
"#;

        let (frontmatter, body): (ProblemFrontmatter, String) = parse_frontmatter(content).unwrap();
        assert_eq!(frontmatter.id, "p1");
        assert_eq!(frontmatter.title, "Test Problem");
        assert!(body.contains("## Description"));
    }

    #[test]
    fn test_to_markdown() {
        let frontmatter = ProblemFrontmatter {
            id: "p1".to_string(),
            title: "Test".to_string(),
            parent_id: None,
            status: ProblemStatus::Open,
            priority: Priority::default(),
            confidence: Confidence::default(),
            solution_ids: vec![],
            child_ids: vec![],
            milestone_id: None,
            assignee: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            dissolved_reason: None,
            github_issue: None,
            tags: vec![],
        };

        let body = "Test description\n";
        let result = to_markdown(&frontmatter, body).unwrap();

        assert!(result.starts_with("---\n"));
        assert!(result.contains("id: p1"));
        assert!(result.contains("Test description"));
    }

    #[test]
    fn test_critique_frontmatter_with_reviewer() {
        use crate::models::{Critique, CritiqueFrontmatter};

        let mut critique = Critique::new(
            "c1".to_string(),
            "Awaiting review".to_string(),
            "s1".to_string(),
        );
        critique.reviewer = Some("bob".to_string());

        let frontmatter = CritiqueFrontmatter::from(&critique);
        let body = format!("{}\n", critique.argument);

        let markdown = to_markdown(&frontmatter, &body).unwrap();
        assert!(markdown.contains("reviewer: bob"));
    }
}
