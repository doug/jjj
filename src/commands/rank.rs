use std::collections::HashMap;
use std::io::Write;

use chrono::Utc;
use serde::Serialize;

use crate::cli::RankAction;
use crate::context::CommandContext;
use crate::display::short_id;
use crate::error::Result;
use crate::ranking::glicko2::{compute_ratings, sorted_ranking, Comparison, WeightedComparison};
use crate::ranking::matchups::suggest_matchups;
use crate::ranking::store::{
    append_comparison, load_attributed_comparisons, load_comparisons, sanitize_user,
};
use crate::utils::truncate;

/// Dispatch a `jjj rank` subcommand.
pub fn execute(ctx: &CommandContext, action: RankAction) -> Result<()> {
    match action {
        RankAction::Session { milestone, count } => session(ctx, milestone, count),
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

/// Check whether `user_slug` matches the milestone owner identity.
fn user_matches_owner(user_slug: &str, owner: &str) -> bool {
    sanitize_user(owner) == user_slug
}

/// Build weighted comparisons from raw comparisons.
///
/// Comparisons made by the milestone owner get 2x weight; everyone else gets 1x.
fn build_weighted(
    comparisons: &[Comparison],
    attributed: &[(Comparison, String)],
    owner: Option<&str>,
) -> Vec<WeightedComparison> {
    // Build a lookup from (winner, loser, ts) -> user_slug for attribution.
    let attr_map: HashMap<(String, String, i64), &str> = attributed
        .iter()
        .map(|(c, user)| {
            (
                (
                    c.winner.clone(),
                    c.loser.clone(),
                    c.ts.timestamp_nanos_opt().unwrap_or(0),
                ),
                user.as_str(),
            )
        })
        .collect();

    comparisons
        .iter()
        .map(|c| {
            let key = (
                c.winner.clone(),
                c.loser.clone(),
                c.ts.timestamp_nanos_opt().unwrap_or(0),
            );
            let weight = match (owner, attr_map.get(&key)) {
                (Some(owner_id), Some(user_slug)) if user_matches_owner(user_slug, owner_id) => 2.0,
                _ => 1.0,
            };
            WeightedComparison {
                winner: c.winner.clone(),
                loser: c.loser.clone(),
                weight,
            }
        })
        .collect()
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
// jjj rank session
// ---------------------------------------------------------------------------

/// Interactive guided ranking session.
fn session(ctx: &CommandContext, milestone: Option<String>, count: usize) -> Result<()> {
    let milestone_id = resolve_milestone_for_rank(ctx, milestone)?;
    let ms = ctx.store.load_milestone(&milestone_id)?;
    let (problem_ids, titles) = open_problems_in_milestone(ctx, &milestone_id)?;

    if problem_ids.len() < 2 {
        println!(
            "Need at least 2 open problems in milestone '{}' to rank.",
            ms.title
        );
        return Ok(());
    }

    let user = ctx.jj().user_identity()?;
    let user_slug = sanitize_user(&user);

    // Load existing data to seed ratings for matchup selection.
    let comparisons = load_comparisons(ctx.store.meta_path(), &milestone_id)?;
    let attributed = load_attributed_comparisons(ctx.store.meta_path(), &milestone_id)?;
    let owner = ms.assignee.as_deref();
    let weighted = build_weighted(&comparisons, &attributed, owner);

    let mut ratings = compute_ratings(&weighted);
    // Ensure every open problem has a rating entry.
    for pid in &problem_ids {
        ratings.entry(pid.clone()).or_default();
    }
    // Remove any ratings for problems no longer in scope.
    ratings.retain(|k, _| problem_ids.contains(k));

    // Build recent pairs from this user to avoid re-asking the same question.
    let recent_pairs: Vec<(String, String)> = attributed
        .iter()
        .filter(|(_, u)| u == &user_slug)
        .map(|(c, _)| (c.winner.clone(), c.loser.clone()))
        .collect();

    let matchups = suggest_matchups(&ratings, &recent_pairs, count);

    if matchups.is_empty() {
        println!("No new matchups available — you have compared every pair.");
        return Ok(());
    }

    println!("Ranking problems in milestone: {}\n", ms.title);
    println!("For each pair, press:  [A] first wins  [B] second wins  [S] skip  [Q] quit\n");

    let mut completed = 0usize;
    let mut skipped = 0usize;

    // Enter raw mode for single-keypress input.
    crossterm::terminal::enable_raw_mode()?;

    let result = run_session_loop(
        ctx,
        &milestone_id,
        &user,
        &matchups,
        &titles,
        &mut completed,
        &mut skipped,
    );

    // Always restore terminal state.
    crossterm::terminal::disable_raw_mode()?;

    // Propagate any error from the session loop.
    result?;

    println!(
        "\nSession complete: {} comparison{} recorded, {} skipped.",
        completed,
        if completed == 1 { "" } else { "s" },
        skipped,
    );

    Ok(())
}

/// Inner loop for the ranking session, separated so we can always disable raw
/// mode in the caller even if this returns an error.
fn run_session_loop(
    ctx: &CommandContext,
    milestone_id: &str,
    user: &str,
    matchups: &[(String, String)],
    titles: &HashMap<String, String>,
    completed: &mut usize,
    skipped: &mut usize,
) -> Result<()> {
    let mut stdout = std::io::stdout();

    for (i, (a, b)) in matchups.iter().enumerate() {
        let title_a = titles
            .get(a)
            .map(|t| truncate(t, 60))
            .unwrap_or_else(|| short_id(a).to_string());
        let title_b = titles
            .get(b)
            .map(|t| truncate(t, 60))
            .unwrap_or_else(|| short_id(b).to_string());

        write!(
            stdout,
            "[{}/{}]  [A] {}  vs  [B] {}  ? ",
            i + 1,
            matchups.len(),
            title_a,
            title_b,
        )?;
        stdout.flush()?;

        loop {
            if let crossterm::event::Event::Key(crossterm::event::KeyEvent {
                code,
                kind: crossterm::event::KeyEventKind::Press,
                ..
            }) = crossterm::event::read()?
            {
                match code {
                    crossterm::event::KeyCode::Char('a' | 'A') => {
                        write!(stdout, "A\r\n")?;
                        stdout.flush()?;
                        let comparison = Comparison {
                            winner: a.clone(),
                            loser: b.clone(),
                            ts: Utc::now(),
                        };
                        let msg = format!("rank: {} > {}", short_id(a), short_id(b));
                        let base = ctx.store.meta_path().to_path_buf();
                        let ms_id = milestone_id.to_string();
                        let usr = user.to_string();
                        ctx.store.with_metadata(&msg, || {
                            append_comparison(&base, &ms_id, &usr, &comparison)
                        })?;
                        *completed += 1;
                        break;
                    }
                    crossterm::event::KeyCode::Char('b' | 'B') => {
                        write!(stdout, "B\r\n")?;
                        stdout.flush()?;
                        let comparison = Comparison {
                            winner: b.clone(),
                            loser: a.clone(),
                            ts: Utc::now(),
                        };
                        let msg = format!("rank: {} > {}", short_id(b), short_id(a));
                        let base = ctx.store.meta_path().to_path_buf();
                        let ms_id = milestone_id.to_string();
                        let usr = user.to_string();
                        ctx.store.with_metadata(&msg, || {
                            append_comparison(&base, &ms_id, &usr, &comparison)
                        })?;
                        *completed += 1;
                        break;
                    }
                    crossterm::event::KeyCode::Char('s' | 'S') => {
                        write!(stdout, "skip\r\n")?;
                        stdout.flush()?;
                        *skipped += 1;
                        break;
                    }
                    crossterm::event::KeyCode::Char('q' | 'Q') | crossterm::event::KeyCode::Esc => {
                        write!(stdout, "quit\r\n")?;
                        stdout.flush()?;
                        return Ok(());
                    }
                    _ => {}
                }
            }
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// jjj rank show
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct RankEntry {
    rank: usize,
    problem_id: String,
    title: String,
    rating: f64,
    confidence: String,
    comparisons: usize,
}

/// Display computed rankings for a milestone.
fn show(ctx: &CommandContext, milestone: Option<String>, by_user: bool, json: bool) -> Result<()> {
    let milestone_id = resolve_milestone_for_rank(ctx, milestone)?;
    let ms = ctx.store.load_milestone(&milestone_id)?;
    let (problem_ids, titles) = open_problems_in_milestone(ctx, &milestone_id)?;

    let comparisons = load_comparisons(ctx.store.meta_path(), &milestone_id)?;
    let attributed = load_attributed_comparisons(ctx.store.meta_path(), &milestone_id)?;

    if comparisons.is_empty() {
        if json {
            println!("[]");
        } else {
            println!(
                "No rankings yet for milestone '{}'. Start with `jjj rank session`.",
                ms.title,
            );
        }
        return Ok(());
    }

    let owner = ms.assignee.as_deref();

    if by_user {
        show_by_user(
            ctx,
            &milestone_id,
            &attributed,
            &problem_ids,
            &titles,
            owner,
            json,
        )?;
    } else {
        let weighted = build_weighted(&comparisons, &attributed, owner);
        let ratings = compute_ratings(&weighted);
        let ranked = sorted_ranking(&ratings);

        // Count comparisons per problem.
        let mut cmp_counts: HashMap<String, usize> = HashMap::new();
        for c in &comparisons {
            *cmp_counts.entry(c.winner.clone()).or_default() += 1;
            *cmp_counts.entry(c.loser.clone()).or_default() += 1;
        }

        // Build entries (only for problems still in the milestone).
        let entries: Vec<RankEntry> = ranked
            .iter()
            .filter(|(id, _)| problem_ids.contains(id))
            .enumerate()
            .map(|(i, (id, rating))| {
                let title = titles.get(id).cloned().unwrap_or_default();
                RankEntry {
                    rank: i + 1,
                    problem_id: id.clone(),
                    title,
                    rating: (rating.mu * 10.0).round() / 10.0,
                    confidence: rating.confidence().to_string(),
                    comparisons: *cmp_counts.get(id).unwrap_or(&0),
                }
            })
            .collect();

        if json {
            println!("{}", serde_json::to_string_pretty(&entries)?);
        } else {
            println!("Rankings for milestone: {}\n", ms.title);
            println!(
                "  {:<5} {:<45} {:>7} {:>10} {:>5}",
                "Rank", "Problem", "Rating", "Confidence", "Cmps"
            );
            println!("  {}", "-".repeat(75));
            for e in &entries {
                println!(
                    "  {:<5} {:<45} {:>7.1} {:>10} {:>5}",
                    e.rank,
                    truncate(&e.title, 44),
                    e.rating,
                    e.confidence,
                    e.comparisons,
                );
            }
        }
    }

    Ok(())
}

/// Show rankings broken down by individual user.
fn show_by_user(
    _ctx: &CommandContext,
    _milestone_id: &str,
    attributed: &[(Comparison, String)],
    problem_ids: &[String],
    titles: &HashMap<String, String>,
    owner: Option<&str>,
    json: bool,
) -> Result<()> {
    // Group comparisons by user.
    let mut by_user: HashMap<String, Vec<&Comparison>> = HashMap::new();
    for (cmp, user) in attributed {
        by_user.entry(user.clone()).or_default().push(cmp);
    }

    let mut users: Vec<&String> = by_user.keys().collect();
    users.sort();

    if json {
        let mut all_data: HashMap<&str, Vec<RankEntry>> = HashMap::new();

        for user in &users {
            let user_cmps: Vec<WeightedComparison> = by_user[*user]
                .iter()
                .map(|c| {
                    let w = match owner {
                        Some(o) if user_matches_owner(user, o) => 2.0,
                        _ => 1.0,
                    };
                    WeightedComparison {
                        winner: c.winner.clone(),
                        loser: c.loser.clone(),
                        weight: w,
                    }
                })
                .collect();

            let ratings = compute_ratings(&user_cmps);
            let ranked = sorted_ranking(&ratings);

            let entries: Vec<RankEntry> = ranked
                .iter()
                .filter(|(id, _)| problem_ids.contains(id))
                .enumerate()
                .map(|(i, (id, rating))| RankEntry {
                    rank: i + 1,
                    problem_id: id.clone(),
                    title: titles.get(id).cloned().unwrap_or_default(),
                    rating: (rating.mu * 10.0).round() / 10.0,
                    confidence: rating.confidence().to_string(),
                    comparisons: by_user[*user]
                        .iter()
                        .filter(|c| c.winner == *id || c.loser == *id)
                        .count(),
                })
                .collect();

            all_data.insert(user.as_str(), entries);
        }

        println!("{}", serde_json::to_string_pretty(&all_data)?);
    } else {
        for user in &users {
            let is_owner = owner.is_some_and(|o| user_matches_owner(user, o));
            let label = if is_owner {
                format!("{} (owner, 2x weight)", user)
            } else {
                user.to_string()
            };
            println!("\n--- {} ---\n", label);

            let user_cmps: Vec<WeightedComparison> = by_user[*user]
                .iter()
                .map(|c| WeightedComparison {
                    winner: c.winner.clone(),
                    loser: c.loser.clone(),
                    weight: 1.0,
                })
                .collect();

            let ratings = compute_ratings(&user_cmps);
            let ranked = sorted_ranking(&ratings);

            println!(
                "  {:<5} {:<45} {:>7} {:>10}",
                "Rank", "Problem", "Rating", "Confidence"
            );
            println!("  {}", "-".repeat(70));
            for (i, (id, rating)) in ranked
                .iter()
                .filter(|(id, _)| problem_ids.contains(id))
                .enumerate()
            {
                let title = titles.get(id).cloned().unwrap_or_default();
                println!(
                    "  {:<5} {:<45} {:>7.1} {:>10}",
                    i + 1,
                    truncate(&title, 44),
                    (rating.mu * 10.0).round() / 10.0,
                    rating.confidence(),
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
        println!(
            "No comparison history for milestone '{}'. Start with `jjj rank session`.",
            ms.title,
        );
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
            .unwrap_or_else(|| short_id(&cmp.winner).to_string());
        let loser_title = titles
            .get(&cmp.loser)
            .map(|t| truncate(t, 28))
            .unwrap_or_else(|| short_id(&cmp.loser).to_string());

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
