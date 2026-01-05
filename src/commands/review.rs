use crate::cli::ReviewAction;
use crate::error::Result;
use crate::jj::JjClient;
use crate::models::{Comment, CommentLocation, ReviewManifest, ReviewStatus};
use crate::storage::MetadataStore;
use crate::utils;
use chrono::Utc;

pub fn execute(action: ReviewAction) -> Result<()> {
    match action {
        ReviewAction::Request { reviewers, stack } => request_review(reviewers, stack),
        ReviewAction::List { mine, pending } => list_reviews(mine, pending),
        ReviewAction::Start { change_id } => start_review(change_id),
        ReviewAction::Comment { change_id, file, line, body } => {
            add_comment(change_id, file, line, body)
        }
        ReviewAction::Status { change_id } => show_status(change_id),
        ReviewAction::Approve { change_id } => approve(change_id),
        ReviewAction::RequestChanges { change_id, message } => {
            request_changes(change_id, message)
        }
    }
}

fn request_review(reviewers: Vec<String>, _stack: bool) -> Result<()> {
    let jj_client = JjClient::new()?;
    let change_id = jj_client.current_change_id()?;
    let author = jj_client.user_identity()?;
    let store = MetadataStore::new(jj_client)?;

    let reviewers: Vec<String> = reviewers.iter().map(|r| utils::parse_mention(r)).collect();

    let manifest = ReviewManifest {
        change_id: change_id.clone(),
        author,
        reviewers: reviewers.clone(),
        status: ReviewStatus::Pending,
        requested_at: Utc::now(),
        updated_at: Utc::now(),
        comment_count: 0,
        is_stack: false,
    };

    store.save_review(&manifest)?;

    println!("✓ Review requested for change {}", utils::format_change_id(&change_id));
    println!("  Reviewers: {}", reviewers.iter().map(|r| format!("@{}", r)).collect::<Vec<_>>().join(", "));

    Ok(())
}

fn list_reviews(mine: bool, pending: bool) -> Result<()> {
    let jj_client = JjClient::new()?;
    let user_identity = jj_client.user_identity()?;
    let store = MetadataStore::new(jj_client)?;

    let reviews = store.list_reviews()?;

    let filtered_reviews: Vec<_> = reviews
        .iter()
        .filter(|r| {
            if mine && r.author != user_identity {
                return false;
            }
            if pending && !r.reviewers.iter().any(|rev| format!("{} <{}>", rev, rev).contains(&user_identity)) {
                return false;
            }
            true
        })
        .collect();

    if filtered_reviews.is_empty() {
        println!("No reviews found.");
        return Ok(());
    }

    println!("Reviews:");
    println!();

    for review in filtered_reviews {
        let status_str = match review.status {
            ReviewStatus::Pending => "⏳ Pending",
            ReviewStatus::Approved => "✓ Approved",
            ReviewStatus::ChangesRequested => "⚠ Changes Requested",
            ReviewStatus::Dismissed => "✕ Dismissed",
        };

        println!("  {} - {}", utils::format_change_id(&review.change_id), status_str);
        println!("    Author: {}", review.author);
        println!("    Reviewers: {}", review.reviewers.iter().map(|r| format!("@{}", r)).collect::<Vec<_>>().join(", "));

        if review.comment_count > 0 {
            println!("    Comments: {}", review.comment_count);
        }

        println!("    Requested: {}", utils::format_relative_time(&review.requested_at));
        println!();
    }

    Ok(())
}

fn start_review(change_id: String) -> Result<()> {
    let jj_client = JjClient::new()?;
    let store = MetadataStore::new(jj_client.clone())?;

    let review = store.load_review(&change_id)?;

    println!("Review for change {}", change_id);
    println!("Author: {}", review.author);
    println!();

    let diff = jj_client.show_diff(&change_id)?;
    println!("{}", diff);

    println!();
    println!("To add a comment: jjj review comment {} --file <path> --line <line> --body <text>", change_id);
    println!("To approve: jjj review approve {}", change_id);

    Ok(())
}

fn add_comment(
    change_id: String,
    file: Option<String>,
    line: Option<usize>,
    body: String,
) -> Result<()> {
    let jj_client = JjClient::new()?;
    let author = jj_client.user_identity()?;
    let store = MetadataStore::new(jj_client.clone())?;
    let comment_id = store.next_comment_id(&change_id)?;

    let comment = if let (Some(file_path), Some(line_num)) = (file, line) {
        // Read context around the line
        let file_content = jj_client.file_at_revision(&change_id, &file_path)?;
        let lines: Vec<String> = file_content.lines().map(|s| s.to_string()).collect();

        let start = line_num.saturating_sub(2);
        let end = (line_num + 2).min(lines.len());
        let context_lines = lines[start..end].to_vec();

        let location = CommentLocation::new(line_num, line_num, context_lines);

        Comment::new_inline(comment_id.clone(), author, change_id.clone(), file_path, location, body)
    } else {
        Comment::new(comment_id.clone(), author, change_id.clone(), body)
    };

    store.save_comment(&comment)?;

    // Update comment count in review
    let mut review = store.load_review(&change_id)?;
    review.comment_count += 1;
    store.save_review(&review)?;

    println!("✓ Comment added: {}", comment_id);

    Ok(())
}

fn show_status(change_id: Option<String>) -> Result<()> {
    let jj_client = JjClient::new()?;
    let change_id = change_id.unwrap_or_else(|| jj_client.current_change_id().unwrap());
    let store = MetadataStore::new(jj_client)?;
    let review = store.load_review(&change_id)?;

    println!("Review Status for {}", utils::format_change_id(&change_id));
    println!("Status: {:?}", review.status);
    println!("Author: {}", review.author);
    println!("Reviewers: {}", review.reviewers.iter().map(|r| format!("@{}", r)).collect::<Vec<_>>().join(", "));
    println!();

    let comments = store.list_comments(&change_id)?;

    if comments.is_empty() {
        println!("No comments yet.");
    } else {
        println!("Comments ({}):", comments.len());
        for comment in comments {
            println!();
            println!("  [{}] {} - {}", comment.id, comment.author, utils::format_relative_time(&comment.timestamp));

            if let Some(ref file_path) = comment.file_path {
                if let Some(ref location) = comment.location {
                    println!("  {}:{}", file_path, location.start_line);
                }
            }

            println!("  {}", comment.body);

            if comment.resolved {
                println!("  ✓ Resolved");
            }
        }
    }

    Ok(())
}

fn approve(change_id: Option<String>) -> Result<()> {
    let jj_client = JjClient::new()?;
    let change_id = change_id.unwrap_or_else(|| jj_client.current_change_id().unwrap());
    let store = MetadataStore::new(jj_client)?;
    let mut review = store.load_review(&change_id)?;

    review.status = ReviewStatus::Approved;
    review.updated_at = Utc::now();

    store.save_review(&review)?;

    println!("✓ Approved change {}", utils::format_change_id(&change_id));

    Ok(())
}

fn request_changes(change_id: Option<String>, _message: String) -> Result<()> {
    let jj_client = JjClient::new()?;
    let change_id = change_id.unwrap_or_else(|| jj_client.current_change_id().unwrap());
    let store = MetadataStore::new(jj_client)?;
    let mut review = store.load_review(&change_id)?;

    review.status = ReviewStatus::ChangesRequested;
    review.updated_at = Utc::now();

    store.save_review(&review)?;

    println!("✓ Requested changes for {}", utils::format_change_id(&change_id));

    Ok(())
}
