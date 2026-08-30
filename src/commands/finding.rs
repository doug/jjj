//! `jjj finding` — record and read evidence.
//!
//! See [`crate::models::Finding`] for why this type exists. In short: jjj
//! modelled conjectures and refutations but not the observations that motivate
//! them, so investigations were filed as solutions and then withdrawn as
//! "documented, not fixed".

use crate::cli::FindingAction;
use crate::context::CommandContext;
use crate::db::{search, Database};
use crate::display::{short_id, truncated_prefixes};
use crate::error::{JjjError, Result};
use crate::models::{Event, EventExtra, EventType, Finding, FindingStatus};

pub fn execute(ctx: &CommandContext, action: FindingAction) -> Result<()> {
    match action {
        FindingAction::New {
            problem_id,
            title,
            body,
            method,
            refs,
            tags,
            json,
        } => new_finding(
            ctx,
            problem_id,
            title,
            super::read_body(body)?,
            method,
            refs,
            tags,
            json,
        ),
        FindingAction::List {
            problem,
            status,
            author,
            mine,
            search,
            about,
            json,
        } => list_findings(
            ctx,
            problem,
            status,
            author,
            mine,
            search.as_deref(),
            about,
            json,
        ),
        FindingAction::Show { finding_id, json } => show_finding(ctx, finding_id, json),
        FindingAction::Edit {
            finding_id,
            title,
            method,
            body,
        } => edit_finding(ctx, finding_id, title, method, body),
        FindingAction::Supersede {
            finding_id,
            by,
            json,
        } => supersede_finding(ctx, finding_id, by, json),
        FindingAction::Delete { finding_id, force } => delete_finding(ctx, finding_id, force),
    }
}

/// Resolve each `--ref` against every entity kind.
///
/// A finding routinely bears on several kinds at once — it explains a solution,
/// answers a critique, builds on an earlier finding — so refs are untyped. That
/// means resolution has to try each kind rather than being told which one.
/// Ambiguity across kinds is vanishingly unlikely with UUID prefixes, and the
/// first match wins in a fixed order so the behaviour is at least deterministic.
fn resolve_refs(ctx: &CommandContext, refs: &[String]) -> Result<Vec<String>> {
    let mut out = Vec::with_capacity(refs.len());
    for r in refs {
        let resolved = ctx
            .resolve_solution(r)
            .or_else(|_| ctx.resolve_critique(r))
            .or_else(|_| ctx.resolve_finding(r))
            .or_else(|_| ctx.resolve_problem(r))
            .map_err(|_| {
                JjjError::Validation(format!(
                    "--ref '{r}' matched no solution, critique, finding or problem"
                ))
            })?;
        if !out.contains(&resolved) {
            out.push(resolved);
        }
    }
    Ok(out)
}

/// Resolve `--cites` arguments to finding IDs.
///
/// Shared with `solution new` and `critique new`, which is the whole point of
/// the flag: a citation that cannot be resolved to a real finding is a typo, and
/// failing loudly here is better than storing a dangling id nothing will notice.
pub(crate) fn resolve_cites(ctx: &CommandContext, cites: &[String]) -> Result<Vec<String>> {
    let mut out = Vec::with_capacity(cites.len());
    for c in cites {
        let id = ctx.resolve_finding(c)?;
        if !out.contains(&id) {
            out.push(id);
        }
    }
    Ok(out)
}

