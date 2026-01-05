use crate::cli::FeatureAction;
use crate::error::Result;
use crate::jj::JjClient;
use crate::models::{Feature, FeatureStatus, Priority};
use crate::storage::MetadataStore;
use chrono::Utc;

pub fn execute(action: FeatureAction) -> Result<()> {
    let jj_client = JjClient::new()?;
    let store = MetadataStore::new(jj_client)?;

    match action {
        FeatureAction::New {
            title,
            milestone,
            priority,
            description,
        } => create_feature(&store, title, milestone, priority, description),
        FeatureAction::List {
            milestone,
            status,
            json,
        } => list_features(&store, milestone, status, json),
        FeatureAction::Show { feature_id, json } => show_feature(&store, feature_id, json),
        FeatureAction::Board { feature_id, json } => show_board(&store, feature_id, json),
        FeatureAction::Progress { feature_id } => show_progress(&store, feature_id),
        FeatureAction::Move { feature_id, status } => move_feature(&store, feature_id, status),
    }
}

fn create_feature(
    store: &MetadataStore,
    title: String,
    milestone: Option<String>,
    priority: Option<String>,
    description: Option<String>,
) -> Result<()> {
    let feature_id = store.next_feature_id()?;

    // Parse priority
    let priority = if let Some(p) = priority {
        parse_priority(&p)?
    } else {
        Priority::Medium
    };

    let feature = Feature {
        id: feature_id.clone(),
        title: title.clone(),
        description,
        milestone_id: milestone.clone(),
        status: FeatureStatus::Backlog,
        assignee: None,
        task_ids: Vec::new(),
        bug_ids: Vec::new(),
        tags: std::collections::HashSet::new(),
        priority,
        story_points: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    store.with_metadata(&format!("Create feature {}", feature_id), || {
        store.save_feature(&feature)?;

        // If milestone specified, add feature to milestone
        if let Some(milestone_id) = &milestone {
            let mut milestone = store.load_milestone(milestone_id)?;
            if !milestone.feature_ids.contains(&feature_id) {
                milestone.feature_ids.push(feature_id.clone());
                milestone.updated_at = Utc::now();
                store.save_milestone(&milestone)?;
            }
        }

        Ok(())
    })?;

    let milestone_str = milestone
        .map(|m| format!(" in milestone {}", m))
        .unwrap_or_default();
    println!("Created feature {} ({}){}", feature_id, title, milestone_str);
    Ok(())
}

fn list_features(
    store: &MetadataStore,
    milestone_filter: Option<String>,
    status_filter: Option<String>,
    json: bool,
) -> Result<()> {
    let mut features = store.list_features()?;

    // Apply filters
    if let Some(milestone) = &milestone_filter {
        features.retain(|f| f.milestone_id.as_ref() == Some(milestone));
    }

    if let Some(status) = &status_filter {
        let status = parse_feature_status(status)?;
        features.retain(|f| f.status == status);
    }

    if json {
        println!("{}", serde_json::to_string_pretty(&features)?);
        return Ok(());
    }

    if features.is_empty() {
        println!("No features found.");
        return Ok(());
    }

    println!("Features:");
    for feature in features {
        let milestone_str = feature
            .milestone_id
            .map(|m| format!(" ({})", m))
            .unwrap_or_default();
        let priority_str = format!("{:?}", feature.priority);
        let status_str = format!("{:?}", feature.status);

        println!(
            "  {} - {} [{}] [{}]{} ({} tasks, {} bugs)",
            feature.id,
            feature.title,
            priority_str,
            status_str,
            milestone_str,
            feature.task_ids.len(),
            feature.bug_ids.len()
        );
    }

    Ok(())
}

fn show_feature(store: &MetadataStore, feature_id: String, json: bool) -> Result<()> {
    let feature = store.load_feature(&feature_id)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&feature)?);
        return Ok(());
    }

    println!("Feature: {} - {}", feature.id, feature.title);
    println!("Status: {:?}", feature.status);
    println!("Priority: {:?}", feature.priority);

    if let Some(desc) = &feature.description {
        println!("Description: {}", desc);
    }

    if let Some(milestone) = &feature.milestone_id {
        println!("Milestone: {}", milestone);
    }

    if let Some(points) = feature.story_points {
        println!("Story Points: {}", points);
    }

    println!("\nTasks ({}):", feature.task_ids.len());
    if feature.task_ids.is_empty() {
        println!("  (none)");
    } else {
        for task_id in &feature.task_ids {
            // Try to load task to show more details
            if let Ok(task) = store.load_task(task_id) {
                println!("  - {} [{}] {}", task.id, task.column, task.title);
            } else {
                println!("  - {}", task_id);
            }
        }
    }

    println!("\nBugs ({}):", feature.bug_ids.len());
    if feature.bug_ids.is_empty() {
        println!("  (none)");
    } else {
        for bug_id in &feature.bug_ids {
            println!("  - {}", bug_id);
        }
    }

    println!("\nCreated: {}", feature.created_at.format("%Y-%m-%d %H:%M"));
    println!("Updated: {}", feature.updated_at.format("%Y-%m-%d %H:%M"));

    Ok(())
}

