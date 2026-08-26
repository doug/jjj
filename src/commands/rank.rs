use std::collections::HashMap;

use serde::Serialize;

use crate::cli::RankAction;
use crate::context::CommandContext;
use crate::error::Result;
use crate::ranking::ordering::load_all_orderings;
use crate::ranking::scoring::aggregate_rankings;
use crate::utils::truncate;

/// Dispatch a `jjj rank` subcommand.
pub fn execute(ctx: &CommandContext, action: RankAction) -> Result<()> {
    match action {
        RankAction::Show {
            milestone,
            by_user,
            json,
        } => show(ctx, milestone, by_user, json),
        RankAction::Set {
            problems,
            milestone,
            gaps,
            json,
        } => set_order(ctx, problems, milestone, gaps, json),
        RankAction::Move {
            problem,
            position,
            milestone,
            json,
        } => move_one(ctx, problem, position, milestone, json),
    }
}

/// Record a full priority order for a milestone.
///
/// Ordering used to be reachable only through the TUI, so anything without a
/// terminal could read a ranking but never author one — which left a whole half
/// of jjj's model unusable by the agents it was designed to coordinate.
fn set_order(
    ctx: &CommandContext,
    problems: Vec<String>,
    milestone: Option<String>,
    gaps: Vec<String>,
    json: bool,
) -> Result<()> {
    // `-` means "read the list from stdin". Piping is the natural way for a
    // script to hand over a list it just computed, and avoids the failure that
    // killed four of nine ranking attempts in one trial: a shell variable that
    // expanded to nothing, silently shortening the argument list.
    let (problems, stdin_gaps) = if problems.len() == 1 && problems[0] == "-" {
        read_order_from_stdin()?
    } else {
        (problems, Vec::new())
    };

    if problems.is_empty() {
        return Err(crate::error::JjjError::Validation(
            "no problems given — pass them in priority order, highest first, \
             or `-` to read them from stdin"
                .to_string(),
        ));
    }
    let milestone_id = resolve_milestone_for_rank(ctx, milestone)?;
    let user = ctx.store.get_current_user()?;

    let mut order = Vec::with_capacity(problems.len());
    for p in &problems {
        let id = ctx.resolve_problem(p)?;
        if order.contains(&id) {
            return Err(crate::error::JjjError::Validation(format!(
                "'{p}' appears twice — an ordering is a sequence, not a multiset"
            )));
        }
        order.push(id);
    }

    // Gaps may arrive either way; the flag wins on a conflict so an explicit
    // argument is never silently overridden by piped input.
    let mut all_gaps = stdin_gaps;
    all_gaps.extend(gaps);
    let gap_map = parse_gaps(ctx, &all_gaps, &order)?;
    write_ordering(ctx, &milestone_id, &user, order, gap_map, json)
}

/// Read a priority order from stdin.
///
/// Accepts either the JSON this command prints with `--json` — so an ordering
/// round-trips — or the plainer form of one reference per line, each optionally
/// suffixed with `:S`, `:M`, `:L` or `:XL` for the gap below it. Blank lines and
/// `#` comments are ignored, so a generated list stays readable.
fn read_order_from_stdin() -> Result<(Vec<String>, Vec<String>)> {
    use std::io::Read;

    let mut buf = String::new();
    std::io::stdin()
        .read_to_string(&mut buf)
        .map_err(crate::error::JjjError::Io)?;

    let trimmed = buf.trim();
    if trimmed.is_empty() {
        return Err(crate::error::JjjError::Validation(
            "nothing on stdin — expected one problem reference per line".to_string(),
        ));
    }

    if trimmed.starts_with('{') {
        #[derive(serde::Deserialize)]
        struct Piped {
            order: Vec<String>,
            #[serde(default)]
            gaps: std::collections::HashMap<String, String>,
        }
        let piped: Piped = serde_json::from_str(trimmed).map_err(|e| {
            crate::error::JjjError::Validation(format!(
                "stdin looked like JSON but did not parse: {e}"
            ))
        })?;
        let gaps = piped
            .gaps
            .iter()
            .map(|(id, size)| format!("{id}:{size}"))
            .collect();
        return Ok((piped.order, gaps));
    }

    let mut order = Vec::new();
    let mut gaps = Vec::new();
    for line in trimmed.lines() {
        let line = line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        match line.rsplit_once(':') {
            Some((reference, size))
                if matches!(size.to_ascii_uppercase().as_str(), "S" | "M" | "L" | "XL") =>
            {
                order.push(reference.trim().to_string());
                gaps.push(line.to_string());
            }
            _ => order.push(line.to_string()),
        }
    }
    Ok((order, gaps))
}

