use crate::cli::TaskAction;
use crate::error::Result;
use crate::jj::JjClient;
use crate::models::{Task, TaskFilter};
use crate::storage::MetadataStore;
use crate::utils;

pub fn execute(action: TaskAction) -> Result<()> {
    let jj_client = JjClient::new()?;
    let store = MetadataStore::new(jj_client)?;

    match action {
        TaskAction::New { title, feature, tag, column } => create_task(&store, title, feature, tag, column),
        TaskAction::List { column, tag, json } => list_tasks(&store, column, tag, json),
        TaskAction::Show { task_id } => show_task(&store, task_id),
        TaskAction::Attach { task_id } => attach_task(&store, task_id),
        TaskAction::Detach { task_id, change_id } => detach_task(&store, task_id, change_id),
        TaskAction::Move { task_id, column } => move_task(&store, task_id, column),
        TaskAction::Edit { task_id, title, add_tag, remove_tag } => {
            edit_task(&store, task_id, title, add_tag, remove_tag)
        }
        TaskAction::Delete { task_id } => delete_task(&store, task_id),
        TaskAction::Assign { task_id, to } => assign_task(&store, task_id, to),
    }
}

fn create_task(
    store: &MetadataStore,
    title: String,
    feature_id: String,
    tags: Vec<String>,
    column: Option<String>,
) -> Result<()> {
    store.with_metadata(&format!("Create task {}", title), || {
        let mut config = store.load_config()?;

        // Validate feature
        if store.load_feature(&feature_id).is_err() {
            return Err(format!("Feature {} not found", feature_id).into());
        }

        // Validate column
        let column = column.unwrap_or_else(|| config.columns[0].clone());
        if !config.is_valid_column(&column) {
            return Err(format!("Invalid column: {}", column).into());
        }

        let task_id = store.next_task_id()?;
        let mut task = Task::new(task_id.clone(), title, feature_id.clone(), column);

        // Resolve and add tags
        for tag_input in tags {
            let tag_id = crate::utils::resolve_tag(&mut config, &tag_input);
            task.add_tag(tag_id);
        }

        // Save config (in case new tags were created)
        store.save_config(&config)?;
        store.save_task(&task)?;

        // Update feature
        let mut feature = store.load_feature(&feature_id)?;
        feature.add_task(task_id.clone());
        store.save_feature(&feature)?;

        println!("Created task {} ({})", task.id, task.title);
        Ok(())
    })
}

fn list_tasks(
    store: &MetadataStore,
    column_filter: Option<String>,
    tag_filter: Option<String>,
    json: bool,
) -> Result<()> {
    let tasks = store.list_tasks()?;
    let mut config = store.load_config()?; // Load config to resolve tag names

    // Resolve tag filter if present
    let resolved_tag_id = if let Some(tag_input) = tag_filter {
        // We don't want to create new tags during list, so just try to find ID or name
        if let Some(tag) = config.get_tag(&tag_input) {
            Some(tag.id.clone())
        } else if let Some(tag) = config.get_tag_by_name(&tag_input) {
            Some(tag.id.clone())
        } else {
            // If tag not found, filter will match nothing
            Some("non-existent-tag".to_string())
        }
    } else {
        None
    };

    let filter = TaskFilter {
        column: column_filter,
        tag_id: resolved_tag_id,
        assignee: None,
    };

    let filtered_tasks: Vec<&Task> = tasks.iter().filter(|t| filter.matches(t)).collect();

    if json {
        println!("{}", serde_json::to_string_pretty(&filtered_tasks)?);
        return Ok(());
    }

    if filtered_tasks.is_empty() {
        println!("No tasks found.");
        return Ok(());
    }

    // Sort by ID
    let mut sorted_tasks = filtered_tasks.into_iter().collect::<Vec<_>>();
    sorted_tasks.sort_by(|a, b| a.id.cmp(&b.id));

    println!("{:<10} {:<30} {:<15} {:<20}", "ID", "Title", "Column", "Tags");
    println!("{:-<10} {:-<30} {:-<15} {:-<20}", "", "", "", "");

    for task in sorted_tasks {
        let tag_names: Vec<String> = task
            .tag_ids
            .iter()
            .map(|id| {
                config
                    .get_tag(id)
                    .map(|t| t.name.clone())
                    .unwrap_or_else(|| id.clone())
            })
            .collect();
        
        println!(
            "{:<10} {:<30} {:<15} {:<20}",
            task.id,
            crate::utils::truncate(&task.title, 28),
            crate::utils::truncate(&task.column, 13),
            tag_names.join(", ")
        );
    }

    Ok(())
}