fn show_board(
    store: &MetadataStore,
    feature_id: Option<String>,
    json: bool,
) -> Result<()> {
    if let Some(fid) = feature_id {
        // Show tasks for specific feature
        let feature = store.load_feature(&fid)?;
        let config = store.load_config()?;
        let all_tasks = store.list_tasks()?;

        // Filter tasks belonging to this feature
        let tasks: Vec<_> = all_tasks
            .into_iter()
            .filter(|t| t.feature_id == fid)
            .collect();

        if json {
            let mut board_data = serde_json::Map::new();
            board_data.insert("feature".to_string(), serde_json::to_value(&feature)?);

            for column in &config.columns {
                let column_tasks: Vec<_> = tasks.iter().filter(|t| &t.column == column).collect();
                board_data.insert(column.clone(), serde_json::to_value(column_tasks)?);
            }
            println!("{}", serde_json::to_string_pretty(&board_data)?);
            return Ok(());
        }

        println!("Feature Board: {} - {}\n", feature.id, feature.title);
        for column in &config.columns {
            let column_tasks: Vec<_> = tasks.iter().filter(|t| &t.column == column).collect();
            println!("{} ({}):", column, column_tasks.len());
            for task in column_tasks {
                println!("  - {} {}", task.id, task.title);
            }
            println!();
        }
    } else {
        // Show all features with task counts
        let features = store.list_features()?;
        let all_tasks = store.list_tasks()?;
        let config = store.load_config()?;

        if json {
            let board_data: Vec<_> = features
                .iter()
                .map(|f| {
                    let feature_tasks: Vec<_> = all_tasks
                        .iter()
                        .filter(|t| t.feature_id == f.id)
                        .collect();

                    let mut feature_data = serde_json::Map::new();
                    feature_data.insert("feature".to_string(), serde_json::to_value(f).unwrap());

                    for column in &config.columns {
                        let column_tasks: Vec<_> = feature_tasks
                            .iter()
                            .filter(|t| &t.column == column)
                            .collect();
                        feature_data.insert(
                            column.clone(),
                            serde_json::to_value(column_tasks).unwrap(),
                        );
                    }

                    feature_data
                })
                .collect();

            println!("{}", serde_json::to_string_pretty(&board_data)?);
            return Ok(());
        }

        println!("Feature Board:\n");
        for feature in features {
            let feature_tasks: Vec<_> = all_tasks
                .iter()
                .filter(|t| t.feature_id == feature.id)
                .collect();

            let total_tasks = feature_tasks.len();
            let done_tasks = feature_tasks
                .iter()
                .filter(|t| t.column == "Done")
                .count();

            let progress = if total_tasks > 0 {
                (done_tasks as f32 / total_tasks as f32 * 100.0) as u32
            } else {
                0
            };

            println!(
                "{} - {} [{:?}] - {} / {} tasks ({}%)",
                feature.id, feature.title, feature.status, done_tasks, total_tasks, progress
            );
        }
    }

    Ok(())
}

fn show_progress(store: &MetadataStore, feature_id: String) -> Result<()> {
    let feature = store.load_feature(&feature_id)?;
    let all_tasks = store.list_tasks()?;
    let config = store.load_config()?;

    // Get tasks for this feature
    let tasks: Vec<_> = all_tasks
        .into_iter()
        .filter(|t| t.feature_id == feature_id)
        .collect();

    println!("Progress for {} - {}\n", feature.id, feature.title);
    println!("Status: {:?}", feature.status);
    println!("Priority: {:?}", feature.priority);

    if let Some(points) = feature.story_points {
        println!("Story Points: {}", points);
    }

    println!("\nTask Breakdown:");
    for column in &config.columns {
        let count = tasks.iter().filter(|t| &t.column == column).count();
        let percentage = if !tasks.is_empty() {
            (count as f32 / tasks.len() as f32 * 100.0) as u32
        } else {
            0
        };
        println!("  {}: {} ({}%)", column, count, percentage);
    }

    let total = tasks.len();
    let done = tasks.iter().filter(|t| t.column == "Done").count();
    let overall_progress = if total > 0 {
        (done as f32 / total as f32 * 100.0) as u32
    } else {
        0
    };

    println!("\nOverall: {} / {} tasks complete ({}%)", done, total, overall_progress);

    if let Some(milestone) = &feature.milestone_id {
        println!("Milestone: {}", milestone);
    }

    Ok(())
}

fn move_feature(store: &MetadataStore, feature_id: String, status: String) -> Result<()> {
    let new_status = parse_feature_status(&status)?;
    let new_status_clone = new_status.clone();

    store.with_metadata(&format!("Move feature {} to {:?}", feature_id, new_status), || {
        let mut feature = store.load_feature(&feature_id)?;
        feature.status = new_status;
        feature.updated_at = Utc::now();
        store.save_feature(&feature)?;
        Ok(())
    })?;

    println!("Moved feature {} to {:?}", feature_id, new_status_clone);
    Ok(())
}

// Helper functions

fn parse_priority(s: &str) -> Result<Priority> {
    match s.to_lowercase().as_str() {
        "low" => Ok(Priority::Low),
        "medium" => Ok(Priority::Medium),
        "high" => Ok(Priority::High),
        "critical" => Ok(Priority::Critical),
        _ => Err(format!("Invalid priority: {}. Use low, medium, high, or critical", s).into()),
    }
}

fn parse_feature_status(s: &str) -> Result<FeatureStatus> {
    match s.to_lowercase().as_str() {
        "backlog" => Ok(FeatureStatus::Backlog),
        "inprogress" | "in-progress" | "in_progress" => Ok(FeatureStatus::InProgress),
        "review" => Ok(FeatureStatus::Review),
        "done" => Ok(FeatureStatus::Done),
        "blocked" => Ok(FeatureStatus::Blocked),
        _ => Err(format!(
            "Invalid status: {}. Use backlog, inprogress, review, done, or blocked",
            s
        )
        .into()),
    }
}
