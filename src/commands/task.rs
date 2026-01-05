use crate::cli::TaskAction;
use crate::error::Result;
use crate::jj::JjClient;
use crate::models::{Task, TaskFilter};
use crate::storage::MetadataStore;
use crate::utils;

pub fn execute(action: TaskAction) -> Result<()> {
    match action {
        TaskAction::New { title, feature, tag, column } => create_task(title, feature, tag, column),
        TaskAction::List { column, tag } => list_tasks(column, tag),
        TaskAction::Show { task_id } => show_task(task_id),
        TaskAction::Attach { task_id } => attach_task(task_id),
        TaskAction::Detach { task_id, change_id } => detach_task(task_id, change_id),
        TaskAction::Move { task_id, column } => move_task(task_id, column),
        TaskAction::Edit { task_id, title, add_tag, remove_tag } => {
            edit_task(task_id, title, add_tag, remove_tag)
        }
        TaskAction::Delete { task_id } => delete_task(task_id),
    }
}

fn create_task(title: String, feature_id: String, tags: Vec<String>, column: Option<String>) -> Result<()> {
    let jj_client = JjClient::new()?;
    let store = MetadataStore::new(jj_client)?;

    let config = store.load_config()?;
    let column = column.unwrap_or_else(|| config.columns[0].clone());

    if !config.is_valid_column(&column) {
        return Err(format!("Invalid column: {}. Valid columns: {:?}", column, config.columns).into());
    }

    let task_id = store.next_task_id()?;
    let mut task = Task::new(task_id.clone(), title, feature_id.clone(), column);

    for tag in tags {
        task.add_tag(tag);
    }

    store.save_task(&task)?;

    println!("✓ Created task {}", task_id);
    println!("  Title: {}", task.title);
    println!("  Feature: {}", feature_id);
    println!("  Column: {}", task.column);
    if !task.tags.is_empty() {
        println!("  Tags: {}", task.tags.iter().map(|s| format!("#{}", s)).collect::<Vec<_>>().join(", "));
    }

    Ok(())
}

fn list_tasks(column: Option<String>, tag: Option<String>) -> Result<()> {
    let jj_client = JjClient::new()?;
    let store = MetadataStore::new(jj_client)?;

    let filter = TaskFilter {
        column,
        tag,
        assignee: None,
    };

    let tasks = store.list_tasks()?;
    let filtered_tasks: Vec<_> = tasks.iter().filter(|t| filter.matches(t)).collect();

    if filtered_tasks.is_empty() {
        println!("No tasks found.");
        return Ok(());
    }

    println!("Tasks:");
    println!();

    for task in filtered_tasks {
        print!("  {} - {}", task.id, task.title);

        if let Some(ref assignee) = task.assignee {
            print!(" (@{})", assignee);
        }

        println!();
        println!("    Column: {}", task.column);

        if !task.tags.is_empty() {
            println!("    Tags: {}", task.tags.iter().map(|s| format!("#{}", s)).collect::<Vec<_>>().join(", "));
        }

        if !task.change_ids.is_empty() {
            println!("    Changes: {}", task.change_ids.iter().map(|id| utils::format_change_id(id)).collect::<Vec<_>>().join(", "));
        }

        println!();
    }

    Ok(())
}

fn show_task(task_id: String) -> Result<()> {
    let jj_client = JjClient::new()?;
    let store = MetadataStore::new(jj_client)?;

    let task = store.load_task(&task_id)?;

    println!("Task: {}", task.id);
    println!("Title: {}", task.title);
    println!("Feature: {}", task.feature_id);
    println!("Column: {}", task.column);

    if let Some(ref assignee) = task.assignee {
        println!("Assignee: @{}", assignee);
    }

    if !task.tags.is_empty() {
        println!("Tags: {}", task.tags.iter().map(|s| format!("#{}", s)).collect::<Vec<_>>().join(", "));
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

fn attach_task(task_id: String) -> Result<()> {
    let jj_client = JjClient::new()?;
    let store = MetadataStore::new(jj_client.clone())?;

    let mut task = store.load_task(&task_id)?;
    let change_id = jj_client.current_change_id()?;

    task.attach_change(change_id.clone());
    store.save_task(&task)?;

    println!("✓ Attached change {} to task {}", utils::format_change_id(&change_id), task_id);

    Ok(())
}

fn detach_task(task_id: String, change_id: Option<String>) -> Result<()> {
    let jj_client = JjClient::new()?;
    let change_id = change_id.unwrap_or_else(|| jj_client.current_change_id().unwrap());
    let store = MetadataStore::new(jj_client)?;

    let mut task = store.load_task(&task_id)?;

    if task.detach_change(&change_id) {
        store.save_task(&task)?;
        println!("✓ Detached change {} from task {}", utils::format_change_id(&change_id), task_id);
    } else {
        println!("Change {} is not attached to task {}", utils::format_change_id(&change_id), task_id);
    }

    Ok(())
}

fn move_task(task_id: String, column: String) -> Result<()> {
    let jj_client = JjClient::new()?;
    let store = MetadataStore::new(jj_client)?;

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
    task_id: String,
    title: Option<String>,
    add_tags: Vec<String>,
    remove_tags: Vec<String>,
) -> Result<()> {
    let jj_client = JjClient::new()?;
    let store = MetadataStore::new(jj_client)?;

    let mut task = store.load_task(&task_id)?;

    if let Some(new_title) = title {
        task.title = new_title;
    }

    for tag in add_tags {
        task.add_tag(tag);
    }

    for tag in remove_tags {
        task.remove_tag(&tag);
    }

    store.save_task(&task)?;

    println!("✓ Updated task {}", task_id);

    Ok(())
}

fn delete_task(task_id: String) -> Result<()> {
    let jj_client = JjClient::new()?;
    let store = MetadataStore::new(jj_client)?;

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
