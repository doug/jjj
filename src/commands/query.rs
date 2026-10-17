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
    /// The unfinished sub-problems holding this one back, for `waiting`.
    ///
    /// `waiting` answered half its question: it named every parent held back
    /// and nothing about what was holding it. The next step is always to go
    /// work the child, so leaving it out cost a `problem show` per row to
    /// recover a set the predicate had already computed to decide the row
    /// belonged in the listing at all.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub blocked_by: Vec<Blocker>,
}

/// A sub-problem keeping its parent off the ready front.
#[derive(Debug, serde::Serialize)]
pub struct Blocker {
    pub id: String,
    pub title: String,
    pub status: String,
}

pub fn execute(
    ctx: &CommandContext,
    expr: String,
    json: bool,
    count_by: Option<String>,
) -> Result<()> {
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

    // `waiting` is a problem-only predicate and needs the whole set to evaluate,
    // so hold onto it rather than consuming the iterator.
    let all_problems = store.list_problems()?;
    if q.wants_kind("problem") {
        for p in all_problems.iter().cloned() {
            if q.waiting && !p.is_waiting_on_children(&all_problems) {
                continue;
            }
            if !wanted_problems.is_empty() && !wanted_problems.contains(&p.id) {
                continue;
            }
            // Only for `waiting`: every other listing would pay a scan per row
            // to attach a field nobody asked about.
            let blocked_by = if q.waiting {
                p.open_children(&all_problems)
                    .into_iter()
                    .map(|c| Blocker {
                        id: c.id.clone(),
                        title: c.title.clone(),
                        status: c.status.to_string(),
                    })
                    .collect()
            } else {
                Vec::new()
            };
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
                blocked_by,
            });
        }
    }
    if q.wants_kind("solution") && !q.waiting {
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
                blocked_by: Vec::new(),
            });
        }
    }
    if q.wants_kind("critique") && !q.waiting {
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
                blocked_by: Vec::new(),
            });
        }
    }
    if q.wants_kind("finding") && !q.waiting {
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
                blocked_by: Vec::new(),
            });
        }
    }
    if q.wants_kind("milestone") && !q.waiting {
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
                blocked_by: Vec::new(),
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

    // A grouped count reports the whole match set, so it runs before `limit`
    // and ignores it. A capped count would be a different number wearing the
    // same name — and idea 32 exists because exactly that kind of silent
    // shortfall is unreadable from the output.
    if let Some(field) = count_by {
        return print_counts(&rows, &field, json);
    }

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

    // Say so when a limit hid matches — on both paths.
    //
    // The text path has always printed "Showing 10 of 47 matches"; the JSON
    // path returned here, before it. So a person could see the set was capped
    // and an agent could not: `limit:10` returning ten rows is
    // indistinguishable from a repository that holds exactly ten matches, and
    // "no other problem is open" is the kind of wrong conclusion that ends a
    // search early.
    //
    // It goes to stderr rather than into the payload because stdout is a bare
    // array of rows, pinned by tests and taught by SKILL.md. Wrapping it in an
    // envelope to carry one boolean would break every caller parsing it today
    // — and beads, which added exactly that envelope late, had to hide it
    // behind an opt-in env var for the same reason.
    let elided = total.saturating_sub(rows.len());
    if json {
        println!("{}", serde_json::to_string_pretty(&rows)?);
        if elided > 0 {
            eprintln!(
                "note: showing {} of {total} matches — {elided} hidden by the limit",
                rows.len()
            );
        }
        return Ok(());
    }

    if rows.is_empty() {
        println!("No matches.");
        return Ok(());
    }

    // Blocker ids go into the same prefix computation as the rows. A prefix is
    // only a usable reference if it is unique across everything printed, and
    // these are printed.
    let ids: Vec<&str> = rows
        .iter()
        .map(|r| r.id.as_str())
        .chain(
            rows.iter()
                .flat_map(|r| r.blocked_by.iter().map(|b| b.id.as_str())),
        )
        .collect();
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
        // Indented under the parent, because the blocker is the thing to go
        // work next and reading the listing should not require a second call
        // to learn its id.
        for b in &r.blocked_by {
            let short = prefixes.get(&b.id).cloned().unwrap_or_else(|| b.id.clone());
            println!(
                "{:<ref_w$}   ← {} {} [{}]",
                "",
                format_with_type_prefix("problem", &short),
                crate::utils::truncate(&b.title, 36),
                b.status
            );
        }
    }
    if elided > 0 {
        println!("\nShowing {} of {} matches (limit).", rows.len(), total);
    }
    Ok(())
}

/// Count rows grouped by one field.
///
/// `insights` reports fixed aggregates and the per-kind listings report rows, so
/// "how many open problems per assignee" meant one call per assignee and
/// counting the output — N calls to learn a shape that is one pass over the
/// same data.
///
/// Ordering is by count descending, then by key, so the answer is stable across
/// runs and across clones: a caller diffing two counts should see a real change
/// rather than a reshuffle.
///
/// `tag` is the one key whose totals can exceed the number of matches, because
/// an entity carries several tags and is counted under each. That is reported
/// when it is actually true — by comparing the sum to the match count, not by
/// assuming it of every tag query — so the caption never contradicts the
/// numbers printed above it.
fn print_counts(rows: &[Row], field: &str, json: bool) -> Result<()> {
    let mut counts: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    for r in rows {
        match field {
            "status" => *counts.entry(r.status.clone()).or_default() += 1,
            "type" | "kind" => *counts.entry(r.kind.clone()).or_default() += 1,
            "who" | "assignee" | "author" => {
                let who = r.who.clone().unwrap_or_else(|| "(unassigned)".to_string());
                *counts.entry(who).or_default() += 1;
            }
            "tag" | "tags" => {
                if r.tags.is_empty() {
                    *counts.entry("(untagged)".to_string()).or_default() += 1;
                }
                for t in &r.tags {
                    *counts.entry(t.clone()).or_default() += 1;
                }
            }
            other => {
                return Err(JjjError::Validation(format!(
                    "cannot count by {other:?}. Try status, type, who, or tag."
                )));
            }
        }
    }

    // Whether the groups overlap is arithmetic, not a property of the field
    // name. Setting it for `tag` unconditionally announced that counts "sum
    // higher" over a set where every entity was untagged and they summed
    // exactly — a caption contradicting the numbers beside it.
    let multi = counts.values().sum::<usize>() > rows.len();

    let mut pairs: Vec<(&String, &usize)> = counts.iter().collect();
    pairs.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));

    if json {
        let payload = serde_json::json!({
            "field": field,
            "matches": rows.len(),
            // True when one entity can land in several groups, so a caller
            // knows the group totals are not expected to sum to `matches`.
            "overlapping": multi,
            "counts": pairs
                .iter()
                .map(|(k, v)| serde_json::json!({ "key": k, "count": v }))
                .collect::<Vec<_>>(),
        });
        println!("{}", serde_json::to_string_pretty(&payload)?);
        return Ok(());
    }

    if pairs.is_empty() {
        println!("No matches.");
        return Ok(());
    }
    let key_w = pairs.iter().map(|(k, _)| k.len()).max().unwrap_or(5).max(5);
    for (k, v) in &pairs {
        println!("{:<key_w$}  {}", k, v);
    }
    let n = rows.len();
    let noun = if n == 1 { "match" } else { "matches" };
    if multi {
        println!("\n{n} {noun}; an entity is counted under each of its tags, so these sum higher.");
    } else {
        println!("\n{n} {noun}.");
    }
    Ok(())
}
