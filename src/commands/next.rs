use crate::context::CommandContext;
use crate::error::Result;

/// Print the highest-priority next action(s) and their suggested commands.
///
/// `top` controls how many items to show (default 1; 0 means all).
/// `mine` restricts to work authored by the current user.
/// The action list is the same one `jjj status` uses.
pub fn execute(ctx: &CommandContext, top: Option<usize>, mine: bool, json: bool) -> Result<()> {
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
        mine,
    );

    if items.is_empty() {
        if json {
            println!("null");
        } else {
            println!("Nothing to do — all caught up!");
        }
        return Ok(());
    }

    // Determine how many items to show: top=None → 1, top=Some(0) → all, top=Some(n) → n
    let count = match top {
        None => 1,
        Some(0) => items.len(),
        Some(n) => n.min(items.len()),
    };

    let to_show = &items[..count];

    if json {
        if count == 1 {
            println!("{}", serde_json::to_string_pretty(&to_show[0])?);
        } else {
            println!("{}", serde_json::to_string_pretty(to_show)?);
        }
        return Ok(());
    }

    for (i, item) in to_show.iter().enumerate() {
        let category = item["category"].as_str().unwrap_or("").to_uppercase();
        let title = item["title"].as_str().unwrap_or("");
        let summary = item["summary"].as_str().unwrap_or("");
        let cmd = item["suggested_command"].as_str().unwrap_or("");

        if count > 1 {
            println!("{}. [{}] {} — {}", i + 1, category, title, summary);
        } else {
            println!("[{}] {} — {}", category, title, summary);
        }
        if !cmd.is_empty() {
            println!("  -> {}", cmd);
        }
    }

    Ok(())
}
