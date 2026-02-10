# Testing Documentation

This document describes the testing strategy and test suite for **jjj**.

## Test Structure

```
tests/
├── config_management.rs       # Configuration tests
├── integration_storage.rs     # Storage layer integration tests
├── integration_test.rs        # End-to-end integration tests
└── workflow_test.rs           # Workflow command tests
```

## Running Tests

### Run All Tests

```bash
cargo test
```

### Run Specific Test Suite

```bash
cargo test --test config_management
cargo test --test integration_test
cargo test --test workflow_test
```

### Run a Specific Test

```bash
cargo test test_default_project_config
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

### Reviewer and Sign-off Tests

**Behavior-Driven Scenarios:**

1. **test_add_reviewer**
   - Given: A solution
   - When: I add reviewers
   - Then: Reviewers are tracked and deduplicated

2. **test_add_sign_off**
   - Given: A solution with reviewers
   - When: A reviewer signs off with a comment
   - Then: The sign-off is recorded with reviewer, timestamp, and comment

3. **test_all_reviewers_signed_off**
   - Given: A solution with assigned reviewers
   - When: All reviewers sign off
   - Then: The acceptance gate passes

4. **test_pending_reviewers**
   - Given: A solution with some sign-offs
   - When: I check pending reviewers
   - Then: Only unsigned reviewers are returned

5. **test_requires_review_derived**
   - Given: A solution
   - When: It has no reviewers / has reviewers
   - Then: `requires_review()` returns false / true accordingly

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

Run `cargo test` to see current test results. Tests cover configuration management, storage operations, workflow commands, and integration scenarios.

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
fn create_test_problem(id: &str) -> Problem {
    Problem::new(
        id.to_string(),
        "Test problem".to_string(),
    )
}
```

### Test Builders

```rust
// Usage example:
use crate::id::generate_id;

let problem_id = generate_id();  // Generates UUID7
let problem = Problem::new(problem_id.clone(), "Test problem".into());
let solution = Solution::new(generate_id(), "Test solution".into(), problem_id);
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
- **Models**: Problem, Solution, Critique, Milestone
- **Storage**: Integration tests cover load/save operations
- **Commands**: Workflow tests cover CLI command execution
- **JJ Integration**: Requires mocking or real jj environment

Target coverage: 80%+ overall

## Running Tests in Demo Environment

See [demo/README.md](demo/README.md) for manual testing procedures.

## Test Resources

- Rust Testing Book: https://doc.rust-lang.org/book/ch11-00-testing.html
- Behavior-Driven Development: https://en.wikipedia.org/wiki/Behavior-driven_development
- Test-Driven Development: https://en.wikipedia.org/wiki/Test-driven_development
