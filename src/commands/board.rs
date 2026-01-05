use crate::error::Result;
use crate::jj::JjClient;
use crate::storage::MetadataStore;
use crate::utils;

pub fn execute() -> Result<()> {
    let jj_client = JjClient::new()?;
    let store = MetadataStore::new(jj_client)?;

    let config = store.load_config()?;
    let tasks = store.list_tasks()?;

    println!("Kanban Board");
    println!();

    // Group tasks by column
    for column in &config.columns {
        let column_tasks: Vec<_> = tasks.iter().filter(|t| &t.column == column).collect();

        println!("┌─ {} ({}) ", column, column_tasks.len());
        println!("│");

        if column_tasks.is_empty() {
            println!("│  (empty)");
        } else {
            for task in column_tasks {
                println!("│  {} - {}", task.id, utils::truncate(&task.title, 50));

                if let Some(ref assignee) = task.assignee {
                    print!("│    @{}", assignee);
                }

                if !task.change_ids.is_empty() {
                    print!("│    Changes: {}", task.change_ids.iter().map(|id| utils::format_change_id(id)).collect::<Vec<_>>().join(", "));
                    println!();
                }

                if !task.tags.is_empty() {
                    println!("│    Tags: {}", task.tags.iter().map(|t| format!("#{}", t)).collect::<Vec<_>>().join(" "));
                }

                if task.comment_count > 0 {
                    println!("│    ⚠ {} comments", task.comment_count);
                }

                println!("│");
            }
        }

        println!("└─");
        println!();
    }

    Ok(())
}
