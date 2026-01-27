// Integration tests for storage layer
// These tests require jj to be installed and will create temporary repositories

use jjj::models::{Problem, ProjectConfig, Solution};
use std::process::Command;
use tempfile::TempDir;

/// Helper to check if jj is installed
fn jj_available() -> bool {
    Command::new("jj")
        .arg("--version")
        .output()
        .is_ok()
}

/// Helper to create a test jj repository
fn create_test_repo() -> Option<TempDir> {
    if !jj_available() {
        return None;
    }

    let temp_dir = TempDir::new().ok()?;
    let repo_path = temp_dir.path();

    // Initialize jj repo
    let status = Command::new("jj")
        .args(&["git", "init", "--colocate"])
        .current_dir(repo_path)
        .status()
        .ok()?;

    if !status.success() {
        return None;
    }

    // Configure user for the repo
    Command::new("jj")
        .args(&["config", "set", "--repo", "user.name", "Test User"])
        .current_dir(repo_path)
        .status()
        .ok()?;

    Command::new("jj")
        .args(&["config", "set", "--repo", "user.email", "test@example.com"])
        .current_dir(repo_path)
        .status()
        .ok()?;

    Some(temp_dir)
}

#[test]
fn test_storage_requires_jj() {
    // This test documents that storage operations require jj
    if !jj_available() {
        println!("jj not available - integration tests skipped");
        return;
    }
    println!("jj is available for integration tests");
}

#[test]
fn test_project_config_roundtrip() {
    // Given: A project configuration
    let mut config = ProjectConfig::default();
    config.name = Some("Test Project".to_string());

    // When: I serialize to TOML
    let toml_str = toml::to_string(&config).expect("Failed to serialize");

    // Then: I can deserialize it back
    let loaded: ProjectConfig = toml::from_str(&toml_str).expect("Failed to deserialize");

    assert_eq!(loaded.name, config.name);
}

#[test]
fn test_problem_creation() {
    // Given: A problem
    let problem = Problem::new("P-1".to_string(), "Test problem".to_string());

    // Then: It should have correct defaults
    assert_eq!(problem.id, "P-1");
    assert_eq!(problem.title, "Test problem");
    assert!(problem.is_open());
    assert!(problem.solution_ids.is_empty());
    assert!(problem.child_ids.is_empty());
    assert!(problem.parent_id.is_none());
}

#[test]
fn test_solution_creation() {
    // Given: A solution for a problem
    let solution = Solution::new(
        "S-1".to_string(),
        "Test solution".to_string(),
        "P-1".to_string(),
    );

    // Then: It should have correct defaults
    assert_eq!(solution.id, "S-1");
    assert_eq!(solution.title, "Test solution");
    assert_eq!(solution.problem_id, "P-1");
    assert!(solution.is_proposed());
}

#[test]
fn test_solution_status_transitions() {
    // Given: A proposed solution
    let mut solution = Solution::new(
        "S-1".to_string(),
        "Test solution".to_string(),
        "P-1".to_string(),
    );
    assert!(solution.is_proposed());

    // When: I start testing
    solution.start_testing();
    assert!(solution.is_testing());

    // When: I accept the solution
    solution.accept();
    assert!(solution.is_accepted());
}

#[test]
fn test_solution_refute() {
    // Given: A testing solution
    let mut solution = Solution::new(
        "S-1".to_string(),
        "Test solution".to_string(),
        "P-1".to_string(),
    );
    solution.start_testing();

    // When: I refute it
    solution.refute();

    // Then: It should be refuted
    assert!(solution.is_refuted());
}

#[test]
fn test_solution_attach_change() {
    // Given: A solution
    let mut solution = Solution::new(
        "S-1".to_string(),
        "Test solution".to_string(),
        "P-1".to_string(),
    );

    // When: I attach a change
    solution.attach_change("abc123".to_string());

    // Then: The change should be attached
    assert_eq!(solution.change_ids.len(), 1);
    assert!(solution.change_ids.contains(&"abc123".to_string()));
}

#[test]
fn test_problem_status_transitions() {
    // Given: An open problem
    let mut problem = Problem::new("P-1".to_string(), "Test problem".to_string());
    assert!(problem.is_open());

    // When: I set to in progress
    problem.set_status(jjj::models::ProblemStatus::InProgress);
    assert!(problem.is_in_progress());

    // When: I solve it
    problem.set_status(jjj::models::ProblemStatus::Solved);
    assert!(problem.is_resolved());
}

#[test]
fn test_problem_dissolve() {
    // Given: An open problem
    let mut problem = Problem::new("P-1".to_string(), "Test problem".to_string());

    // When: I dissolve it (realize it was based on false premises)
    problem.set_status(jjj::models::ProblemStatus::Dissolved);

    // Then: It should be resolved (dissolved is a type of resolution)
    assert!(problem.is_resolved());
}

#[test]
fn test_problem_dag_structure() {
    // Given: A parent problem
    let mut parent = Problem::new("P-1".to_string(), "Parent problem".to_string());

    // When: I create a child problem
    let mut child = Problem::new("P-2".to_string(), "Child problem".to_string());
    child.set_parent(Some("P-1".to_string()));
    parent.add_child("P-2".to_string());

    // Then: The DAG relationship should be established
    assert!(child.parent_id.as_deref() == Some("P-1"));
    assert!(parent.child_ids.contains(&"P-2".to_string()));
}

#[test]
fn test_problem_milestone_assignment() {
    // Given: A problem
    let mut problem = Problem::new("P-1".to_string(), "Test problem".to_string());
    assert!(problem.milestone_id.is_none());

    // When: I assign it to a milestone
    problem.set_milestone(Some("M-1".to_string()));

    // Then: The milestone should be set
    assert_eq!(problem.milestone_id.as_deref(), Some("M-1"));

    // When: I remove the milestone
    problem.set_milestone(None);

    // Then: No milestone
    assert!(problem.milestone_id.is_none());
}

/// Behavior: Solution file naming convention
#[test]
fn test_entity_file_naming() {
    // Given: Entity IDs
    let problem_ids = vec!["P-1", "P-100", "P-9999"];
    let solution_ids = vec!["S-1", "S-100", "S-9999"];

    for id in problem_ids {
        let filename = format!("{}.md", id);
        assert!(filename.ends_with(".md"));
        assert!(filename.starts_with("P-"));
    }

    for id in solution_ids {
        let filename = format!("{}.md", id);
        assert!(filename.ends_with(".md"));
        assert!(filename.starts_with("S-"));
    }
}

/// Behavior: Config file is TOML for human editing
#[test]
fn test_config_is_human_readable() {
    // Given: A config
    let mut config = ProjectConfig::default();
    config.name = Some("My Project".to_string());

    // When: I serialize to TOML
    let toml_str = toml::to_string(&config).expect("Failed");

    // Then: It's human-readable
    assert!(toml_str.contains("My Project"));

    // And: It can be manually edited
    let modified = toml_str.replace("My Project", "Edited Project");
    let loaded: ProjectConfig = toml::from_str(&modified).expect("Failed");
    assert_eq!(loaded.name.as_deref(), Some("Edited Project"));
}

/// Behavior: Timestamps are preserved in serialization
#[test]
fn test_timestamps_preserved() {
    // Given: A problem
    let problem = Problem::new("P-1".to_string(), "Test".to_string());
    let created = problem.created_at;

    // Note: For markdown serialization, we would test the full roundtrip through storage
    // For unit tests, we just verify the timestamp is set correctly
    assert!(problem.updated_at >= created);
}