#[allow(clippy::too_many_arguments)]
fn new_finding(
    ctx: &CommandContext,
    problem_input: String,
    title: String,
    evidence: String,
    method: Option<String>,
    refs: Vec<String>,
    tags: Vec<String>,
    json: bool,
) -> Result<()> {
    let problem_id = ctx.resolve_problem(&problem_input)?;
    let store = &ctx.store;

    // Validate the problem exists before writing anything.
    let problem = store.load_problem(&problem_id)?;
    let resolved_refs = resolve_refs(ctx, &refs)?;
    let user = store.get_current_user()?;

    let finding_id_cell = std::cell::RefCell::new(String::new());
    store.with_metadata(
        &format!("Record finding on {}: {}", problem_id, title),
        || {
            let finding_id = store.next_finding_id()?;
            let mut finding = Finding::new(finding_id.clone(), title.clone(), problem_id.clone());
            finding.evidence = evidence.clone();
            finding.method = method.clone();
            finding.refs = resolved_refs.clone();
            finding.tags = tags.clone();
            finding.author = Some(user.clone());

            let extra = EventExtra {
                problem: Some(problem_id.clone()),
                title: Some(title.clone()),
                ..Default::default()
            };
            let event = Event::new(EventType::FindingRecorded, finding_id.clone(), user.clone())
                .with_extra(extra)
                .with_refs(resolved_refs.clone());
            store.set_pending_event(event);

            store.save_finding(&finding)?;

            if json {
                println!("{}", serde_json::to_string_pretty(&finding)?);
            } else {
                println!(
                    "Recorded finding {} on problem {} ({})",
                    finding.id,
                    short_id(&problem_id),
                    problem.title
                );
                if let Some(ref m) = finding.method {
                    println!("  Method: {}", m);
                }
                if !finding.refs.is_empty() {
                    println!(
                        "  Bears on: {}",
                        finding
                            .refs
                            .iter()
                            .map(|r| short_id(r))
                            .collect::<Vec<_>>()
                            .join(", ")
                    );
                }
            }

            *finding_id_cell.borrow_mut() = finding_id;
            Ok(())
        },
    )?;

    let fid = finding_id_cell.into_inner();
    if !fid.is_empty() {
        let event = Event::new(EventType::FindingRecorded, fid.clone(), user);
        crate::automation::run(&ctx.store, &event, &fid);
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn list_findings(
    ctx: &CommandContext,
    problem_filter: Option<String>,
    status_filter: Option<String>,
    author_filter: Option<String>,
    mine: bool,
    search_query: Option<&str>,
    about: Option<String>,
    json: bool,
) -> Result<()> {
    let store = &ctx.store;
    let mut findings = store.list_findings()?;

    if let Some(ref problem_input) = problem_filter {
        let problem_id = ctx.resolve_problem(problem_input)?;
        findings.retain(|f| f.problem_id == problem_id);
    }

    if let Some(status_str) = status_filter {
        let status: FindingStatus = status_str.parse().map_err(JjjError::Validation)?;
        findings.retain(|f| f.status == status);
    }

    let author_pattern = if mine {
        Some(store.get_current_user()?)
    } else {
        author_filter
    };
    if let Some(ref pattern) = author_pattern {
        let pat = pattern.trim_start_matches('@').to_lowercase();
        findings.retain(|f| {
            f.author
                .as_deref()
                .map(|a| a.to_lowercase().contains(&pat))
                .unwrap_or(false)
        });
    }

    // `--about` takes any entity reference, so it resolves the same untyped way
    // `--ref` does on creation.
    if let Some(ref about_input) = about {
        let ids = resolve_refs(ctx, std::slice::from_ref(about_input))?;
        findings.retain(|f| ids.iter().any(|id| f.refs.contains(id)));
    }

    if let Some(query) = search_query {
        let db_path = ctx.jj().repo_root().join(".jj").join("jjj.db");
        let db = Database::open(&db_path)?;
        crate::db::load_from_markdown_incremental(&db, &ctx.store)?;
        let results = search::search(db.conn(), query, Some("finding"))?;
        let matching: std::collections::HashSet<_> =
            results.iter().map(|r| r.entity_id.as_str()).collect();
        findings.retain(|f| matching.contains(f.id.as_str()));
    }

    if json {
        println!("{}", serde_json::to_string_pretty(&findings)?);
        return Ok(());
    }

    if findings.is_empty() {
        println!("No findings found.");
        return Ok(());
    }

    let finding_uuids: Vec<&str> = findings.iter().map(|f| f.id.as_str()).collect();
    let finding_prefixes = truncated_prefixes(&finding_uuids);
    let problem_uuids: Vec<&str> = findings.iter().map(|f| f.problem_id.as_str()).collect();
    let problem_prefixes = truncated_prefixes(&problem_uuids);

    println!(
        "{:<10} {:<13} {:<10} {:<8} TITLE",
        "ID", "STATUS", "PROBLEM", "AUTHOR"
    );
    println!("{}", "-".repeat(80));

    for ((finding, (_, fp)), (_, pp)) in findings
        .iter()
        .zip(finding_prefixes.iter())
        .zip(problem_prefixes.iter())
    {
        // "=" for a measurement that stands, "~" for one that has been
        // corrected — deliberately not a check/cross, which would read as
        // approval.
        let icon = match finding.status {
            FindingStatus::Current => "=",
            FindingStatus::Superseded => "~",
        };
        println!(
            "{:<10} {}{:<12} {:<10} {:<8} {}",
            fp,
            icon,
            finding.status,
            pp,
            crate::utils::truncate(finding.author.as_deref().unwrap_or("-"), 8),
            finding.title
        );
    }

    Ok(())
}

fn show_finding(ctx: &CommandContext, finding_input: String, json: bool) -> Result<()> {
    let finding_id = ctx.resolve_finding(&finding_input)?;
    let store = &ctx.store;
    let finding = store.load_finding(&finding_id)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&finding)?);
        return Ok(());
    }

    println!("Finding {}", finding.id);
    println!("  Title:    {}", finding.title);
    println!("  Status:   {}", finding.status);
    if let Some(ref by) = finding.superseded_by {
        // Resolving the title here is what makes a superseded finding useful
        // rather than a dead end: the reader needs to know what replaced it.
        match store.load_finding(by) {
            Ok(replacement) => {
                println!("  Replaced by: {} — {}", short_id(by), replacement.title)
            }
            Err(_) => println!("  Replaced by: {} (missing)", short_id(by)),
        }
    }
    match store.load_problem(&finding.problem_id) {
        Ok(p) => println!(
            "  Problem:  {} — {}",
            short_id(&finding.problem_id),
            p.title
        ),
        Err(_) => println!("  Problem:  {} (missing)", short_id(&finding.problem_id)),
    }
    if let Some(ref author) = finding.author {
        println!("  Author:   {}", author);
    }
    println!(
        "  Recorded: {}",
        finding.created_at.format("%Y-%m-%d %H:%M UTC")
    );
    if !finding.tags.is_empty() {
        println!("  Tags:     {}", finding.tags.join(", "));
    }
    if !finding.refs.is_empty() {
        println!("  Bears on:");
        for r in &finding.refs {
            println!("    {}", short_id(r));
        }
    }
    if let Some(ref method) = finding.method {
        println!("\nMethod:\n  {}", method);
    }
    if !finding.evidence.trim().is_empty() {
        println!("\nEvidence:\n{}", finding.evidence);
    }

    Ok(())
}

