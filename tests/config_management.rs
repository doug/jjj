use jjj::models::ProjectConfig;

/// Behavior: Creating default configuration
#[test]
fn test_default_project_config() {
    // Given: No existing configuration
    // When: I create a default config
    let config = ProjectConfig::default();

    // Then: It should have sensible defaults
    assert!(config.name.is_none());
    assert!(config.default_reviewers.is_empty());
}

/// Behavior: Custom project settings
#[test]
fn test_custom_project_settings() {
    // Given: A config
    let mut config = ProjectConfig::default();

    // When: I add custom settings
    config
        .settings
        .insert("auto_assign".to_string(), "true".to_string());
    config
        .settings
        .insert("max_wip".to_string(), "3".to_string());
    config
        .settings
        .insert("require_approval".to_string(), "2".to_string());

    // Then: Settings should be stored
    assert_eq!(config.settings.len(), 3);
    assert_eq!(
        config.settings.get("auto_assign"),
        Some(&"true".to_string())
    );
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
    config.default_reviewers = vec!["alice".to_string(), "bob".to_string()];
    config
        .settings
        .insert("require_approval".to_string(), "true".to_string());

    // When: I serialize to TOML
    let toml_string = toml::to_string(&config).expect("Failed to serialize");

    // Then: It should contain all configuration
    assert!(toml_string.contains("Test Project"));
    assert!(toml_string.contains("alice"));

    // When: I deserialize back
    let deserialized: ProjectConfig = toml::from_str(&toml_string).expect("Failed to deserialize");

    // Then: All data should be preserved
    assert_eq!(deserialized.name, config.name);
    assert_eq!(
        deserialized.default_reviewers.len(),
        config.default_reviewers.len()
    );
}

/// Behavior: Config with extensive settings
#[test]
fn test_extensive_project_settings() {
    // Given: A config with many custom settings
    let mut config = ProjectConfig::default();

    config
        .settings
        .insert("auto_assign_reviewers".to_string(), "true".to_string());
    config
        .settings
        .insert("min_approvals".to_string(), "2".to_string());
    config
        .settings
        .insert("allow_self_review".to_string(), "false".to_string());
    config
        .settings
        .insert("wip_limit".to_string(), "5".to_string());
    config.settings.insert(
        "notification_email".to_string(),
        "team@example.com".to_string(),
    );

    // Then: All settings should be retrievable
    assert_eq!(config.settings.len(), 5);
    assert_eq!(config.settings.get("min_approvals"), Some(&"2".to_string()));
    assert_eq!(
        config.settings.get("notification_email"),
        Some(&"team@example.com".to_string())
    );
}
