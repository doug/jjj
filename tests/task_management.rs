use jjj::models::{Task, TaskFilter};

/// Behavior: Creating a new task
#[test]
fn test_create_task_with_defaults() {
    // Given: I want to create a new task
    let task_id = "T-1".to_string();
    let title = "Implement user authentication".to_string();
    let column = "TODO".to_string();

    // When: I create the task
    let task = Task::new(task_id.clone(), title.clone(), "F-TEST".to_string(), column.clone());

    // Then: The task should have the correct properties
    assert_eq!(task.id, task_id);
    assert_eq!(task.title, title);
    assert_eq!(task.column, column);
    assert!(task.tags.is_empty());
    assert!(task.assignee.is_none());
    assert!(task.change_ids.is_empty());
    assert_eq!(task.comment_count, 0);
    assert_eq!(task.version, 1);
}

/// Behavior: Adding tags to a task
#[test]
fn test_add_tags_to_task() {
    // Given: A task exists
    let mut task = Task::new("T-1".to_string(), "Test task".to_string(), "F-TEST".to_string(), "TODO".to_string());
    let initial_version = task.version;

    // When: I add tags to the task
    task.add_tag("backend".to_string());
    task.add_tag("database".to_string());

    // Then: The task should have the tags and version should increment
    assert_eq!(task.tags.len(), 2);
    assert!(task.tags.contains("backend"));
    assert!(task.tags.contains("database"));
    assert_eq!(task.version, initial_version + 2);
}

/// Behavior: Removing tags from a task
#[test]
fn test_remove_tag_from_task() {
    // Given: A task with tags
    let mut task = Task::new("T-1".to_string(), "Test task".to_string(), "F-TEST".to_string(), "TODO".to_string());
    task.add_tag("backend".to_string());
    task.add_tag("frontend".to_string());
    let version_before_removal = task.version;

    // When: I remove a tag
    let removed = task.remove_tag("backend");

    // Then: The tag should be removed and version incremented
    assert!(removed);
    assert!(!task.tags.contains("backend"));
    assert!(task.tags.contains("frontend"));
    assert_eq!(task.version, version_before_removal + 1);
}

/// Behavior: Removing a non-existent tag
#[test]
fn test_remove_nonexistent_tag() {
    // Given: A task without a specific tag
    let mut task = Task::new("T-1".to_string(), "Test task".to_string(), "F-TEST".to_string(), "TODO".to_string());
    let initial_version = task.version;

    // When: I try to remove a tag that doesn't exist
    let removed = task.remove_tag("nonexistent");

    // Then: Nothing should change
    assert!(!removed);
    assert_eq!(task.version, initial_version);
}

/// Behavior: Attaching a change to a task
#[test]
fn test_attach_change_to_task() {
    // Given: A task and a change ID
    let mut task = Task::new("T-1".to_string(), "Implement feature".to_string(), "F-TEST".to_string(), "TODO".to_string());
    let change_id = "kpqxywon".to_string();
    let initial_version = task.version;

    // When: I attach the change to the task
    task.attach_change(change_id.clone());

    // Then: The change should be attached and version incremented
    assert_eq!(task.change_ids.len(), 1);
    assert_eq!(task.change_ids[0], change_id);
    assert_eq!(task.version, initial_version + 1);
}

/// Behavior: Attaching the same change twice
#[test]
fn test_attach_same_change_twice() {
    // Given: A task with an attached change
    let mut task = Task::new("T-1".to_string(), "Test task".to_string(), "F-TEST".to_string(), "TODO".to_string());
    let change_id = "kpqxywon".to_string();
    task.attach_change(change_id.clone());
    let version_after_first_attach = task.version;

    // When: I try to attach the same change again
    task.attach_change(change_id.clone());

    // Then: The change should only appear once and version should not increment
    assert_eq!(task.change_ids.len(), 1);
    assert_eq!(task.version, version_after_first_attach);
}

/// Behavior: Detaching a change from a task
#[test]
fn test_detach_change_from_task() {
    // Given: A task with attached changes
    let mut task = Task::new("T-1".to_string(), "Test task".to_string(), "F-TEST".to_string(), "TODO".to_string());
    task.attach_change("change1".to_string());
    task.attach_change("change2".to_string());
    let version_before_detach = task.version;

    // When: I detach one change
    let detached = task.detach_change("change1");

    // Then: The change should be removed and version incremented
    assert!(detached);
    assert_eq!(task.change_ids.len(), 1);
    assert_eq!(task.change_ids[0], "change2");
    assert_eq!(task.version, version_before_detach + 1);
}

/// Behavior: Detaching a non-existent change
#[test]
fn test_detach_nonexistent_change() {
    // Given: A task with a change
    let mut task = Task::new("T-1".to_string(), "Test task".to_string(), "F-TEST".to_string(), "TODO".to_string());
    task.attach_change("change1".to_string());
    let initial_version = task.version;

    // When: I try to detach a change that isn't attached
    let detached = task.detach_change("nonexistent");

    // Then: Nothing should change
    assert!(!detached);
    assert_eq!(task.change_ids.len(), 1);
    assert_eq!(task.version, initial_version);
}

/// Behavior: Moving a task to a different column
#[test]
fn test_move_task_to_different_column() {
    // Given: A task in TODO
    let mut task = Task::new("T-1".to_string(), "Test task".to_string(), "F-TEST".to_string(), "TODO".to_string());
    let initial_version = task.version;

    // When: I move it to "In Progress"
    task.move_to_column("In Progress".to_string());

    // Then: The column should change and version increment
    assert_eq!(task.column, "In Progress");
    assert_eq!(task.version, initial_version + 1);
}

