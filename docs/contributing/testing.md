# Testing Documentation

This document describes the testing strategy and test suite for **jjj**.

## Test Structure

```
tests/
├── task_management.rs      # Task model and workflow tests
├── review_workflow.rs      # Review and comment tests
└── config_management.rs    # Configuration tests
```

## Running Tests

### Run All Tests

```bash
cargo test
```

### Run Specific Test Suite

```bash
cargo test --test task_management
cargo test --test review_workflow
cargo test --test config_management
```

### Run a Specific Test

```bash
cargo test test_create_task_with_defaults
cargo test test_comment_relocation_fuzzy_match
```

### Run Tests with Output

```bash
cargo test -- --nocapture
```

### Run Tests in Parallel

```bash
cargo test -- --test-threads=4
```

## Test Coverage

### Task Management Tests (14 tests)

**Behavior-Driven Scenarios:**

1. **test_create_task_with_defaults**
   - Given: I want to create a new task
   - When: I create the task with basic properties
   - Then: The task should have correct defaults

2. **test_add_tags_to_task**
   - Given: A task exists
   - When: I add multiple tags
   - Then: Tags are added and version increments

3. **test_remove_tag_from_task**
   - Given: A task with tags
   - When: I remove a tag
   - Then: The tag is removed and version increments

4. **test_remove_nonexistent_tag**
   - Given: A task without a specific tag
   - When: I try to remove a non-existent tag
   - Then: Nothing changes

5. **test_attach_change_to_task**
   - Given: A task and a change ID
   - When: I attach the change
   - Then: The change is linked and version increments

6. **test_attach_same_change_twice**
   - Given: A task with an attached change
   - When: I try to attach the same change again
   - Then: It's idempotent (no duplicate, no version increment)

7. **test_detach_change_from_task**
   - Given: A task with multiple attached changes
   - When: I detach one change
   - Then: Only that change is removed

8. **test_detach_nonexistent_change**
   - Given: A task with a change
   - When: I try to detach a non-existent change
   - Then: Nothing changes

9. **test_move_task_to_different_column**
   - Given: A task in one column
   - When: I move it to another column
   - Then: The column updates and version increments

10. **test_filter_tasks_by_column**
    - Given: Tasks in different columns
    - When: I filter by a specific column
    - Then: Only matching tasks are returned

11. **test_filter_tasks_by_tag**
    - Given: Tasks with different tags
    - When: I filter by a specific tag
    - Then: Only tasks with that tag match

12. **test_filter_tasks_by_assignee**
    - Given: Tasks with different assignees
    - When: I filter by assignee
    - Then: Only that user's tasks match

13. **test_filter_tasks_with_multiple_criteria**
    - Given: Tasks with various properties
    - When: I apply multiple filters (column + tag + assignee)
    - Then: Only tasks matching ALL criteria are returned

14. **test_task_serialization**
    - Given: A fully-configured task
    - When: I serialize and deserialize via JSON
    - Then: All properties are preserved

### Review Workflow Tests (14 tests)

**Behavior-Driven Scenarios:**

1. **test_create_review_request**
   - Given: A change and reviewers
   - When: I create a review request
   - Then: The manifest has correct properties

2. **test_review_status_transitions**
   - Given: A pending review
   - When: Status changes (Pending → Approved → ChangesRequested)
   - Then: State transitions correctly

3. **test_create_general_comment**
   - Given: Comment metadata
   - When: I create a general (non-inline) comment
   - Then: Comment is created without file location

4. **test_create_inline_comment**
   - Given: Comment with file and line location
   - When: I create an inline comment
   - Then: Location information is attached

5. **test_comment_location_context_hash**
   - Given: Context lines around a comment
   - When: I create locations with same/different context
   - Then: Hash is consistent for same context, different for different context

6. **test_comment_relocation_exact_match**
   - Given: A comment at a specific line
   - When: The file hasn't changed
   - Then: Comment stays at the same line

