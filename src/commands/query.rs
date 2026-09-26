//! `jjj query` — one grammar across every entity kind.
//!
//! See [`crate::query`] for the grammar and why it exists. This module only
//! applies it: load each requested kind, filter, sort, truncate, print.
//!
//! Deliberately a *reader*. It does not claim/assign/approve anything, so a
//! mistyped query can only ever return the wrong list.

use crate::context::CommandContext;
use crate::display::{format_with_type_prefix, truncated_prefixes};
use crate::error::{JjjError, Result};
use crate::query::{Query, SortKey};

/// One result row, flattened so five different entity types can share a listing
/// and a JSON shape.
#[derive(Debug, serde::Serialize)]
pub struct Row {
    pub kind: String,
    pub id: String,
    pub title: String,
    pub status: String,
    /// Whoever the kind attributes it to — author for critiques and findings,
    /// assignee for problems and solutions. Collapsed into one column because a
    /// cross-kind listing has room for one person, and "who is behind this" is
    /// the question either field answers.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub who: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    /// Causal edit clock. `sort:edit` uses this rather than `updated_at`, so the
    /// order does not depend on whose machine had the fastest clock.
    pub lamport: u64,
    /// The problem this hangs off, where the kind has one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub problem_id: Option<String>,
}

pub fn execute(ctx: &CommandContext, expr: String, json: bool) -> Result<()> {
    let q = Query::parse(&expr).map_err(JjjError::Validation)?;
    let store = &ctx.store;

    // `mine` resolves here, not in the parser: the query layer has no business
    // reading identity, and it keeps the grammar testable without a repo.
    let me = if q.mine {
        Some(store.get_current_user()?.to_ascii_lowercase())
    } else {
        None
    };

    // Resolve `problem:<ref>` through the normal reference resolver, so a prefix
    // or fuzzy title works here exactly as it does everywhere else.
    let mut wanted_problems = std::collections::BTreeSet::new();
    for p in &q.problems {
        wanted_problems.insert(ctx.resolve_problem(p)?);
    }

    let mut rows: Vec<Row> = Vec::new();

    if q.wants_kind("problem") {
        for p in store.list_problems()? {
            if !wanted_problems.is_empty() && !wanted_problems.contains(&p.id) {
                continue;
            }
            rows.push(Row {
                kind: "problem".into(),
                id: p.id,
                title: p.title,
                status: p.status.to_string(),
                who: p.assignee,
                tags: p.tags,
                created_at: p.created_at,
                updated_at: p.updated_at,
                lamport: p.lamport,
                problem_id: p.parent_id,
            });
        }
    }
    if q.wants_kind("solution") {
        for s in store.list_solutions()? {
            if !wanted_problems.is_empty() && !wanted_problems.contains(&s.problem_id) {
                continue;
            }
            rows.push(Row {
                kind: "solution".into(),
                id: s.id,
                title: s.title,
                status: s.status.to_string(),
                who: s.assignee,
                tags: s.tags,
                created_at: s.created_at,
                updated_at: s.updated_at,
                lamport: s.lamport,
                problem_id: Some(s.problem_id),
            });
        }
    }
    if q.wants_kind("critique") {
        // A critique's problem is its solution's, so map once rather than
        // loading the solution per critique.
        let problem_of: std::collections::HashMap<String, String> = store
            .list_solutions()?
            .into_iter()
            .map(|s| (s.id, s.problem_id))
            .collect();
        for c in store.list_critiques()? {
            let pid = problem_of.get(&c.solution_id).cloned();
            if !wanted_problems.is_empty()
                && !pid.as_ref().is_some_and(|p| wanted_problems.contains(p))
            {
                continue;
            }
            rows.push(Row {
                kind: "critique".into(),
                id: c.id,
                title: c.title,
                status: c.status.to_string(),
                who: c.author.or(c.reviewer),
                tags: Vec::new(),
                created_at: c.created_at,
                updated_at: c.updated_at,
                lamport: c.lamport,
                problem_id: pid,
            });
        }
    }
    if q.wants_kind("finding") {
        for f in store.list_findings()? {
            if !wanted_problems.is_empty() && !wanted_problems.contains(&f.problem_id) {
                continue;
            }
            rows.push(Row {
                kind: "finding".into(),
                id: f.id,
                title: f.title,
                status: f.status.to_string(),
                who: f.author,
                tags: f.tags,
                created_at: f.created_at,
                updated_at: f.updated_at,
                lamport: f.lamport,
                problem_id: Some(f.problem_id),
            });
        }
    }
    if q.wants_kind("milestone") {
        // Milestones had no filters at all before this; they get the same
        // grammar as everything else for free.
        for m in store.list_milestones()? {
            if !wanted_problems.is_empty() {
                continue; // a milestone does not belong to a problem
            }
            rows.push(Row {
                kind: "milestone".into(),
                id: m.id,
                title: m.title,
                status: m.status.to_string(),
                who: m.assignee,
                tags: Vec::new(),
                created_at: m.created_at,
                updated_at: m.updated_at,
                lamport: m.lamport,
                problem_id: None,
            });
        }
    }

    rows.retain(|r| {
        q.wants_status(&r.status)
            && q.wants_tags(&r.tags)
            && q.wants_title(&r.title)
            && match &me {
                // `mine` matches whichever person field the kind carries, which
                // is the same collapse the `who` column makes.
                Some(actor) => r.who.as_deref().is_some_and(|w| {
                    w.trim_start_matches('@')
                        .to_ascii_lowercase()
                        .contains(actor)
                }),
                None => true,
            }
            && Query::wants_person(&q.authors, r.who.as_deref())
            && Query::wants_person(&q.assignees, r.who.as_deref())
    });

    match q.sort {
        SortKey::Edit => rows.sort_by(|a, b| {
            // Clock first, then creation, so entities that predate clocks (all
            // zero) still come out in a stable, meaningful order instead of
            // whatever the load happened to produce.
            b.lamport
                .cmp(&a.lamport)
                .then_with(|| b.created_at.cmp(&a.created_at))
        }),
        SortKey::Created => rows.sort_by_key(|r| std::cmp::Reverse(r.created_at)),
        SortKey::Updated => rows.sort_by_key(|r| std::cmp::Reverse(r.updated_at)),
        SortKey::Title => rows.sort_by_key(|r| r.title.to_lowercase()),
    }
    if q.reverse {
        rows.reverse();
    }
    let total = rows.len();
    if let Some(limit) = q.limit {
        rows.truncate(limit);
    }

    if json {
        println!("{}", serde_json::to_string_pretty(&rows)?);
        return Ok(());
    }

    if rows.is_empty() {
        println!("No matches.");
        return Ok(());
    }

    let ids: Vec<&str> = rows.iter().map(|r| r.id.as_str()).collect();
    let prefixes: std::collections::HashMap<String, String> =
        truncated_prefixes(&ids).into_iter().collect();

    // Size the reference column from the data. Prefixes auto-extend for
    // uniqueness, so entities created in the same second need far more than a
    // fixed width — and a fixed width silently misaligns every later column.
    let refs: Vec<String> = rows
        .iter()
        .map(|r| {
            let short = prefixes.get(&r.id).cloned().unwrap_or_else(|| r.id.clone());
            format_with_type_prefix(&r.kind, &short)
        })
        .collect();
    let ref_w = refs.iter().map(String::len).max().unwrap_or(4).max(4);

    println!("{:<ref_w$} {:<14} {:<10} TITLE", "REF", "STATUS", "WHO");
    println!("{}", "-".repeat(ref_w + 68));
    for (r, reference) in rows.iter().zip(refs.iter()) {
        println!(
            "{:<ref_w$} {:<14} {:<10} {}",
            reference,
            crate::utils::truncate(&r.status, 14),
            crate::utils::truncate(r.who.as_deref().unwrap_or("-"), 10),
            crate::utils::truncate(&r.title, 40)
        );
    }
    if total > rows.len() {
        println!("\nShowing {} of {} matches (limit).", rows.len(), total);
    }
    Ok(())
}
