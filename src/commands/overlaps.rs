use crate::context::CommandContext;
use crate::error::Result;
use crate::models::Solution;
use std::collections::HashMap;
use std::path::PathBuf;

/// A file touched by multiple active solutions.
pub(crate) struct Overlap {
    pub file: PathBuf,
    pub solutions: Vec<(String, String)>, // (id, title)
}

/// Find files touched by more than one active solution.
///
/// For each active solution with attached change IDs, asks `jj diff --summary`
/// which files it modifies, then reports any file appearing in 2+ solutions.
pub(crate) fn find_overlaps(ctx: &CommandContext) -> Result<Vec<Overlap>> {
    let store = &ctx.store;
    let jj_client = ctx.jj();

    let solutions = store.list_solutions()?;
    let active: Vec<&Solution> = solutions.iter().filter(|s| s.is_active()).collect();

    let mut file_map: HashMap<PathBuf, Vec<(String, String)>> = HashMap::new();

    for solution in &active {
        for change_id in &solution.change_ids {
            match jj_client.changed_files(change_id) {
                Ok(files) => {
                    for file in files {
                        file_map
                            .entry(file)
                            .or_default()
                            .push((solution.id.clone(), solution.title.clone()));
                    }
                }
                Err(_) => {
                    // Change may no longer exist (rebased away, etc.) — skip silently
                    continue;
                }
            }
        }
    }

    // Deduplicate: a solution may touch the same file via multiple change IDs
    for entries in file_map.values_mut() {
        entries.sort_by(|a, b| a.0.cmp(&b.0));
        entries.dedup_by(|a, b| a.0 == b.0);
    }

    // Keep only files with 2+ distinct solutions
    let mut overlaps: Vec<Overlap> = file_map
        .into_iter()
        .filter(|(_, sols)| sols.len() >= 2)
        .map(|(file, solutions)| Overlap { file, solutions })
        .collect();

    overlaps.sort_by(|a, b| a.file.cmp(&b.file));
    Ok(overlaps)
}

pub fn execute(ctx: &CommandContext, json: bool) -> Result<()> {
    let overlaps = find_overlaps(ctx)?;

    if json {
        let json_val: Vec<serde_json::Value> = overlaps
            .iter()
            .map(|o| {
                serde_json::json!({
                    "file": o.file.to_string_lossy(),
                    "solutions": o.solutions.iter().map(|(id, title)| {
                        serde_json::json!({ "id": id, "title": title })
                    }).collect::<Vec<_>>(),
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&json_val)?);
        return Ok(());
    }

    if overlaps.is_empty() {
        println!("No file overlaps between active solutions.");
        return Ok(());
    }

    println!("File overlaps between active solutions:\n");
    for overlap in &overlaps {
        println!("  {}", overlap.file.display());
        for (id, title) in &overlap.solutions {
            let short_id = &id[..6.min(id.len())];
            println!("    s/{} \"{}\"", short_id, title);
        }
        println!();
    }

    Ok(())
}
