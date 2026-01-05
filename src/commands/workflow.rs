use crate::error::Result;
use crate::jj::JjClient;
use crate::models::{ReviewStatus, Task};
use crate::storage::MetadataStore;

pub fn start(arg: String, feature_id: Option<String>) -> Result<()> {
    let jj_client = JjClient::new()?;
    let store = MetadataStore::new(jj_client.clone())?;
    let config = store.load_config()?;

    // Check if arg is a Task ID
    if let Ok(task) = store.load_task(&arg) {
        println!("Resuming task {} ({})", task.id, task.title);

        // Check if task has an active change attached
        if let Some(change_id) = task.change_ids.last() {
            println!("Switching to change {}", change_id);
            jj_client.edit(change_id)?;
        } else {
            println!("No active change for task. Creating new change.");
            jj_client.new_empty_change(&task.title)?;
            
            // Attach new change to task
            let change_id = jj_client.current_change_id()?;
            let mut task = task;
            task.attach_change(change_id);
            task.move_to_column("In Progress".to_string());
            store.save_task(&task)?;
        }

        return Ok(());
    }

    // Treat arg as Title for new Task
    let title = arg;
    println!("Starting new task: {}", title);

    store.with_metadata(&format!("Start task {}", title), || {
        // Create new empty change
        jj_client.new_empty_change(&title)?;
        let change_id = jj_client.current_change_id()?;

        // Create Task
        let task_id = store.next_task_id()?;
        // Handle feature linking
        let feature = if let Some(fid) = feature_id {
            // Validate feature exists
            if store.load_feature(&fid).is_err() {
                 return Err(format!("Feature {} not found", fid).into());
            }
            fid
        } else {
             // Fallback: use empty string if no feature provided
             String::new()
        };

        // Determine column (In Progress)
        let default_column = "In Progress".to_string();
        let column = if !config.is_valid_column(&default_column) {
             // Fallback if "In Progress" doesn't exist, use second column or first
             if config.columns.len() > 1 {
                 config.columns[1].clone()
             } else {
                 config.columns[0].clone()
             }
        } else {
            default_column
        };

        let mut task = Task::new(task_id.clone(), title.clone(), feature.clone(), column);
        task.attach_change(change_id);
        
        store.save_task(&task)?;

        // If feature provided, link it
        if !feature.is_empty() {
            let mut feat = store.load_feature(&feature)?;
            feat.add_task(task_id.clone());
            store.save_feature(&feat)?;
        }

        println!("Started task {} ({})", task.id, task.title);
        Ok(())
    })
}

pub fn submit(force: bool) -> Result<()> {
    let jj_client = JjClient::new()?;
    let store = MetadataStore::new(jj_client.clone())?;

    let change_id = jj_client.current_change_id()?;
    
    // 1. Review Check
    if !force {
        match store.load_review(&change_id) {
            Ok(review) => {
                if review.status != ReviewStatus::Approved {
                    return Err(format!("Change {} is not approved. Current status: {:?}. Use --force to bypass.", change_id, review.status).into());
                }
            },
            Err(_) => {
                return Err(format!("No review found for change {}. Use --force to bypass.", change_id).into());
            }
        }
    }

    // 2. Rebase onto main
    println!("Rebasing onto main...");
    jj_client.execute(&["rebase", "-d", "main"])?;

    // 3. Squash
    println!("Squashing...");
    jj_client.squash()?;

    // 4. Update Task Status
    let tasks = store.list_tasks()?;
    let mut tasks_to_update = Vec::new();
    
    for task in tasks {
        if task.change_ids.contains(&change_id) {
            tasks_to_update.push(task);
        }
    }

    for mut task in tasks_to_update {
        task.move_to_column("Done".to_string());
        store.save_task(&task)?;
        println!("Task {} moved to Done.", task.id);
    }

    println!("Submitted successfully.");
    Ok(())
}