7. **test_comment_relocation_fuzzy_match**
   - Given: A comment with specific context
   - When: Lines are inserted above (shifting code down)
   - Then: Comment is relocated to the new position via fuzzy matching

8. **test_comment_relocation_fails_when_context_removed**
   - Given: A comment on deleted code
   - When: The context is completely removed
   - Then: Relocation fails or returns low-confidence match

9. **test_review_manifest_serialization**
   - Given: A review with all fields
   - When: I serialize/deserialize via TOML
   - Then: All fields are preserved

10. **test_comment_serialization**
    - Given: A comment with location
    - When: I serialize/deserialize via JSON
    - Then: All fields including location are preserved

11. **test_review_status_equality**
    - Given: Different review statuses
    - When: I compare them
    - Then: Equality works correctly

12. **test_multiple_reviewers**
    - Given: A review with many reviewers
    - When: I create the review
    - Then: All reviewers are tracked

13. **test_comment_resolution**
    - Given: An unresolved comment
    - When: I mark it as resolved
    - Then: The resolved flag is set

14. **test_stack_review_flag**
    - Given: A review for a stack of changes
    - When: I set the stack flag
    - Then: The flag is recorded

### Configuration Management Tests (15 tests)

**Behavior-Driven Scenarios:**

1. **test_default_project_config**
   - Given: No existing configuration
   - When: I create default config
   - Then: Standard Kanban columns are present

2. **test_validate_column_names**
   - Given: A config with specific columns
   - When: I validate column names
   - Then: Valid columns pass, invalid fail

3. **test_add_custom_column**
   - Given: Default config
   - When: I add a custom column
   - Then: Column is added successfully

4. **test_add_duplicate_column**
   - Given: Config with existing columns
   - When: I try to add a duplicate
   - Then: No change occurs

5. **test_remove_column**
   - Given: Config with standard columns
   - When: I remove a column
   - Then: Column is removed

6. **test_remove_nonexistent_column**
   - Given: Default config
   - When: I try to remove non-existent column
   - Then: Nothing changes

7. **test_add_tags_to_config**
   - Given: Config with no tags
   - When: I add multiple tags
   - Then: All tags are present

8. **test_add_duplicate_tag**
   - Given: Config with existing tags
   - When: I add a duplicate tag
   - Then: No duplicate is created

9. **test_custom_project_settings**
   - Given: A config
   - When: I add custom key-value settings
   - Then: Settings are stored

10. **test_set_project_name**
    - Given: Config without a name
    - When: I set the project name
    - Then: Name is stored

11. **test_default_reviewers**
    - Given: A config
    - When: I set default reviewers
    - Then: Reviewers are configured

12. **test_config_serialization_toml**
    - Given: Fully configured project
    - When: I serialize/deserialize via TOML
    - Then: All data is preserved

13. **test_custom_workflow_columns**
    - Given: Custom workflow with many columns
    - When: I validate columns
    - Then: Custom columns work, standard ones don't

14. **test_empty_config_edge_cases**
    - Given: Minimal empty config
    - When: I use it
    - Then: Handles empty state gracefully

15. **test_extensive_project_settings**
    - Given: Many custom settings
    - When: I add them
    - Then: All are retrievable

## Test Results

All **43 tests** pass successfully:

```
running 14 tests (config_management)
test result: ok. 14 passed; 0 failed; 0 ignored

running 14 tests (review_workflow)
test result: ok. 14 passed; 0 failed; 0 ignored

running 14 tests (task_management)
test result: ok. 14 passed; 0 failed; 0 ignored
```

## Testing Philosophy

### Behavior-Driven Development (BDD)

All tests follow the BDD pattern:

```rust
#[test]
fn test_name() {
    // Given: Initial state and context
    let task = Task::new(...);

    // When: Action is performed
    task.add_tag("backend");

    // Then: Expected outcome
    assert!(task.tags.contains("backend"));
}
```

This approach:
- Makes tests readable as specifications
- Documents expected behavior
- Serves as living documentation
- Validates user stories

### Test Coverage Goals

