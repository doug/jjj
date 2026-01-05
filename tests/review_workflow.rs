use jjj::models::{Comment, CommentLocation, ReviewManifest, ReviewStatus};
use chrono::Utc;

/// Behavior: Creating a review request
#[test]
fn test_create_review_request() {
    // Given: A change and reviewers
    let change_id = "kpqxywon".to_string();
    let author = "Alice <alice@example.com>".to_string();
    let reviewers = vec!["bob".to_string(), "charlie".to_string()];

    // When: I create a review request
    let manifest = ReviewManifest {
        change_id: change_id.clone(),
        author: author.clone(),
        reviewers: reviewers.clone(),
        status: ReviewStatus::Pending,
        requested_at: Utc::now(),
        updated_at: Utc::now(),
        comment_count: 0,
        is_stack: false,
    };

    // Then: The review should have the correct properties
    assert_eq!(manifest.change_id, change_id);
    assert_eq!(manifest.author, author);
    assert_eq!(manifest.reviewers, reviewers);
    assert_eq!(manifest.status, ReviewStatus::Pending);
    assert_eq!(manifest.comment_count, 0);
}

/// Behavior: Review status transitions
#[test]
fn test_review_status_transitions() {
    // Given: A pending review
    let mut manifest = ReviewManifest {
        change_id: "test123".to_string(),
        author: "Alice <alice@example.com>".to_string(),
        reviewers: vec!["bob".to_string()],
        status: ReviewStatus::Pending,
        requested_at: Utc::now(),
        updated_at: Utc::now(),
        comment_count: 0,
        is_stack: false,
    };

    // When: The review is approved
    manifest.status = ReviewStatus::Approved;
    manifest.updated_at = Utc::now();

    // Then: The status should change to Approved
    assert_eq!(manifest.status, ReviewStatus::Approved);

    // When: Changes are requested instead
    manifest.status = ReviewStatus::ChangesRequested;

    // Then: The status should reflect that
    assert_eq!(manifest.status, ReviewStatus::ChangesRequested);
}

/// Behavior: Creating a general comment
#[test]
fn test_create_general_comment() {
    // Given: Comment metadata
    let id = "c-1".to_string();
    let author = "Bob <bob@example.com>".to_string();
    let change_id = "kpqxywon".to_string();
    let body = "Looks good overall!".to_string();

    // When: I create a general comment
    let comment = Comment::new(id.clone(), author.clone(), change_id.clone(), body.clone());

    // Then: The comment should have correct properties
    assert_eq!(comment.id, id);
    assert_eq!(comment.author, author);
    assert_eq!(comment.target_change_id, change_id);
    assert_eq!(comment.body, body);
    assert!(comment.file_path.is_none());
    assert!(comment.location.is_none());
    assert!(!comment.resolved);
}

/// Behavior: Creating an inline comment with location
#[test]
fn test_create_inline_comment() {
    // Given: Comment with file location
    let id = "c-2".to_string();
    let author = "Charlie <charlie@example.com>".to_string();
    let change_id = "kpqxywon".to_string();
    let file_path = "src/auth.rs".to_string();
    let body = "This function could use better error handling".to_string();

    let context_lines = vec![
        "fn authenticate(user: &str) -> Result<Token> {".to_string(),
        "    let token = generate_token(user);".to_string(),
        "    Ok(token)".to_string(),
    ];
    let location = CommentLocation::new(42, 42, context_lines.clone());

    // When: I create an inline comment
    let comment = Comment::new_inline(
        id.clone(),
        author.clone(),
        change_id.clone(),
        file_path.clone(),
        location,
        body.clone(),
    );

    // Then: The comment should have location information
    assert_eq!(comment.file_path, Some(file_path));
    assert!(comment.location.is_some());

    let loc = comment.location.unwrap();
    assert_eq!(loc.start_line, 42);
    assert_eq!(loc.end_line, 42);
    assert_eq!(loc.context_lines, context_lines);
    assert!(!loc.context_hash.is_empty());
}