fn show_task(store: &MetadataStore, task_id: String) -> Result<()> {
    let task = store.load_task(&task_id)?;
    let config = store.load_config()?;

    println!("Task: {}", task.id);
    println!("Title: {}", task.title);
    println!("Feature: {}", task.feature_id);
    println!("Column: {}", task.column);

    if let Some(ref assignee) = task.assignee {
        println!("Assignee: @{}", assignee);
    }

    if !task.tag_ids.is_empty() {
        let tag_names: Vec<String> = task
            .tag_ids
            .iter()
            .map(|id| {
                config
                    .get_tag(id)
                    .map(|t| t.name.clone())
                    .unwrap_or_else(|| id.clone())
            })
            .collect();
        println!("Tags: {}", tag_names.join(", "));
    }

    if let Some(ref desc) = task.description {
        println!("Description: {}", desc);
    }

    if !task.change_ids.is_empty() {
        println!("Attached changes:");
        for change_id in &task.change_ids {
            println!("  - {}", change_id);
        }
    }

    println!("Created: {}", utils::format_relative_time(&task.created_at));
    println!("Updated: {}", utils::format_relative_time(&task.updated_at));

    Ok(())
}

fn attach_task(store: &MetadataStore, task_id: String) -> Result<()> {
    let change_id = store.jj_client.current_change_id()?;

    let mut task = store.load_task(&task_id)?;
    task.attach_change(change_id.clone());
    store.save_task(&task)?;

    println!("✓ Attached change {} to task {}", utils::format_change_id(&change_id), task_id);

    Ok(())
}

fn detach_task(store: &MetadataStore, task_id: String, change_id: Option<String>) -> Result<()> {
    let change_id = change_id.unwrap_or_else(|| store.jj_client.current_change_id().unwrap());

    let mut task = store.load_task(&task_id)?;

    if task.detach_change(&change_id) {
        store.save_task(&task)?;
        println!("✓ Detached change {} from task {}", utils::format_change_id(&change_id), task_id);
    } else {
        println!("Change {} is not attached to task {}", utils::format_change_id(&change_id), task_id);
    }

    Ok(())
}

fn move_task(store: &MetadataStore, task_id: String, column: String) -> Result<()> {
    let config = store.load_config()?;
    if !config.is_valid_column(&column) {
        return Err(format!("Invalid column: {}. Valid columns: {:?}", column, config.columns).into());
    }

    let mut task = store.load_task(&task_id)?;
    let old_column = task.column.clone();

    task.move_to_column(column.clone());
    store.save_task(&task)?;

    println!("✓ Moved task {} from '{}' to '{}'", task_id, old_column, column);

    Ok(())
}

fn edit_task(
    store: &MetadataStore,
    task_id: String,
    title: Option<String>,
    add_tags: Vec<String>,
    remove_tags: Vec<String>,
) -> Result<()> {
    store.with_metadata(&format!("Edit task {}", task_id), || {
        let mut task = store.load_task(&task_id)?;
        let mut config = store.load_config()?;

        if let Some(t) = title {
            task.title = t;
        }

        for tag_input in add_tags {
            let tag_id = crate::utils::resolve_tag(&mut config, &tag_input);
            task.add_tag(tag_id);
        }

        for tag_input in remove_tags {
            // For removal, we try to resolve ID, but if it's a name, we find the ID
            let tag_id = if let Some(tag) = config.get_tag(&tag_input) {
                Some(tag.id.clone())
            } else if let Some(tag) = config.get_tag_by_name(&tag_input) {
                Some(tag.id.clone())
            } else {
                // If we can't resolve it, assume it might be the ID directly (legacy cleanup?)
                // or just try to remove it as is
                Some(tag_input)
            };
            
            if let Some(id) = tag_id {
                task.remove_tag(&id);
            }
        }

        store.save_config(&config)?;
        store.save_task(&task)?;
        println!("Updated task {}", task_id);
        Ok(())
    })
}

fn delete_task(store: &MetadataStore, task_id: String) -> Result<()> {
    let task = store.load_task(&task_id)?;

    println!("Are you sure you want to delete task '{}'?", task.title);
    if utils::confirm("Delete")? {
        store.delete_task(&task_id)?;
        println!("✓ Deleted task {}", task_id);
    } else {
        println!("Cancelled");
    }

    Ok(())
}

fn assign_task(store: &MetadataStore, task_id: String, assignee: Option<String>) -> Result<()> {
    // Determine assignee
    let assignee_name = if let Some(name) = assignee {
        name
    } else {
        // Default to current user
        store.jj_client.user_name()?
    };
    
    let assignee_clone = assignee_name.clone();

    store.with_metadata(&format!("Assign task {} to {}", task_id, assignee_name), || {
        let mut task = store.load_task(&task_id)?;
        task.assignee = Some(assignee_name.clone());
        task.updated_at = chrono::Utc::now();
        task.version += 1;
        store.save_task(&task)?;
        Ok(())
    })?;

    println!("Assigned task {} to {}", task_id, assignee_clone);
    Ok(())
}
