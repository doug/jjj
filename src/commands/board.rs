use crate::error::Result;
use crate::jj::JjClient;
use crate::storage::MetadataStore;
use crate::tui;
use crate::utils;

pub fn execute(json: bool) -> Result<()> {
    let jj_client = JjClient::new()?;
    let store = MetadataStore::new(jj_client)?;

    let config = store.load_config()?;
    let tasks = store.list_tasks()?;

    if json {
        let mut board_data = serde_json::Map::new();
        for column in &config.columns {
            let column_tasks: Vec<_> = tasks.iter().filter(|t| &t.column == column).collect();
            board_data.insert(column.clone(), serde_json::to_value(column_tasks)?);
        }
        println!("{}", serde_json::to_string_pretty(&board_data)?);
        return Ok(());
    }

    tui::launch_board(tasks, config.columns)?;

    Ok(())
}
