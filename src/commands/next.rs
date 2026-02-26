use crate::context::CommandContext;
use crate::error::Result;

/// Print the single highest-priority next action and its suggested command.
///
/// The action list is the same one `jjj status` uses, but `next` is designed
/// for scripting and shell prompts where you just want one thing to do.
pub fn execute(ctx: &CommandContext, json: bool) -> Result<()> {
    let store = &ctx.store;

    let problems = store.list_problems()?;
    let solutions = store.list_solutions()?;
    let critiques = store.list_critiques()?;

    let user = store.jj_client.user_identity().unwrap_or_default();

    let items = crate::commands::status::build_next_actions(
        &problems,
        &solutions,
        &critiques,
        &user,
        false,
    );

    if items.is_empty() {
        if json {
            println!("null");
        } else {
            println!("Nothing to do — all caught up!");
        }
        return Ok(());
    }

    let item = &items[0];

    if json {
        println!("{}", serde_json::to_string_pretty(item)?);
        return Ok(());
    }

    let category = item["category"].as_str().unwrap_or("").to_uppercase();
    let title = item["title"].as_str().unwrap_or("");
    let summary = item["summary"].as_str().unwrap_or("");
    let cmd = item["suggested_command"].as_str().unwrap_or("");

    println!("[{}] {} — {}", category, title, summary);
    if !cmd.is_empty() {
        println!("  -> {}", cmd);
    }

    Ok(())
}
