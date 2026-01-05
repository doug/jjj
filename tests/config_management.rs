use jjj::models::ProjectConfig;

/// Behavior: Creating default configuration
#[test]
fn test_default_project_config() {
    // Given: No existing configuration
    // When: I create a default config
    let config = ProjectConfig::default();

    // Then: It should have standard Kanban columns
    assert_eq!(config.columns.len(), 4);
    assert_eq!(config.columns[0], "TODO");
    assert_eq!(config.columns[1], "In Progress");
    assert_eq!(config.columns[2], "Review");
    assert_eq!(config.columns[3], "Done");

    // And: No project-specific settings
    assert!(config.name.is_none());
    assert!(config.tags.is_empty());
    assert!(config.default_reviewers.is_empty());
}

/// Behavior: Validating column names
#[test]
fn test_validate_column_names() {
    // Given: A config with specific columns
    let config = ProjectConfig::default();

    // Then: Valid columns should pass validation
    assert!(config.is_valid_column("TODO"));
    assert!(config.is_valid_column("In Progress"));
    assert!(config.is_valid_column("Review"));
    assert!(config.is_valid_column("Done"));

    // And: Invalid columns should fail
    assert!(!config.is_valid_column("Nonexistent"));
    assert!(!config.is_valid_column("todo")); // Case sensitive
}

/// Behavior: Adding custom columns
#[test]
fn test_add_custom_column() {
    // Given: A default config
    let mut config = ProjectConfig::default();
    let initial_count = config.columns.len();

    // When: I add a custom column
    config.add_column("Blocked".to_string());

    // Then: The column should be added
    assert_eq!(config.columns.len(), initial_count + 1);
    assert!(config.is_valid_column("Blocked"));
}

/// Behavior: Adding duplicate columns
#[test]
fn test_add_duplicate_column() {
    // Given: A config with existing columns
    let mut config = ProjectConfig::default();
    let initial_count = config.columns.len();

    // When: I try to add an existing column
    config.add_column("TODO".to_string());

    // Then: It should not be duplicated
    assert_eq!(config.columns.len(), initial_count);
}

/// Behavior: Removing columns
#[test]
fn test_remove_column() {
    // Given: A config with standard columns
    let mut config = ProjectConfig::default();

    // When: I remove a column
    let removed = config.remove_column("Review");

    // Then: The column should be gone
    assert!(removed);
    assert!(!config.is_valid_column("Review"));
    assert_eq!(config.columns.len(), 3);
}

/// Behavior: Removing non-existent columns
#[test]
fn test_remove_nonexistent_column() {
    // Given: A default config
    let mut config = ProjectConfig::default();
    let initial_count = config.columns.len();

    // When: I try to remove a column that doesn't exist
    let removed = config.remove_column("Nonexistent");

    // Then: Nothing should change
    assert!(!removed);
    assert_eq!(config.columns.len(), initial_count);
}

/// Behavior: Adding tags to config
#[test]
fn test_add_tags_to_config() {
    // Given: A config with no tags
    let mut config = ProjectConfig::default();
    assert!(config.tags.is_empty());

    // When: I add tags
    config.add_tag("backend".to_string());
    config.add_tag("frontend".to_string());
    config.add_tag("database".to_string());

    // Then: All tags should be present
    assert_eq!(config.tags.len(), 3);
    assert!(config.tags.contains(&"backend".to_string()));
    assert!(config.tags.contains(&"frontend".to_string()));
    assert!(config.tags.contains(&"database".to_string()));
}

/// Behavior: Adding duplicate tags
#[test]
fn test_add_duplicate_tag() {
    // Given: A config with existing tags
    let mut config = ProjectConfig::default();
    config.add_tag("backend".to_string());
    let initial_count = config.tags.len();

    // When: I try to add the same tag again
    config.add_tag("backend".to_string());

    // Then: It should not be duplicated
    assert_eq!(config.tags.len(), initial_count);
}

/// Behavior: Custom project settings
#[test]
fn test_custom_project_settings() {
    // Given: A config
    let mut config = ProjectConfig::default();

    // When: I add custom settings
    config.settings.insert("auto_assign".to_string(), "true".to_string());
    config.settings.insert("max_wip".to_string(), "3".to_string());
    config.settings.insert("require_approval".to_string(), "2".to_string());

    // Then: Settings should be stored
    assert_eq!(config.settings.len(), 3);
    assert_eq!(config.settings.get("auto_assign"), Some(&"true".to_string()));
    assert_eq!(config.settings.get("max_wip"), Some(&"3".to_string()));
}