/// Move one problem within an existing order.
///
/// Restating a whole ordering to move one item is how orderings get corrupted
/// by callers that only meant to change one thing.
fn move_one(
    ctx: &CommandContext,
    problem: String,
    position: String,
    milestone: Option<String>,
    json: bool,
) -> Result<()> {
    let milestone_id = resolve_milestone_for_rank(ctx, milestone)?;
    let user = ctx.store.get_current_user()?;
    let id = ctx.resolve_problem(&problem)?;

    let base = ctx.store.meta_path();
    let existing = crate::ranking::ordering::load_user_ordering(base, &milestone_id, &user)?
        .ok_or_else(|| {
            crate::error::JjjError::Validation(
                "you have no ordering for this milestone yet — use `jjj rank set` first"
                    .to_string(),
            )
        })?;

    let mut order = existing.order.clone();
    let from = order.iter().position(|x| x == &id).ok_or_else(|| {
        crate::error::JjjError::Validation(format!(
            "'{problem}' is not in your ordering — add it with `jjj rank set`"
        ))
    })?;
    order.remove(from);

    let to = match position.as_str() {
        "top" => 0,
        "bottom" => order.len(),
        "up" => from.saturating_sub(1),
        "down" => (from + 1).min(order.len()),
        other => match other.strip_prefix("before:") {
            Some(target) => {
                let tid = ctx.resolve_problem(target)?;
                order.iter().position(|x| x == &tid).ok_or_else(|| {
                    crate::error::JjjError::Validation(format!(
                        "'{target}' is not in your ordering"
                    ))
                })?
            }
            None => {
                return Err(crate::error::JjjError::Validation(format!(
                    "unknown position '{other}' — use top, bottom, up, down, or before:<problem>"
                )))
            }
        },
    };
    order.insert(to, id);
    write_ordering(ctx, &milestone_id, &user, order, existing.gaps, json)
}

/// Parse `<problem>:<size>` gap arguments against an ordering.
fn parse_gaps(
    ctx: &CommandContext,
    gaps: &[String],
    order: &[String],
) -> Result<std::collections::HashMap<String, crate::ranking::ordering::GapSize>> {
    use crate::ranking::ordering::GapSize;
    let mut out = std::collections::HashMap::new();
    for g in gaps {
        let (p, size) = g.rsplit_once(':').ok_or_else(|| {
            crate::error::JjjError::Validation(format!(
                "malformed --gap '{g}' — expected <problem>:<S|M|L|XL>"
            ))
        })?;
        let size = match size.to_ascii_uppercase().as_str() {
            "S" => GapSize::S,
            "M" => GapSize::M,
            "L" => GapSize::L,
            "XL" => GapSize::XL,
            other => {
                return Err(crate::error::JjjError::Validation(format!(
                    "unknown gap size '{other}' — use S, M, L or XL"
                )))
            }
        };
        let id = ctx.resolve_problem(p)?;
        if !order.contains(&id) {
            return Err(crate::error::JjjError::Validation(format!(
                "--gap names '{p}', which is not in the ordering"
            )));
        }
        out.insert(id, size);
    }
    Ok(out)
}

