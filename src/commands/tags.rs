use crate::context::CommandContext;
use crate::error::Result;
use std::collections::HashMap;

pub fn execute(ctx: &CommandContext, json: bool) -> Result<()> {
    let store = &ctx.store;

    let mut counts: HashMap<String, usize> = HashMap::new();

    let problems = store.list_problems()?;
    for problem in &problems {
        for tag in &problem.tags {
            *counts.entry(tag.clone()).or_insert(0) += 1;
        }
    }

    let solutions = store.list_solutions()?;
    for solution in &solutions {
        for tag in &solution.tags {
            *counts.entry(tag.clone()).or_insert(0) += 1;
        }
    }

    if counts.is_empty() {
        if json {
            println!("[]");
        } else {
            println!("No tags in use.");
        }
        return Ok(());
    }

    // Sort by count desc, then alphabetically
    let mut entries: Vec<(String, usize)> = counts.into_iter().collect();
    entries.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

    if json {
        let json_entries: Vec<serde_json::Value> = entries
            .iter()
            .map(|(tag, count)| {
                serde_json::json!({
                    "tag": tag,
                    "count": count,
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&json_entries)?);
    } else {
        println!("{:<30} COUNT", "TAG");
        println!("{}", "-".repeat(40));
        for (tag, count) in &entries {
            println!("{:<30} {}", tag, count);
        }
    }

    Ok(())
}