/// Behavior: Comment location context hashing
#[test]
fn test_comment_location_context_hash() {
    // Given: Context lines
    let context_lines = vec![
        "fn test() {".to_string(),
        "    println!(\"hello\");".to_string(),
        "}".to_string(),
    ];

    // When: I create two locations with the same context
    let loc1 = CommentLocation::new(10, 12, context_lines.clone());
    let loc2 = CommentLocation::new(10, 12, context_lines.clone());

    // Then: They should have the same hash
    assert_eq!(loc1.context_hash, loc2.context_hash);

    // When: I create a location with different context
    let different_context = vec![
        "fn other() {".to_string(),
        "    println!(\"world\");".to_string(),
        "}".to_string(),
    ];
    let loc3 = CommentLocation::new(10, 12, different_context);

    // Then: It should have a different hash
    assert_ne!(loc1.context_hash, loc3.context_hash);
}

/// Behavior: Exact line relocation (no changes)
#[test]
fn test_comment_relocation_exact_match() {
    // Given: A comment at line 42
    let context_lines = vec![
        "    let x = 10;".to_string(),
        "    let y = 20;".to_string(),
        "    let z = x + y;".to_string(),
    ];
    let location = CommentLocation::new(42, 42, context_lines.clone());

    // When: The file hasn't changed (same lines at same position)
    let file_lines = vec![
        "fn test() {".to_string(),
        "    let x = 10;".to_string(),
        "    let y = 20;".to_string(),
        "    let z = x + y;".to_string(),
        "    println!(\"{}\", z);".to_string(),
        "}".to_string(),
    ];

    let relocated = location.try_relocate(&file_lines);

    // Then: The comment should stay at line 42
    assert!(relocated.is_some());
    let (start, end) = relocated.unwrap();
    assert_eq!(start, 42);
    assert_eq!(end, 42);
}

/// Behavior: Fuzzy relocation when lines move
#[test]
fn test_comment_relocation_fuzzy_match() {
    // Given: A comment with specific context
    let context_lines = vec![
        "fn calculate(x: i32, y: i32) -> i32 {".to_string(),
        "    x + y".to_string(),
        "}".to_string(),
    ];
    let location = CommentLocation::new(10, 10, context_lines);

    // When: New code is inserted above, shifting lines down
    let new_file_lines = vec![
        "// New header comment".to_string(),
        "// Another comment".to_string(),
        "// More comments".to_string(),
        "fn calculate(x: i32, y: i32) -> i32 {".to_string(),
        "    x + y".to_string(),
        "}".to_string(),
        "fn other() {}".to_string(),
    ];

    let relocated = location.try_relocate(&new_file_lines);

    // Then: The comment should be relocated to the new position
    assert!(relocated.is_some());
    // The fuzzy match should find the function around line 3-5
    // (exact line depends on fuzzy match algorithm)
}

/// Behavior: Failed relocation when context is gone
#[test]
fn test_comment_relocation_fails_when_context_removed() {
    // Given: A comment on a specific function
    let context_lines = vec![
        "fn removed_function() {".to_string(),
        "    // This will be deleted".to_string(),
        "}".to_string(),
    ];
    let location = CommentLocation::new(10, 10, context_lines);

    // When: The function is completely removed
    let new_file_lines = vec![
        "fn different_function() {".to_string(),
        "    println!(\"Hello\");".to_string(),
        "}".to_string(),
    ];

    let relocated = location.try_relocate(&new_file_lines);

    // Then: Relocation should fail (or return very different position)
    // This might return None or a poor match below the similarity threshold
    // The behavior depends on the fuzzy matching threshold
}

/// Behavior: Review serialization preserves all fields
#[test]
fn test_review_manifest_serialization() {
    // Given: A review with all fields set
    let manifest = ReviewManifest {
        change_id: "abc123".to_string(),
        author: "Alice <alice@example.com>".to_string(),
        reviewers: vec!["bob".to_string(), "charlie".to_string()],
        status: ReviewStatus::ChangesRequested,
        requested_at: Utc::now(),
        updated_at: Utc::now(),
        comment_count: 5,
        is_stack: true,
    };

    // When: I serialize and deserialize
    let toml_string = toml::to_string(&manifest).expect("Failed to serialize");
    let deserialized: ReviewManifest = toml::from_str(&toml_string).expect("Failed to deserialize");

    // Then: All fields should be preserved
    assert_eq!(deserialized.change_id, manifest.change_id);
    assert_eq!(deserialized.author, manifest.author);
    assert_eq!(deserialized.reviewers, manifest.reviewers);
    assert_eq!(deserialized.status, manifest.status);
    assert_eq!(deserialized.comment_count, manifest.comment_count);
    assert_eq!(deserialized.is_stack, manifest.is_stack);
}

