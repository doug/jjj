use crate::context::CommandContext;
use crate::db::{self, search, Database};
use crate::error::Result;

pub fn execute(
    ctx: &CommandContext,
    query: &str,
    entity_type: Option<&str>,
    json: bool,
) -> Result<()> {
    let jj_client = ctx.jj();
    let db_path = jj_client.repo_root().join(".jj").join("jjj.db");

    // Ensure DB exists
    let db = if db_path.exists() {
        Database::open(&db_path)?
    } else {
        // Create and populate from markdown
        let db = Database::open(&db_path)?;
        db::load_from_markdown(&db, &ctx.store)?;
        db
    };

    let results = search::search(db.conn(), query, entity_type)?;

    if json {
        let json_results: Vec<_> = results.iter().map(|r| {
            serde_json::json!({
                "type": r.entity_type,
                "id": r.entity_id,
                "title": r.title,
                "snippet": r.snippet,
            })
        }).collect();
        println!("{}", serde_json::to_string_pretty(&json_results)?);
    } else {
        if results.is_empty() {
            println!("No results found for \"{}\"", query);
        } else {
            println!("Found {} result(s) for \"{}\":\n", results.len(), query);
            for result in results {
                println!("[{}] {} - {}", result.entity_type, result.entity_id, result.title);
                if !result.snippet.is_empty() {
                    println!("    {}", result.snippet.replace('\n', " "));
                }
                println!();
            }
        }
    }

    Ok(())
}