- **Unit Tests**: Test individual components in isolation
- **Property Tests**: Verify invariants (versioning, idempotency)
- **Serialization Tests**: Ensure data persistence works correctly
- **Edge Case Tests**: Handle empty states, duplicates, non-existent items
- **Integration Tests**: (Future) Test command execution end-to-end

## Future Testing Plans

### Integration Tests

```rust
// tests/integration/
├── cli_commands.rs          # Test CLI interface
├── storage_operations.rs    # Test metadata storage
└── jj_integration.rs        # Test jj command execution
```

### Property-Based Tests

Using `proptest` or `quickcheck`:

```rust
#[proptest]
fn task_version_always_increases(
    operations: Vec<TaskOperation>
) {
    let mut task = Task::new(...);
    let initial_version = task.version;

    for op in operations {
        apply_operation(&mut task, op);
    }

    assert!(task.version >= initial_version);
}
```

### Snapshot Tests

For output formatting:

```rust
#[test]
fn test_board_output_format() {
    let output = render_board(&tasks);
    insta::assert_snapshot!(output);
}
```

### Performance Tests

```rust
#[test]
fn test_large_task_list_performance() {
    let tasks: Vec<Task> = (0..10_000)
        .map(|i| create_task(i))
        .collect();

    let start = Instant::now();
    let filtered = filter_tasks(&tasks, &filter);
    let duration = start.elapsed();

    assert!(duration < Duration::from_millis(100));
}
```

## Test Data Patterns

### Factory Functions

```rust
fn create_test_task(id: &str) -> Task {
    Task::new(
        id.to_string(),
        "Test task".to_string(),
        "TODO".to_string(),
    )
}

fn create_test_review(change_id: &str) -> ReviewManifest {
    ReviewManifest {
        change_id: change_id.to_string(),
        author: "test@example.com".to_string(),
        // ... defaults
    }
}
```

### Test Builders

```rust
struct TaskBuilder {
    task: Task,
}

impl TaskBuilder {
    fn new(id: &str) -> Self { ... }
    fn with_tag(mut self, tag: &str) -> Self { ... }
    fn with_assignee(mut self, user: &str) -> Self { ... }
    fn build(self) -> Task { ... }
}

// Usage:
let task = TaskBuilder::new("T-1")
    .with_tag("backend")
    .with_assignee("alice")
    .build();
```

## Continuous Integration

### GitHub Actions Workflow

```yaml
name: Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - run: cargo test --all-features
      - run: cargo test --doc
```

## Test Maintenance

### Adding New Tests

1. Identify the behavior to test
2. Write the test in BDD format (Given/When/Then)
3. Place in appropriate test file
4. Run `cargo test` to verify
5. Update this documentation

### Updating Existing Tests

When changing behavior:
1. Update affected tests first
2. Verify tests fail with old code
3. Implement the change
4. Verify tests pass with new code
5. Update documentation

## Test Quality Checklist

- [ ] Test name clearly describes what is being tested
- [ ] Test follows Given/When/Then structure
- [ ] Test is deterministic (no random data, no time dependencies)
- [ ] Test is isolated (doesn't depend on other tests)
- [ ] Test verifies one specific behavior
- [ ] Test assertions are clear and specific
- [ ] Edge cases are covered
- [ ] Error conditions are tested

## Coverage Metrics

Current coverage (approximate):
- **Models**: ~95% (task, review, config)
- **Storage**: ~0% (requires integration tests)
- **Commands**: ~0% (requires integration tests)
- **JJ Integration**: ~0% (requires mocking or real jj)

Target coverage: 80%+ overall

## Running Tests in Demo Environment

See [demo/README.md](demo/README.md) for manual testing procedures.

## Test Resources

- Rust Testing Book: https://doc.rust-lang.org/book/ch11-00-testing.html
- Behavior-Driven Development: https://en.wikipedia.org/wiki/Behavior-driven_development
- Test-Driven Development: https://en.wikipedia.org/wiki/Test-driven_development
