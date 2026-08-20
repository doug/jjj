use crate::context::CommandContext;
use crate::error::Result;

/// Print the highest-priority next action(s) and their suggested commands.
///
/// `top` controls how many items to show (default 1; 0 means all).
/// `mine` restricts to work authored by the current user.
/// `claim` assigns the top item to the current user before displaying it.
/// The action list is the same one `jjj status` uses.
/// Report when a claim is taken from another actor.
///
/// Only a lapsed lease can be taken, so this is never a silent theft — but it
/// still means someone's in-flight work was handed elsewhere, and that should be
/// visible rather than inferred from an assignee changing.
fn note_reclaim(
    _store: &crate::storage::MetadataStore,
    entity_id: &str,
    previous: Option<&str>,
    actor: &str,
) {
    if let Some(previous) = previous {
        if !crate::identity::actor_matches(previous, actor) {
            crate::output::warn(&format!(
                "reclaimed {} from {} — their claim had lapsed",
                crate::display::short_id(entity_id),
                previous
            ));
        }
    }
}

pub fn execute(
    ctx: &CommandContext,
    top: Option<usize>,
    mine: bool,
    json: bool,
    claim: bool,
) -> Result<()> {
    let store = &ctx.store;

    let problems = store.list_problems()?;
    let solutions = store.list_solutions()?;
    let critiques = store.list_critiques()?;

    let user = store.get_current_user().unwrap_or_default();
    let ttl = crate::claim::claim_ttl(&store.load_config().unwrap_or_default().settings);

    let items = crate::commands::status::build_next_actions_with_ttl(
        &problems, &solutions, &critiques, &user, mine, ttl,
    );

    if items.is_empty() {
        if json {
            println!("null");
        } else if claim {
            println!("Nothing to claim.");
        } else {
            println!("Nothing to do — all caught up!");
        }
        return Ok(());
    }

    // --claim: assign the top item to the current user, then display it
    if claim {
        let item = &items[0];
        let entity_type = item["entity_type"].as_str().unwrap_or("");
        let entity_id = item["entity_id"].as_str().unwrap_or("");

        match entity_type {
            "problem" => {
                store.with_metadata(
                    &format!("Claim problem {} for {}", entity_id, user),
                    || {
                        let mut problem = store.load_problem(entity_id)?;
                        let previous = problem.assignee.clone();
                        // Always restamp, even when re-claiming our own item:
                        // the lease refreshes at the agent's sync boundaries
                        // rather than through a separate heartbeat.
                        problem.assignee = Some(user.clone());
                        problem.claimed_at = Some(chrono::Utc::now());
                        store.save_problem(&problem)?;
                        note_reclaim(store, entity_id, previous.as_deref(), &user);
                        Ok(())
                    },
                )?;
            }
            "solution" => {
                store.with_metadata(
                    &format!("Claim solution {} for {}", entity_id, user),
                    || {
                        let mut solution = store.load_solution(entity_id)?;
                        let previous = solution.assignee.clone();
                        solution.assignee = Some(user.clone());
                        solution.claimed_at = Some(chrono::Utc::now());
                        store.save_solution(&solution)?;
                        note_reclaim(store, entity_id, previous.as_deref(), &user);
                        Ok(())
                    },
                )?;
            }
            // Critiques already have a reviewer — skip assignment
            _ => {}
        }

        if json {
            println!("{}", serde_json::to_string_pretty(&item)?);
        } else {
            let category = item["category"].as_str().unwrap_or("").to_uppercase();
            let title = item["title"].as_str().unwrap_or("");
            let summary = item["summary"].as_str().unwrap_or("");
            let cmd = item["suggested_command"].as_str().unwrap_or("");

            println!("Claimed: [{}] {} — {}", category, title, summary);
            if !cmd.is_empty() {
                println!("  -> {}", cmd);
            }
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