/// Behavior: Setting project name
#[test]
fn test_set_project_name() {
    // Given: A config without a name
    let mut config = ProjectConfig::default();
    assert!(config.name.is_none());

    // When: I set the project name
    config.name = Some("My Awesome Project".to_string());

    // Then: The name should be set
    assert_eq!(config.name, Some("My Awesome Project".to_string()));
}

/// Behavior: Default reviewers list
#[test]
fn test_default_reviewers() {
    // Given: A config
    let mut config = ProjectConfig::default();
    assert!(config.default_reviewers.is_empty());

    // When: I set default reviewers
    config.default_reviewers = vec![
        "alice".to_string(),
        "bob".to_string(),
        "charlie".to_string(),
    ];

    // Then: Default reviewers should be configured
    assert_eq!(config.default_reviewers.len(), 3);
    assert!(config.default_reviewers.contains(&"alice".to_string()));
}

/// Behavior: Config serialization to TOML
#[test]
fn test_config_serialization_toml() {
    // Given: A fully configured project
    let mut config = ProjectConfig::default();
    config.name = Some("Test Project".to_string());
    config.add_column("Blocked".to_string());
    config.add_tag("backend".to_string());
    config.add_tag("frontend".to_string());
    config.default_reviewers = vec!["alice".to_string(), "bob".to_string()];
    config.settings.insert("require_approval".to_string(), "true".to_string());

    // When: I serialize to TOML
    let toml_string = toml::to_string(&config).expect("Failed to serialize");

    // Then: It should contain all configuration
    assert!(toml_string.contains("Test Project"));
    assert!(toml_string.contains("backend"));
    assert!(toml_string.contains("alice"));

    // When: I deserialize back
    let deserialized: ProjectConfig = toml::from_str(&toml_string).expect("Failed to deserialize");

    // Then: All data should be preserved
    assert_eq!(deserialized.name, config.name);
    assert_eq!(deserialized.columns.len(), config.columns.len());
    assert_eq!(deserialized.tags.len(), config.tags.len());
    assert_eq!(deserialized.default_reviewers.len(), config.default_reviewers.len());
}

/// Behavior: Config with custom column workflow
#[test]
fn test_custom_workflow_columns() {
    // Given: A team with a custom workflow
    let mut config = ProjectConfig {
        name: Some("Custom Workflow Project".to_string()),
        columns: vec![
            "Backlog".to_string(),
            "Prioritized".to_string(),
            "In Development".to_string(),
            "Code Review".to_string(),
            "QA Testing".to_string(),
            "Staging".to_string(),
            "Production".to_string(),
        ],
        tags: vec!["bug".to_string(), "feature".to_string(), "refactor".to_string()],
        default_reviewers: vec![],
        settings: std::collections::HashMap::new(),
    };

    // Then: All custom columns should be valid
    assert!(config.is_valid_column("Backlog"));
    assert!(config.is_valid_column("QA Testing"));
    assert!(config.is_valid_column("Production"));

    // And: Standard columns should not be valid
    assert!(!config.is_valid_column("TODO"));
}

/// Behavior: Empty config edge cases
#[test]
fn test_empty_config_edge_cases() {
    // Given: A minimal config
    let config = ProjectConfig {
        name: None,
        columns: vec![],
        tags: vec![],
        default_reviewers: vec![],
        settings: std::collections::HashMap::new(),
    };

    // Then: It should handle empty state gracefully
    assert!(config.name.is_none());
    assert_eq!(config.columns.len(), 0);
    assert!(!config.is_valid_column("anything"));
}

/// Behavior: Config with extensive settings
#[test]
fn test_extensive_project_settings() {
    // Given: A config with many custom settings
    let mut config = ProjectConfig::default();

    config.settings.insert("auto_assign_reviewers".to_string(), "true".to_string());
    config.settings.insert("min_approvals".to_string(), "2".to_string());
    config.settings.insert("allow_self_review".to_string(), "false".to_string());
    config.settings.insert("wip_limit".to_string(), "5".to_string());
    config.settings.insert("notification_email".to_string(), "team@example.com".to_string());

    // Then: All settings should be retrievable
    assert_eq!(config.settings.len(), 5);
    assert_eq!(
        config.settings.get("min_approvals"),
        Some(&"2".to_string())
    );
    assert_eq!(
        config.settings.get("notification_email"),
        Some(&"team@example.com".to_string())
    );
}
