//! `jjj prime` — one call that orients an agent at the start of a turn.
//!
//! An agent turn begins with no memory of the last one and re-derives the world
//! from scratch. Measured in a swarm run: **2,322 orientation calls against 138
//! that produced anything**, a ratio of 17 to 1. The harness worked around it by
//! hand-rolling a briefing out of four `jjj` invocations and sixty lines of
//! embedded Python — which only helped agents running under that harness, and
//! could not be tested.
//!
//! This is that briefing as a command: one process, one pass over the metadata,
//! and the same answer available to anything that can run `jjj`.
//!
//! # What goes in, and what does not
//!
//! Only state an agent cannot guess and will otherwise query for: what to work
//! on, what is already in flight so it does not duplicate it, what has already
//! been measured so it does not re-measure it, and whether anything is blocked
//! on a person. Workflow *instructions* are deliberately absent — those live in
//! the skill file and the target's brief, which are loaded by the harness. A
//! command that printed both would go stale in two places at once.
//!
//! The output is markdown because the consumer is a language model, and short
//! because the point is to spend fewer tokens than the questions it replaces.

use crate::context::CommandContext;
use crate::error::Result;
use crate::models::{CritiqueStatus, FindingStatus, ProblemStatus, SolutionStatus};

/// How many items to list per section before summarising the rest.
///
/// A briefing that grows with the corpus stops being a briefing. Five is enough
/// to see the shape of what is in flight; the counts carry the rest.
const PER_SECTION: usize = 5;

