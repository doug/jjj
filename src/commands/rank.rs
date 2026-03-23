use std::collections::HashMap;

use serde::Serialize;

use crate::cli::RankAction;
use crate::context::CommandContext;
use crate::error::Result;
use crate::ranking::borda::{aggregate_rankings, qv_budget, total_vote_cost};
use crate::ranking::ordering::load_all_orderings;
use crate::ranking::store::{load_attributed_comparisons, sanitize_user};
use crate::utils::truncate;

/// Dispatch a `jjj rank` subcommand.
pub fn execute(ctx: &CommandContext, action: RankAction) -> Result<()> {
    match action {
        RankAction::Show {
            milestone,
            by_user,
            json,
        } => show(ctx, milestone, by_user, json),
        RankAction::History { milestone, limit } => history(ctx, milestone, limit),
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Resolve a milestone ID for ranking.
///
/// If the user gave a milestone reference, resolve it. Otherwise pick the first
/// active milestone. Returns an error if no active milestone can be found.
fn resolve_milestone_for_rank(ctx: &CommandContext, input: Option<String>) -> Result<String> {
    if let Some(ref ms) = input {
        return ctx.resolve_milestone(ms);
    }

    let milestones = ctx.store.list_milestones()?;
    for m in &milestones {
        if m.is_active() {
            return Ok(m.id.clone());
        }
    }

    Err(crate::error::JjjError::Validation(
        "No active milestone found. Create one with `jjj milestone new` or specify a milestone."
            .into(),
    ))
}

/// Collect the set of open problem IDs belonging to a milestone, along with a
/// lookup table of problem ID -> title.
fn open_problems_in_milestone(
    ctx: &CommandContext,
    milestone_id: &str,
) -> Result<(Vec<String>, HashMap<String, String>)> {
    let milestone = ctx.store.load_milestone(milestone_id)?;
    let all_problems = ctx.store.list_problems()?;

    let mut ids = Vec::new();
    let mut titles = HashMap::new();

    for p in &all_problems {
        if p.milestone_id.as_deref() == Some(milestone_id) && p.is_open() {
            ids.push(p.id.clone());
            titles.insert(p.id.clone(), p.title.clone());
        }
    }

    // Also check milestone.problem_ids for problems that have the milestone reference
    // stored on the milestone side rather than the problem side.
    for pid in &milestone.problem_ids {
        if !titles.contains_key(pid) {
            if let Ok(p) = ctx.store.load_problem(pid) {
                if p.is_open() {
                    ids.push(p.id.clone());
                    titles.insert(p.id.clone(), p.title.clone());
                }
            }
        }
    }

    Ok((ids, titles))
}

// ---------------------------------------------------------------------------
// jjj rank show
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct RankEntry {
    rank: usize,
    problem_id: String,
    title: String,
    score: f64,
    voters: usize,
}

#[derive(Serialize)]
struct UserOrderingEntry {
    rank: usize,
    problem_id: String,
    title: String,
    votes: u32,
}

#[derive(Serialize)]
struct UserBreakdown {
    is_owner: bool,
    budget: u32,
    budget_used: u32,
    ordering: Vec<UserOrderingEntry>,
}

/// Display computed rankings for a milestone.
fn show(ctx: &CommandContext, milestone: Option<String>, by_user: bool, json: bool) -> Result<()> {
    let milestone_id = resolve_milestone_for_rank(ctx, milestone)?;
    let ms = ctx.store.load_milestone(&milestone_id)?;
    let (problem_ids, titles) = open_problems_in_milestone(ctx, &milestone_id)?;

    let orderings = load_all_orderings(ctx.store.meta_path(), &milestone_id)?;

    if orderings.is_empty() {
        if json {
            println!("[]");
        } else {
            println!(
                "No rankings yet for milestone '{}'. Use the TUI to create personal orderings.",
                ms.title,
            );
        }
        return Ok(());
    }

    let owner = ms.assignee.as_deref();
    let owner_slug = owner.map(sanitize_user);

    if by_user {
        show_by_user(
            &orderings,
            &problem_ids,
            &titles,
            owner_slug.as_deref(),
            json,
        )?;
    } else {
        let ranked = aggregate_rankings(&orderings, owner_slug.as_deref(), problem_ids.len());

        // Build entries (only for problems still in the milestone).
        let entries: Vec<RankEntry> = ranked
            .iter()
            .filter(|(id, _)| problem_ids.contains(id))
            .enumerate()
            .map(|(i, (id, agg))| {
                let title = titles.get(id).cloned().unwrap_or_default();
                RankEntry {
                    rank: i + 1,
                    problem_id: id.clone(),
                    title,
                    score: (agg.score * 10.0).round() / 10.0,
                    voters: agg.voter_count,
                }
            })
            .collect();

        if json {
            println!("{}", serde_json::to_string_pretty(&entries)?);
        } else {
            println!("Rankings for milestone: {}\n", ms.title);
            println!(
                "  {:<5} {:<45} {:>7} {:>7}",
                "Rank", "Problem", "Score", "Voters"
            );
            println!("  {}", "-".repeat(66));
            for e in &entries {
                println!(
                    "  {:<5} {:<45} {:>7.1} {:>7}",
                    e.rank,
                    truncate(&e.title, 44),
                    e.score,
                    e.voters,
                );
            }
        }
    }

    Ok(())
}

/// Show rankings broken down by individual user.
fn show_by_user(
    orderings: &HashMap<String, crate::ranking::ordering::UserOrdering>,
    problem_ids: &[String],
    titles: &HashMap<String, String>,
    owner_slug: Option<&str>,
    json: bool,
) -> Result<()> {
    let mut users: Vec<&String> = orderings.keys().collect();
    users.sort();

    let budget = qv_budget(problem_ids.len());

    if json {
        let mut all_data: HashMap<&str, UserBreakdown> = HashMap::new();

        for user in &users {
            let ordering = &orderings[*user];
            let is_owner = owner_slug == Some(user.as_str());
            let budget_used = total_vote_cost(&ordering.votes);

            let entries: Vec<UserOrderingEntry> = ordering
                .order
                .iter()
                .filter(|id| problem_ids.contains(id))
                .enumerate()
                .map(|(i, id)| UserOrderingEntry {
                    rank: i + 1,
                    problem_id: id.clone(),
                    title: titles.get(id).cloned().unwrap_or_default(),
                    votes: ordering.votes.get(id).copied().unwrap_or(0),
                })
                .collect();

            all_data.insert(
                user.as_str(),
                UserBreakdown {
                    is_owner,
                    budget,
                    budget_used,
                    ordering: entries,
                },
            );
        }

        println!("{}", serde_json::to_string_pretty(&all_data)?);
    } else {
        for user in &users {
            let ordering = &orderings[*user];
            let is_owner = owner_slug == Some(user.as_str());
            let label = if is_owner {
                format!("{} (owner, 2x weight)", user)
            } else {
                user.to_string()
            };
            let budget_used = total_vote_cost(&ordering.votes);
            println!("\n--- {} ---", label);
            println!("  QV budget: {}/{} used\n", budget_used, budget);

            println!("  {:<5} {:<45} {:>5}", "Rank", "Problem", "Votes");
            println!("  {}", "-".repeat(57));
            for (i, id) in ordering
                .order
                .iter()
                .filter(|id| problem_ids.contains(id))
                .enumerate()
            {
                let title = titles.get(id).cloned().unwrap_or_default();
                let votes = ordering.votes.get(id).copied().unwrap_or(0);
                let votes_str = if votes > 0 {
                    format!("+{}", votes)
                } else {
                    String::new()
                };
                println!(
                    "  {:<5} {:<45} {:>5}",
                    i + 1,
                    truncate(&title, 44),
                    votes_str,
                );
            }
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// jjj rank history
// ---------------------------------------------------------------------------

/// Display comparison history for a milestone.
fn history(ctx: &CommandContext, milestone: Option<String>, limit: usize) -> Result<()> {
    let milestone_id = resolve_milestone_for_rank(ctx, milestone)?;
    let ms = ctx.store.load_milestone(&milestone_id)?;

    let mut attributed = load_attributed_comparisons(ctx.store.meta_path(), &milestone_id)?;

    if attributed.is_empty() {
        println!("No comparison history for milestone '{}'.", ms.title,);
        return Ok(());
    }

    // Build a problem title lookup.
    let all_problems = ctx.store.list_problems()?;
    let titles: HashMap<String, String> = all_problems
        .iter()
        .map(|p| (p.id.clone(), p.title.clone()))
        .collect();

    // Most recent first.
    attributed.sort_by(|a, b| b.0.ts.cmp(&a.0.ts));
    let shown = attributed.iter().take(limit);

    println!("Comparison history for milestone: {}\n", ms.title);
    println!(
        "  {:<22} {:<16} {:<30} {:<30}",
        "Timestamp", "User", "Winner", "Loser"
    );
    println!("  {}", "-".repeat(96));

    for (cmp, user) in shown {
        let ts = cmp.ts.format("%Y-%m-%d %H:%M:%S");
        let winner_title = titles
            .get(&cmp.winner)
            .map(|t| truncate(t, 28))
            .unwrap_or_else(|| cmp.winner[..8.min(cmp.winner.len())].to_string());
        let loser_title = titles
            .get(&cmp.loser)
            .map(|t| truncate(t, 28))
            .unwrap_or_else(|| cmp.loser[..8.min(cmp.loser.len())].to_string());

        println!(
            "  {:<22} {:<16} {:<30} {:<30}",
            ts,
            truncate(user, 15),
            winner_title,
            loser_title,
        );
    }

    if attributed.len() > limit {
        println!(
            "\n  (showing {} of {} — use --limit to see more)",
            limit,
            attributed.len()
        );
    }

    Ok(())
}
