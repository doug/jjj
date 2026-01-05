use crate::error::Result;
use crate::jj::JjClient;
use crate::models::ReviewStatus;
use crate::storage::MetadataStore;
use crate::utils;

pub fn execute(json: bool) -> Result<()> {
    let jj_client = JjClient::new()?;
    let user_identity = jj_client.user_identity()?;
    let store = MetadataStore::new(jj_client)?;

    if !json {
        println!("Dashboard for {}", user_identity);
        println!();
    }

    // Show tasks assigned to user
    let tasks = store.list_tasks()?;
    let my_tasks: Vec<_> = tasks
        .iter()
        .filter(|t| t.assignee.as_ref().map(|a| format!("@{}", a)).as_deref() == Some(&user_identity))
        .collect();

    if !my_tasks.is_empty() {
        println!("My Tasks ({}):", my_tasks.len());
        for task in &my_tasks {
            println!("  {} - {} [{}]", task.id, task.title, task.column);
        }
        println!();
    }

    // Show reviews requested
    let reviews = store.list_reviews()?;
    let pending_reviews: Vec<_> = reviews
        .iter()
        .filter(|r| {
            r.status == ReviewStatus::Pending
                && r.reviewers.iter().any(|rev| {
                    let reviewer_str = format!("@{}", rev);
                    user_identity.contains(rev) || reviewer_str == user_identity
                })
        })
        .collect();

    if !pending_reviews.is_empty() {
        println!("Reviews Requested ({}):", pending_reviews.len());
        for review in &pending_reviews {
            println!(
                "  {} - {} (Author: {})",
                utils::format_change_id(&review.change_id),
                utils::format_relative_time(&review.requested_at),
                review.author
            );
        }
        println!();
    }

    // Show my reviews
    let my_reviews: Vec<_> = reviews
        .iter()
        .filter(|r| r.author == user_identity)
        .collect();

    if !my_reviews.is_empty() {
        println!("My Reviews ({}):", my_reviews.len());
        for review in &my_reviews {
            let status_icon = match review.status {
                ReviewStatus::Pending => "⏳",
                ReviewStatus::Approved => "✓",
                ReviewStatus::ChangesRequested => "⚠",
                ReviewStatus::Dismissed => "✕",
            };

            println!(
                "  {} {} - {:?}",
                status_icon,
                utils::format_change_id(&review.change_id),
                review.status
            );
        }
        println!();
    }

    if my_tasks.is_empty() && pending_reviews.is_empty() && my_reviews.is_empty() {
        println!("No pending tasks or reviews.");
    }

    if json {
        let dashboard_data = serde_json::json!({
            "my_tasks": my_tasks,
            "pending_reviews": pending_reviews,
            "my_reviews": my_reviews,
        });
        println!("{}", serde_json::to_string_pretty(&dashboard_data)?);
        return Ok(());
    }

    Ok(())
}