/// Behavior: Filtering tasks by column
#[test]
fn test_filter_tasks_by_column() {
    // Given: Multiple tasks in different columns
    let task1 = Task::new("T-1".to_string(), "Task 1".to_string(), "F-TEST".to_string(), "TODO".to_string());
    let task2 = Task::new("T-2".to_string(), "Task 2".to_string(), "F-TEST".to_string(), "In Progress".to_string());
    let task3 = Task::new("T-3".to_string(), "Task 3".to_string(), "F-TEST".to_string(), "TODO".to_string());

    // When: I filter by "TODO" column
    let filter = TaskFilter {
        column: Some("TODO".to_string()),
        tag: None,
        assignee: None,
    };

    // Then: Only TODO tasks should match
    assert!(filter.matches(&task1));
    assert!(!filter.matches(&task2));
    assert!(filter.matches(&task3));
}

/// Behavior: Filtering tasks by tag
#[test]
fn test_filter_tasks_by_tag() {
    // Given: Tasks with different tags
    let mut task1 = Task::new("T-1".to_string(), "Task 1".to_string(), "F-TEST".to_string(), "TODO".to_string());
    task1.add_tag("backend".to_string());

    let mut task2 = Task::new("T-2".to_string(), "Task 2".to_string(), "F-TEST".to_string(), "TODO".to_string());
    task2.add_tag("frontend".to_string());

    let mut task3 = Task::new("T-3".to_string(), "Task 3".to_string(), "F-TEST".to_string(), "TODO".to_string());
    task3.add_tag("backend".to_string());

    // When: I filter by "backend" tag
    let filter = TaskFilter {
        column: None,
        tag: Some("backend".to_string()),
        assignee: None,
    };

    // Then: Only backend tasks should match
    assert!(filter.matches(&task1));
    assert!(!filter.matches(&task2));
    assert!(filter.matches(&task3));
}

/// Behavior: Filtering tasks by assignee
#[test]
fn test_filter_tasks_by_assignee() {
    // Given: Tasks with different assignees
    let mut task1 = Task::new("T-1".to_string(), "Task 1".to_string(), "F-TEST".to_string(), "TODO".to_string());
    task1.assignee = Some("alice".to_string());

    let mut task2 = Task::new("T-2".to_string(), "Task 2".to_string(), "F-TEST".to_string(), "TODO".to_string());
    task2.assignee = Some("bob".to_string());

    let task3 = Task::new("T-3".to_string(), "Task 3".to_string(), "F-TEST".to_string(), "TODO".to_string());
    // task3 has no assignee

    // When: I filter by "alice"
    let filter = TaskFilter {
        column: None,
        tag: None,
        assignee: Some("alice".to_string()),
    };

    // Then: Only alice's tasks should match
    assert!(filter.matches(&task1));
    assert!(!filter.matches(&task2));
    assert!(!filter.matches(&task3));
}

/// Behavior: Complex filtering (multiple criteria)
#[test]
fn test_filter_tasks_with_multiple_criteria() {
    // Given: Tasks with various properties
    let mut task1 = Task::new("T-1".to_string(), "Task 1".to_string(), "F-TEST".to_string(), "TODO".to_string());
    task1.add_tag("backend".to_string());
    task1.assignee = Some("alice".to_string());

    let mut task2 = Task::new("T-2".to_string(), "Task 2".to_string(), "F-TEST".to_string(), "TODO".to_string());
    task2.add_tag("backend".to_string());
    task2.assignee = Some("bob".to_string());

    let mut task3 = Task::new("T-3".to_string(), "Task 3".to_string(), "F-TEST".to_string(), "In Progress".to_string());
    task3.add_tag("backend".to_string());
    task3.assignee = Some("alice".to_string());

    // When: I filter by TODO column, backend tag, and alice
    let filter = TaskFilter {
        column: Some("TODO".to_string()),
        tag: Some("backend".to_string()),
        assignee: Some("alice".to_string()),
    };

    // Then: Only task1 should match all criteria
    assert!(filter.matches(&task1));
    assert!(!filter.matches(&task2)); // Wrong assignee
    assert!(!filter.matches(&task3)); // Wrong column
}

/// Behavior: Serialization and deserialization
#[test]
fn test_task_serialization() {
    // Given: A task with various properties
    let mut task = Task::new("T-42".to_string(), "Implement feature X".to_string(), "F-TEST".to_string(), "In Progress".to_string());
    task.add_tag("backend".to_string());
    task.add_tag("api".to_string());
    task.assignee = Some("alice".to_string());
    task.attach_change("kpqxywon".to_string());
    task.description = Some("Detailed description here".to_string());

    // When: I serialize and deserialize the task
    let json = serde_json::to_string(&task).expect("Failed to serialize");
    let deserialized: Task = serde_json::from_str(&json).expect("Failed to deserialize");

    // Then: All properties should be preserved
    assert_eq!(deserialized.id, task.id);
    assert_eq!(deserialized.title, task.title);
    assert_eq!(deserialized.column, task.column);
    assert_eq!(deserialized.tags, task.tags);
    assert_eq!(deserialized.assignee, task.assignee);
    assert_eq!(deserialized.change_ids, task.change_ids);
    assert_eq!(deserialized.description, task.description);
    assert_eq!(deserialized.version, task.version);
}