fn edit_finding(
    ctx: &CommandContext,
    finding_input: String,
    title: Option<String>,
    method: Option<String>,
    body: Option<String>,
) -> Result<()> {
    let finding_id = ctx.resolve_finding(&finding_input)?;
    let store = &ctx.store;
    let evidence = body.map(|b| super::read_body(Some(b))).transpose()?;

    if title.is_none() && method.is_none() && evidence.is_none() {
        return Err(JjjError::Validation(
            "nothing to change — pass --title, --method or --body".to_string(),
        ));
    }

    store.with_metadata(&format!("Edit finding {}", finding_id), || {
        let mut finding = store.load_finding(&finding_id)?;
        if let Some(ref t) = title {
            finding.title = t.clone();
        }
        if let Some(ref m) = method {
            finding.method = Some(m.clone());
        }
        if let Some(ref e) = evidence {
            finding.evidence = e.clone();
        }
        finding.updated_at = chrono::Utc::now();
        store.save_finding(&finding)?;
        println!("Updated finding {}", short_id(&finding_id));
        Ok(())
    })
}

fn supersede_finding(
    ctx: &CommandContext,
    finding_input: String,
    by_input: String,
    json: bool,
) -> Result<()> {
    let finding_id = ctx.resolve_finding(&finding_input)?;
    let by_id = ctx.resolve_finding(&by_input)?;
    let store = &ctx.store;

    // Load the replacement first: superseding by a finding that does not exist
    // would leave a dangling pointer that `finding show` reports as "(missing)"
    // forever.
    let replacement = store.load_finding(&by_id)?;
    let user = store.get_current_user()?;

    store.with_metadata(
        &format!("Supersede finding {} with {}", finding_id, by_id),
        || {
            let mut finding = store.load_finding(&finding_id)?;
            finding
                .supersede(by_id.clone())
                .map_err(JjjError::Validation)?;

            let event = Event::new(
                EventType::FindingSuperseded,
                finding_id.clone(),
                user.clone(),
            )
            .with_refs(vec![by_id.clone()]);
            store.set_pending_event(event);

            store.save_finding(&finding)?;

            if json {
                println!("{}", serde_json::to_string_pretty(&finding)?);
            } else {
                println!(
                    "Finding {} superseded by {} — {}",
                    short_id(&finding_id),
                    short_id(&by_id),
                    replacement.title
                );
            }
            Ok(())
        },
    )
}

fn delete_finding(ctx: &CommandContext, finding_input: String, force: bool) -> Result<()> {
    let finding_id = ctx.resolve_finding(&finding_input)?;
    let store = &ctx.store;
    let finding = store.load_finding(&finding_id)?;

    if !force {
        // Superseding is almost always what someone wants: it keeps the record
        // of what was once believed, which is the reason the same investigation
        // does not get run a third time.
        eprintln!(
            "About to delete finding {} — \"{}\"",
            short_id(&finding_id),
            finding.title
        );
        eprintln!("If it was merely corrected, `jjj finding supersede` keeps the record instead.");
        eprintln!("Re-run with --force to delete.");
        return Err(JjjError::Validation("delete not confirmed".to_string()));
    }

    store.with_metadata(&format!("Delete finding {}", finding_id), || {
        store.delete_finding(&finding_id)?;
        println!("Deleted finding {}", short_id(&finding_id));
        Ok(())
    })
}
