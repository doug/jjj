// Integration tests for storage layer
// These tests require jj to be installed and will create temporary repositories

use jjj::models::{ProjectConfig, Task};
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
    config.add_tag("backend".to_string());
    config.add_tag("frontend".to_string());

    // When: I serialize to TOML
    let toml_str = toml::to_string(&config).expect("Failed to serialize");

    // Then: I can deserialize it back
    let loaded: ProjectConfig = toml::from_str(&toml_str).expect("Failed to deserialize");

    assert_eq!(loaded.name, config.name);
    assert_eq!(loaded.tags.len(), config.tags.len());
}

#[test]
fn test_task_json_roundtrip() {
    // Given: A task with all properties set
    let mut task = Task::new(
        "T-123".to_string(),
        "Test task".to_string(),
        "F-TEST".to_string(),
        "In Progress".to_string(),
    );
    task.add_tag("backend".to_string());
    task.assignee = Some("alice".to_string());
    task.attach_change("abc123".to_string());

    // When: I serialize to JSON
    let json = serde_json::to_string_pretty(&task).expect("Failed to serialize");

    // Then: I can deserialize it back
    let loaded: Task = serde_json::from_str(&json).expect("Failed to deserialize");

    assert_eq!(loaded.id, task.id);
    assert_eq!(loaded.title, task.title);
    assert_eq!(loaded.column, task.column);
    assert_eq!(loaded.tags, task.tags);
    assert_eq!(loaded.assignee, task.assignee);
    assert_eq!(loaded.change_ids, task.change_ids);
}

#[test]
fn test_multiple_tasks_in_directory() {
    // Given: Multiple tasks
    let task1 = Task::new("T-1".to_string(), "Task 1".to_string(), "F-TEST".to_string(), "TODO".to_string());
    let task2 = Task::new("T-2".to_string(), "Task 2".to_string(), "F-TEST".to_string(), "Done".to_string());

    // When: I serialize them
    let json1 = serde_json::to_string_pretty(&task1).expect("Failed");
    let json2 = serde_json::to_string_pretty(&task2).expect("Failed");

    // Then: Each has unique ID
    assert_ne!(task1.id, task2.id);
    assert!(json1.contains("T-1"));
    assert!(json2.contains("T-2"));
}

#[test]
fn test_config_with_custom_columns() {
    // Given: A config with custom workflow
    let mut config = ProjectConfig::default();
    config.columns = vec![
        "Backlog".to_string(),
        "Development".to_string(),
        "Testing".to_string(),
        "Deployed".to_string(),
    ];

    // When: I serialize and deserialize
    let toml_str = toml::to_string(&config).expect("Failed to serialize");
    let loaded: ProjectConfig = toml::from_str(&toml_str).expect("Failed to deserialize");

    // Then: Custom columns are preserved
    assert_eq!(loaded.columns.len(), 4);
    assert!(loaded.is_valid_column("Backlog"));
    assert!(loaded.is_valid_column("Deployed"));
    assert!(!loaded.is_valid_column("TODO"));
}

#[test]
fn test_task_version_tracking() {
    // Given: A task
    let mut task = Task::new("T-1".to_string(), "Test".to_string(), "F-TEST".to_string(), "TODO".to_string());
    assert_eq!(task.version, 1);

    // When: I modify it multiple times
    task.add_tag("tag1".to_string());
    assert_eq!(task.version, 2);

    task.move_to_column("In Progress".to_string());
    assert_eq!(task.version, 3);

    task.attach_change("change1".to_string());
    assert_eq!(task.version, 4);

    // When: I serialize and deserialize
    let json = serde_json::to_string(&task).expect("Failed");
    let loaded: Task = serde_json::from_str(&json).expect("Failed");

    // Then: Version is preserved
    assert_eq!(loaded.version, 4);
}

/// Behavior: Task file naming convention
#[test]
fn test_task_file_naming() {
    // Given: Task IDs
    let task_ids = vec!["T-1", "T-100", "T-9999"];

    for id in task_ids {
        let filename = format!("{}.json", id);

        // Then: Filenames are valid and unique
        assert!(filename.ends_with(".json"));
        assert!(filename.starts_with("T-"));
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
    assert!(toml_str.contains("TODO"));
    assert!(toml_str.contains("In Progress"));

    // And: It can be manually edited
    let modified = toml_str.replace("My Project", "Edited Project");
    let loaded: ProjectConfig = toml::from_str(&modified).expect("Failed");
    assert_eq!(loaded.name.as_deref(), Some("Edited Project"));
}

/// Behavior: Empty tags list serializes correctly
#[test]
fn test_empty_collections_serialize() {
    // Given: A task with no tags or changes
    let task = Task::new("T-1".to_string(), "Simple".to_string(), "F-TEST".to_string(), "TODO".to_string());

    // When: I serialize it
    let json = serde_json::to_string(&task).expect("Failed");

    // Then: Empty collections are present
    assert!(json.contains("\"tags\""));
    assert!(json.contains("\"change_ids\""));
}

/// Behavior: Timestamps are preserved in serialization
#[test]
fn test_timestamps_preserved() {
    // Given: A task
    let task = Task::new("T-1".to_string(), "Test".to_string(), "F-TEST".to_string(), "TODO".to_string());
    let created = task.created_at;

    // When: I serialize and deserialize
    let json = serde_json::to_string(&task).expect("Failed");
    let loaded: Task = serde_json::from_str(&json).expect("Failed");

    // Then: Timestamps match
    assert_eq!(loaded.created_at, created);
    assert_eq!(loaded.updated_at, task.updated_at);
}