pub fn execute(ctx: &CommandContext, json: bool, full: bool) -> Result<()> {
    let store = &ctx.store;
    let limit = if full { usize::MAX } else { PER_SECTION };

    let actor = store.get_current_user().unwrap_or_default();
    let problems = store.list_problems()?;
    let solutions = store.list_solutions()?;
    let critiques = store.list_critiques()?;
    let findings = store.list_findings()?;
    let events = store.list_events_cached()?;
    let escalations = crate::escalation::open_escalations(&events);

    // A solution carries an assignee but not an author, so recover who proposed
    // what from the creation event. Without this the briefing says "@?" for
    // every unassigned proposal, which is exactly the field an agent needs in
    // order to know whose idea it would be duplicating.
    let author_of: std::collections::HashMap<&str, &str> = events
        .iter()
        .filter(|e| e.event_type == crate::models::EventType::SolutionCreated && !e.by.is_empty())
        .map(|e| (e.entity.as_str(), e.by.as_str()))
        .collect();

    // What this actor should do next, by the same reckoning `jjj next` uses, so
    // the briefing never disagrees with the command an agent runs after reading
    // it.
    let ttl = crate::claim::claim_ttl(&store.load_config().unwrap_or_default().settings);
    let next = crate::commands::status::build_next_actions_with_ttl(
        &problems, &solutions, &critiques, &actor, false, ttl,
    );

    let submitted: Vec<_> = solutions
        .iter()
        .filter(|s| s.status == SolutionStatus::Submitted)
        .collect();
    let proposed: Vec<_> = solutions
        .iter()
        .filter(|s| s.status == SolutionStatus::Proposed)
        .collect();
    let open_critiques: Vec<_> = critiques
        .iter()
        .filter(|c| c.status == CritiqueStatus::Open)
        .collect();
    let current_findings: Vec<_> = findings
        .iter()
        .filter(|f| f.status == FindingStatus::Current)
        .collect();
    let now = chrono::Utc::now();
    let held: Vec<_> = problems
        .iter()
        .filter(|p| {
            p.assignee.is_some()
                && matches!(p.status, ProblemStatus::Open | ProblemStatus::InProgress)
        })
        .collect();

    // Reference prefixes computed across EVERY id this briefing mentions, not
    // per section. UUID7 ids are time-ordered, so entities created in the same
    // second share their leading characters: a fixed-width prefix printed
    // `01a144` for all five sections, and an agent cannot paste that anywhere.
    let all_ids: Vec<&str> = problems
        .iter()
        .map(|p| p.id.as_str())
        .chain(solutions.iter().map(|s| s.id.as_str()))
        .chain(critiques.iter().map(|c| c.id.as_str()))
        .chain(findings.iter().map(|f| f.id.as_str()))
        .collect();
    let prefixes: std::collections::HashMap<String, String> =
        crate::display::truncated_prefixes(&all_ids)
            .into_iter()
            .collect();
    let r = |id: &str| prefixes.get(id).cloned().unwrap_or_else(|| id.to_string());

    if json {
        let out = serde_json::json!({
            "actor": actor,
            "escalations": escalations.iter().take(limit).collect::<Vec<_>>(),
            "next": next.first(),
            "counts": {
                "escalations": escalations.len(),
                "problems_open": problems.iter().filter(|p| p.is_open()).count(),
                "awaiting_review": submitted.len(),
                "proposed": proposed.len(),
                "open_critiques": open_critiques.len(),
                "findings_current": current_findings.len(),
                "claims_held": held.len(),
            },
            "awaiting_review": submitted.iter().take(limit)
                .map(|s| serde_json::json!({"id": s.id, "title": s.title, "problem_id": s.problem_id}))
                .collect::<Vec<_>>(),
            "proposed": proposed.iter().take(limit)
                .map(|s| serde_json::json!({"id": s.id, "title": s.title, "problem_id": s.problem_id,
                                            "who": s.assignee}))
                .collect::<Vec<_>>(),
            "open_critiques": open_critiques.iter().take(limit)
                .map(|c| serde_json::json!({"id": c.id, "title": c.title, "solution_id": c.solution_id,
                                            "severity": c.severity.to_string()}))
                .collect::<Vec<_>>(),
            "findings": current_findings.iter().take(limit)
                .map(|f| serde_json::json!({"id": f.id, "title": f.title, "problem_id": f.problem_id}))
                .collect::<Vec<_>>(),
            "claims_held": held.iter().take(limit)
                .map(|p| serde_json::json!({
                    "id": p.id, "title": p.title, "assignee": p.assignee,
                    "age_minutes": p.claimed_at.map(|t| now.signed_duration_since(t).num_minutes()),
                }))
                .collect::<Vec<_>>(),
        });
        println!("{}", serde_json::to_string_pretty(&out)?);
        return Ok(());
    }

    println!("# jjj — you are @{actor}\n");

    // Escalations first, for the same reason they lead `jjj status`: a fleet
    // blocked on a person must not have that fact ranked below its work queue.
    if !escalations.is_empty() {
        println!(
            "## !! {} open escalation{} — a person is needed",
            escalations.len(),
            if escalations.len() == 1 { "" } else { "s" }
        );
        for e in escalations.iter().take(limit) {
            let age = e.age(now);
            let waited = if age.num_hours() >= 1 {
                format!("{}h", age.num_hours())
            } else {
                format!("{}m", age.num_minutes().max(0))
            };
            println!("- [{waited} ago, {}] {}", e.by, e.reason);
            // A person's reply is the most useful line in the whole briefing
            // when it exists: it is the difference between re-discovering a wall
            // and knowing what was found about it.
            for a in &e.answers {
                println!("  ↳ {} answered: {}", a.by, a.message);
            }
        }
        // Capped like every other section: a briefing whose length tracks the
        // corpus is useless to the agent it is written for, and 132 escalations
        // printed in full is what that looks like. The heading still carries
        // the true total, and `--full` still shows every one.
        if escalations.len() > limit {
            println!("- … and {} more", escalations.len() - limit);
        }
        println!("\nDo not start new work that depends on this being fixed.\n");
    }

    match next.first() {
        Some(item) => {
            println!("## Next for you");
            println!(
                "- [{}] {} — {}",
                item["category"].as_str().unwrap_or("?").to_uppercase(),
                item["title"].as_str().unwrap_or("?"),
                item["summary"].as_str().unwrap_or("")
            );
            println!("  `{}`", item["entity_id"].as_str().unwrap_or(""));
            if let Some(cmd) = item["suggested_command"].as_str() {
                if !cmd.is_empty() {
                    println!("  → {cmd}");
                }
            }
            println!();
        }
        None => println!("## Next for you\n- nothing queued\n"),
    }

    section(
        "Already proposed — read before starting, so you do not build the same idea",
        proposed.len(),
        limit,
        proposed.iter().map(|s| {
            format!(
                "{}  @{}: {}",
                r(&s.id),
                s.assignee
                    .as_deref()
                    .or_else(|| author_of.get(s.id.as_str()).copied())
                    .unwrap_or("?"),
                s.title
            )
        }),
    );

    section(
        "Awaiting review",
        submitted.len(),
        limit,
        submitted
            .iter()
            .map(|s| format!("{}  {}", r(&s.id), s.title)),
    );

    section(
        "Open critiques — unanswered objections block approval",
        open_critiques.len(),
        limit,
        open_critiques
            .iter()
            .map(|c| format!("{}  [{}] {}", r(&c.id), c.severity, c.title)),
    );

    // Evidence, because the failure this command exists for is agents
    // re-deriving what a previous agent already measured.
    section(
        "Already measured — cite these rather than measuring again",
        current_findings.len(),
        limit,
        current_findings
            .iter()
            .map(|f| format!("{}  {}", r(&f.id), f.title)),
    );

    section(
        "Claimed right now — a live claim is someone else's work",
        held.len(),
        limit,
        held.iter().map(|p| {
            let age = p
                .claimed_at
                .map(|t| format!(" ({}m ago)", now.signed_duration_since(t).num_minutes()))
                .unwrap_or_else(|| " (assigned, no expiry)".to_string());
            format!(
                "{}  @{}{}: {}",
                r(&p.id),
                p.assignee.as_deref().unwrap_or("?"),
                age,
                p.title
            )
        }),
    );

    if !full {
        println!("_`jjj prime --full` for every item; `--json` to parse._");
    }
    Ok(())
}

/// Print one section, or nothing when it is empty.
///
/// An empty heading is noise in a briefing whose whole purpose is to be cheaper
/// than the questions it answers.
fn section(heading: &str, total: usize, limit: usize, items: impl Iterator<Item = String>) {
    if total == 0 {
        return;
    }
    println!("## {heading} ({total})");
    let mut shown = 0;
    for item in items.take(limit) {
        println!("- {item}");
        shown += 1;
    }
    if total > shown {
        println!("- … and {} more", total - shown);
    }
    println!();
}