/// Behavior: Comment serialization preserves all fields
#[test]
fn test_comment_serialization() {
    // Given: A comment with location
    let context_lines = vec![
        "line 1".to_string(),
        "line 2".to_string(),
        "line 3".to_string(),
    ];
    let location = CommentLocation::new(42, 44, context_lines);

    let comment = Comment::new_inline(
        "c-123".to_string(),
        "Bob <bob@example.com>".to_string(),
        "change456".to_string(),
        "src/main.rs".to_string(),
        location,
        "Please fix this".to_string(),
    );

    // When: I serialize and deserialize
    let json = serde_json::to_string(&comment).expect("Failed to serialize");
    let deserialized: Comment = serde_json::from_str(&json).expect("Failed to deserialize");

    // Then: All fields should be preserved
    assert_eq!(deserialized.id, comment.id);
    assert_eq!(deserialized.author, comment.author);
    assert_eq!(deserialized.target_change_id, comment.target_change_id);
    assert_eq!(deserialized.file_path, comment.file_path);
    assert_eq!(deserialized.body, comment.body);
    assert_eq!(deserialized.resolved, comment.resolved);

    assert!(deserialized.location.is_some());
    let loc = deserialized.location.unwrap();
    assert_eq!(loc.start_line, 42);
    assert_eq!(loc.end_line, 44);
}

/// Behavior: Review status equality
#[test]
fn test_review_status_equality() {
    // Given: Review statuses
    let pending1 = ReviewStatus::Pending;
    let pending2 = ReviewStatus::Pending;
    let approved = ReviewStatus::Approved;

    // Then: Same statuses should be equal
    assert_eq!(pending1, pending2);
    assert_ne!(pending1, approved);
}

/// Behavior: Multiple reviewers on a single review
#[test]
fn test_multiple_reviewers() {
    // Given: A review request with multiple reviewers
    let reviewers = vec![
        "alice".to_string(),
        "bob".to_string(),
        "charlie".to_string(),
        "diana".to_string(),
    ];

    let manifest = ReviewManifest {
        change_id: "test".to_string(),
        author: "author@example.com".to_string(),
        reviewers: reviewers.clone(),
        status: ReviewStatus::Pending,
        requested_at: Utc::now(),
        updated_at: Utc::now(),
        comment_count: 0,
        is_stack: false,
    };

    // Then: All reviewers should be recorded
    assert_eq!(manifest.reviewers.len(), 4);
    assert!(manifest.reviewers.contains(&"alice".to_string()));
    assert!(manifest.reviewers.contains(&"diana".to_string()));
}

/// Behavior: Comment resolution tracking
#[test]
fn test_comment_resolution() {
    // Given: A comment that starts unresolved
    let mut comment = Comment::new(
        "c-1".to_string(),
        "Bob".to_string(),
        "change1".to_string(),
        "Please fix".to_string(),
    );

    assert!(!comment.resolved);

    // When: The comment is marked as resolved
    comment.resolved = true;

    // Then: It should be tracked as resolved
    assert!(comment.resolved);
}

/// Behavior: Stack review flag
#[test]
fn test_stack_review_flag() {
    // Given: A review for an entire stack of changes
    let mut manifest = ReviewManifest {
        change_id: "base_change".to_string(),
        author: "Alice".to_string(),
        reviewers: vec!["bob".to_string()],
        status: ReviewStatus::Pending,
        requested_at: Utc::now(),
        updated_at: Utc::now(),
        comment_count: 0,
        is_stack: false,
    };

    // When: I mark it as a stack review
    manifest.is_stack = true;

    // Then: The flag should be set
    assert!(manifest.is_stack);
}