/// Persist an ordering and report it.
fn write_ordering(
    ctx: &CommandContext,
    milestone_id: &str,
    user: &str,
    order: Vec<String>,
    gaps: std::collections::HashMap<String, crate::ranking::ordering::GapSize>,
    json: bool,
) -> Result<()> {
    use crate::ranking::ordering::{save_user_ordering, UserOrdering};

    let ordering = UserOrdering {
        order,
        gaps,
        updated_at: chrono::Utc::now(),
    };
    let base = ctx.store.meta_path().to_path_buf();
    let m = milestone_id.to_string();
    let u = user.to_string();
    let o = ordering.clone();
    ctx.store.with_metadata(
        &format!("Rank problems for milestone {milestone_id}"),
        || save_user_ordering(&base, &m, &u, &o),
    )?;

    if json {
        println!("{}", serde_json::to_string_pretty(&ordering)?);
        return Ok(());
    }
    println!("Ranked {} problem(s) as @{}:", ordering.order.len(), user);
    for (i, id) in ordering.order.iter().enumerate() {
        let title = ctx
            .store
            .load_problem(id)
            .map(|p| p.title)
            .unwrap_or_else(|_| id.clone());
        let gap = ordering
            .gaps
            .get(id)
            .map(|g| format!("  [{} gap below]", g.label()))
            .unwrap_or_default();
        println!("  {}. {}{}", i + 1, title, gap);
    }
    Ok(())
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
    /// Gap *below* this item ("S"/"M"/"L"/"XL"), or empty for the implicit unit gap.
    gap: String,
}

#[derive(Serialize)]
struct UserBreakdown {
    ordering: Vec<UserOrderingEntry>,
}

/// Display computed rankings for a milestone.
fn show(ctx: &CommandContext, milestone: Option<String>, by_user: bool, json: bool) -> Result<()> {
    let milestone_id = resolve_milestone_for_rank(ctx, milestone)?;
    let ms = ctx.store.load_milestone(&milestone_id)?;
    let (problem_ids, titles) = open_problems_in_milestone(ctx, &milestone_id)?;

    // Canonical problem_count for the QV budget — must match the TUI vote-entry
    // budget (all problems in the milestone), not the open-problem subset, or
    // votes accepted in the TUI get dropped here as over-budget.
    let problem_count = crate::ranking::scoring::milestone_problem_count(
        &ctx.store.list_problems()?,
        &milestone_id,
    );

    let orderings = load_all_orderings(ctx.store.meta_path(), &milestone_id)?;

    if orderings.is_empty() {
        if json {
            println!("[]");
        } else {
            println!(
                "No rankings yet for milestone '{}'. Set one with `jjj rank set <problem>... [--gap <problem>:<S|M|L|XL>]`, or open the TUI (`jjj ui`) and use the ranking view.",
                ms.title,
            );
        }
        return Ok(());
    }

    if by_user {
        show_by_user(&orderings, &problem_ids, problem_count, &titles, json)?;
    } else {
        let ranked = aggregate_rankings(&orderings, problem_count);

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
    _problem_count: usize,
    titles: &HashMap<String, String>,
    json: bool,
) -> Result<()> {
    let mut users: Vec<&String> = orderings.keys().collect();
    users.sort();

    if json {
        let mut all_data: HashMap<&str, UserBreakdown> = HashMap::new();

        for user in &users {
            let ordering = &orderings[*user];

            let entries: Vec<UserOrderingEntry> = ordering
                .order
                .iter()
                .filter(|id| problem_ids.contains(id))
                .enumerate()
                .map(|(i, id)| UserOrderingEntry {
                    rank: i + 1,
                    problem_id: id.clone(),
                    title: titles.get(id).cloned().unwrap_or_default(),
                    gap: ordering
                        .gaps
                        .get(id)
                        .map(|g| g.label().to_string())
                        .unwrap_or_default(),
                })
                .collect();

            all_data.insert(user.as_str(), UserBreakdown { ordering: entries });
        }

        println!("{}", serde_json::to_string_pretty(&all_data)?);
    } else {
        for user in &users {
            let ordering = &orderings[*user];
            println!("\n--- {} ---\n", user);

            println!("  {:<5} {:<45} {:>5}", "Rank", "Problem", "Gap");
            println!("  {}", "-".repeat(57));
            for (i, id) in ordering
                .order
                .iter()
                .filter(|id| problem_ids.contains(id))
                .enumerate()
            {
                let title = titles.get(id).cloned().unwrap_or_default();
                let gap_str = ordering.gaps.get(id).map(|g| g.label()).unwrap_or("");
                println!("  {:<5} {:<45} {:>5}", i + 1, truncate(&title, 44), gap_str,);
            }
        }
    }

    Ok(())
}
