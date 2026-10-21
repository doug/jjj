//! `jjj plan apply` — create a whole graph of work in one call.
//!
//! An agent that has thought through a problem usually has the shape of several
//! entities at once: a problem, two sub-problems, the evidence it already
//! measured, a conjecture for each. Creating those one command at a time costs a
//! process per entity, makes a metadata commit per entity, and — worst — gives
//! up halfway if the fourth call fails, leaving a half-built graph with no
//! record of what was intended.
//!
//! Borrowed from beads' `bd create --graph`, whose good idea is **symbolic
//! keys**: a node names its parent by a key local to the plan rather than by an
//! id that does not exist yet, so a parent and its children can be described in
//! one document and wired up as they are created. The result maps each key to
//! the id it became, which is what the caller needs in order to say anything
//! else about the work.
//!
//! # What "atomic" means here, exactly
//!
//! The whole plan is **validated before anything is written**: every reference
//! resolved, every enum parsed, duplicate keys and parent cycles rejected. Then
//! the creates run inside a single `with_metadata`, so they land as one metadata
//! commit rather than N.
//!
//! That is not a transaction. `with_metadata` skips its commit on error but does
//! not roll back files already written, so an I/O failure part-way through would
//! leave entities on disk uncommitted. Validating first is what makes that
//! window hold only genuine I/O faults rather than every typo in the plan —
//! which is the difference that matters in practice, and `--dry-run` reports the
//! outcome without writing at all.

use std::collections::{HashMap, HashSet};

use serde::Deserialize;

use crate::context::CommandContext;
use crate::error::{JjjError, Result};
use crate::models::{Critique, CritiqueSeverity, Finding, Milestone, Priority, Problem, Solution};

/// A plan is five optional lists. Unknown fields are refused rather than
/// ignored: a plan is written by an agent that cannot see a warning, and
/// silently dropping a misspelled `prioirty` would create the work with the
/// wrong priority and report success.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Plan {
    #[serde(default)]
    milestones: Vec<MilestoneNode>,
    #[serde(default)]
    problems: Vec<ProblemNode>,
    #[serde(default)]
    findings: Vec<FindingNode>,
    #[serde(default)]
    solutions: Vec<SolutionNode>,
    #[serde(default)]
    critiques: Vec<CritiqueNode>,
    /// Field changes to entities that already exist (or were created above).
    #[serde(default)]
    updates: Vec<UpdateNode>,
    /// Lifecycle transitions. Separate from `updates` on purpose — see
    /// [`ActionNode`].
    #[serde(default)]
    actions: Vec<ActionNode>,
}

/// A field change: annotations only, never a status.
///
/// Status is deliberately absent. Every transition has a guard and emits an
/// event — solving needs an approved solution, approving is refused while a
/// critique is open — and a bulk path that set `status` directly would bypass
/// all of it. beads has exactly that hole and documents it: its batch `close`
/// "does not apply that policy at all", unlike its own `bd close`. Lifecycle
/// therefore goes through `actions`, which calls the same functions the
/// single-entity commands call.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct UpdateNode {
    /// problem | solution | critique | finding | milestone
    kind: String,
    /// A plan key, or an existing entity (id, prefix or fuzzy title).
    on: String,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    body: Option<String>,
    #[serde(default)]
    priority: Option<String>,
    #[serde(default)]
    tags: Option<Vec<String>>,
    #[serde(default)]
    assignee: Option<String>,
    /// Refuse this one change if the entity moved since it was read.
    #[serde(default)]
    if_lamport: Option<u64>,
}

/// A lifecycle transition, run through the shared domain layer so that guards
/// and events behave exactly as they do for the single-entity commands.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, tag = "action", rename_all = "snake_case")]
enum ActionNode {
    Solve {
        on: String,
    },
    Dissolve {
        on: String,
        #[serde(default)]
        reason: Option<String>,
    },
    Reopen {
        on: String,
    },
    Submit {
        on: String,
    },
    Withdraw {
        on: String,
        #[serde(default)]
        rationale: Option<String>,
    },
    Address {
        on: String,
    },
    Validate {
        on: String,
    },
    Dismiss {
        on: String,
    },
}

impl ActionNode {
    fn target(&self) -> &str {
        match self {
            Self::Solve { on }
            | Self::Dissolve { on, .. }
            | Self::Reopen { on }
            | Self::Submit { on }
            | Self::Withdraw { on, .. }
            | Self::Address { on }
            | Self::Validate { on }
            | Self::Dismiss { on } => on,
        }
    }

    /// Which kind the target must be, so a reference can be resolved before
    /// anything is written.
    fn kind(&self) -> &'static str {
        match self {
            Self::Solve { .. } | Self::Dissolve { .. } | Self::Reopen { .. } => "problem",
            Self::Submit { .. } | Self::Withdraw { .. } => "solution",
            Self::Address { .. } | Self::Validate { .. } | Self::Dismiss { .. } => "critique",
        }
    }
}

/// Which fields an update may carry for a given kind.
///
/// Refusing an inapplicable field rather than ignoring it: `priority` on a
/// milestone is a mistake, and silently dropping it would leave the author
/// believing it was applied.
fn check_update_fields(u: &UpdateNode) -> Result<()> {
    let (priority_ok, tags_ok, assignee_ok) = match u.kind.as_str() {
        "problem" => (true, true, true),
        "solution" => (false, true, true),
        "finding" => (false, true, false),
        "milestone" => (false, false, true),
        "critique" => (false, false, false),
        other => {
            return Err(invalid(format!(
                "update on '{}': unknown kind '{other}' — use problem, solution, \
                 critique, finding or milestone",
                u.on
            )))
        }
    };
    if u.priority.is_some() && !priority_ok {
        return Err(invalid(format!(
            "update on '{}': a {} has no priority",
            u.on, u.kind
        )));
    }
    if u.tags.is_some() && !tags_ok {
        return Err(invalid(format!(
            "update on '{}': a {} has no tags",
            u.on, u.kind
        )));
    }
    if u.assignee.is_some() && !assignee_ok {
        return Err(invalid(format!(
            "update on '{}': a {} has no assignee",
            u.on, u.kind
        )));
    }
    if u.title.is_none()
        && u.body.is_none()
        && u.priority.is_none()
        && u.tags.is_none()
        && u.assignee.is_none()
    {
        return Err(invalid(format!("update on '{}' changes nothing", u.on)));
    }
    Ok(())
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct MilestoneNode {
    key: String,
    title: String,
    #[serde(default)]
    body: Option<String>,
    #[serde(default)]
    date: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProblemNode {
    key: String,
    title: String,
    #[serde(default)]
    body: Option<String>,
    #[serde(default)]
    priority: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
    /// A plan key, or an existing problem (id, prefix or fuzzy title).
    #[serde(default)]
    parent: Option<String>,
    #[serde(default)]
    milestone: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FindingNode {
    key: String,
    title: String,
    problem: String,
    #[serde(default)]
    body: Option<String>,
    #[serde(default)]
    method: Option<String>,
    #[serde(default)]
    refs: Vec<String>,
    /// When the evidence was measured, RFC3339. Distinct from when the plan is
    /// applied, which is the only time the entity would otherwise carry.
    #[serde(default)]
    measured_at: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SolutionNode {
    key: String,
    title: String,
    problem: String,
    #[serde(default)]
    body: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
    /// Findings this conjecture rests on — plan keys or existing findings.
    #[serde(default)]
    cites: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CritiqueNode {
    key: String,
    title: String,
    solution: String,
    #[serde(default)]
    body: Option<String>,
    #[serde(default)]
    severity: Option<String>,
}

fn invalid(msg: impl Into<String>) -> JjjError {
    JjjError::Validation(msg.into())
}

/// Every key in the plan, refused if any is duplicated.
///
/// A duplicate key is not a harmless collision: it silently decides which of two
/// nodes a `parent` reference points at, and the author cannot tell which.
fn collect_keys(plan: &Plan) -> Result<HashSet<String>> {
    let mut seen = HashSet::new();
    let all = plan
        .milestones
        .iter()
        .map(|n| &n.key)
        .chain(plan.problems.iter().map(|n| &n.key))
        .chain(plan.findings.iter().map(|n| &n.key))
        .chain(plan.solutions.iter().map(|n| &n.key))
        .chain(plan.critiques.iter().map(|n| &n.key));
    for key in all {
        if key.trim().is_empty() {
            return Err(invalid("a node has an empty key"));
        }
        if !seen.insert(key.clone()) {
            return Err(invalid(format!(
                "duplicate key '{key}' — keys must be unique across the whole plan"
            )));
        }
    }
    Ok(seen)
}

/// Apply one field update, honouring its own `if_lamport` guard.
///
/// Each update carries its own guard rather than the plan carrying one, because
/// a plan touching ten entities has ten independent "has this moved" answers.
fn apply_update(store: &crate::storage::MetadataStore, u: &UpdateNode, id: &str) -> Result<()> {
    let set_tags = |existing: &mut Vec<String>| {
        if let Some(t) = &u.tags {
            let mut v: Vec<String> = t
                .iter()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            v.sort();
            v.dedup();
            *existing = v;
        }
    };

    match u.kind.as_str() {
        "problem" => {
            let mut e = store.load_problem(id)?;
            super::guard_lamport(u.if_lamport, e.lamport, &u.kind, id)?;
            if let Some(t) = &u.title {
                e.title = t.clone();
            }
            if let Some(b) = &u.body {
                e.description = b.clone();
            }
            if let Some(pr) = &u.priority {
                e.priority = pr.parse::<Priority>().map_err(invalid)?;
            }
            if let Some(a) = &u.assignee {
                e.assignee = Some(a.clone());
            }
            set_tags(&mut e.tags);
            store.save_problem(&e)?;
        }
        "solution" => {
            let mut e = store.load_solution(id)?;
            super::guard_lamport(u.if_lamport, e.lamport, &u.kind, id)?;
            if let Some(t) = &u.title {
                e.title = t.clone();
            }
            if let Some(b) = &u.body {
                e.approach = b.clone();
            }
            if let Some(a) = &u.assignee {
                e.assignee = Some(a.clone());
            }
            set_tags(&mut e.tags);
            store.save_solution(&e)?;
        }
        "critique" => {
            let mut e = store.load_critique(id)?;
            super::guard_lamport(u.if_lamport, e.lamport, &u.kind, id)?;
            if let Some(t) = &u.title {
                e.title = t.clone();
            }
            if let Some(b) = &u.body {
                e.argument = b.clone();
            }
            store.save_critique(&e)?;
        }
        "finding" => {
            let mut e = store.load_finding(id)?;
            super::guard_lamport(u.if_lamport, e.lamport, &u.kind, id)?;
            if let Some(t) = &u.title {
                e.title = t.clone();
            }
            if let Some(b) = &u.body {
                e.evidence = b.clone();
            }
            set_tags(&mut e.tags);
            store.save_finding(&e)?;
        }
        "milestone" => {
            let mut e = store.load_milestone(id)?;
            super::guard_lamport(u.if_lamport, e.lamport, &u.kind, id)?;
            if let Some(t) = &u.title {
                e.title = t.clone();
            }
            if let Some(b) = &u.body {
                e.description = b.clone();
            }
            if let Some(a) = &u.assignee {
                e.assignee = Some(a.clone());
            }
            store.save_milestone(&e)?;
        }
        other => return Err(invalid(format!("unknown kind '{other}'"))),
    }
    Ok(())
}

pub fn execute(ctx: &CommandContext, file: String, dry_run: bool, json: bool) -> Result<()> {
    // `-` is stdin, anything else is a path. `read_body` would treat a filename
    // as the literal content, which fails in a confusing way: the parse error
    // points at column 1 of the filename.
    let text = if file == "-" {
        super::read_body(Some(file))?
    } else {
        std::fs::read_to_string(&file)
            .map_err(|e| invalid(format!("cannot read plan file '{file}': {e}")))?
    };
    let plan: Plan = serde_json::from_str(&text).map_err(|e| {
        invalid(format!(
            "could not read the plan: {e}\n\nExpected JSON with any of: \
             milestones, problems, findings, solutions, critiques."
        ))
    })?;

    let keys = collect_keys(&plan)?;
    let created = plan.milestones.len()
        + plan.problems.len()
        + plan.findings.len()
        + plan.solutions.len()
        + plan.critiques.len();
    let total = created + plan.updates.len() + plan.actions.len();
    if total == 0 {
        return Err(invalid("the plan does nothing"));
    }

    // key → id, filled in as entities are created. A reference that is not a
    // plan key is resolved against the repository instead, so a plan can hang
    // new work off problems that already exist.
    let mut ids: HashMap<String, String> = HashMap::new();

    // ── Validation pass. Everything that can be checked without writing. ──

    for m in &plan.milestones {
        if let Some(d) = &m.date {
            chrono::NaiveDate::parse_from_str(d, "%Y-%m-%d").map_err(|_| {
                invalid(format!(
                    "milestone '{}': bad date '{d}', want YYYY-MM-DD",
                    m.key
                ))
            })?;
        }
    }
    for p in &plan.problems {
        if let Some(pr) = &p.priority {
            pr.parse::<Priority>()
                .map_err(|e: String| invalid(format!("problem '{}': {e}", p.key)))?;
        }
    }
    for c in &plan.critiques {
        if let Some(sev) = &c.severity {
            sev.parse::<CritiqueSeverity>()
                .map_err(|e: String| invalid(format!("critique '{}': {e}", c.key)))?;
        }
    }

    // Parent references must form a DAG within the plan. A cycle would other-
    // wise be discovered only by the creation loop failing to make progress,
    // which is a confusing way to learn you wrote one.
    let plan_problem_keys: HashSet<&str> = plan.problems.iter().map(|p| p.key.as_str()).collect();
    let parent_in_plan: HashMap<&str, &str> = plan
        .problems
        .iter()
        .filter_map(|p| {
            p.parent
                .as_deref()
                .filter(|parent| plan_problem_keys.contains(parent))
                .map(|parent| (p.key.as_str(), parent))
        })
        .collect();
    for start in plan_problem_keys.iter() {
        let mut seen = HashSet::new();
        let mut at = *start;
        while let Some(next) = parent_in_plan.get(at) {
            if !seen.insert(at) {
                return Err(invalid(format!(
                    "the plan's parent references form a cycle, reached from '{start}'"
                )));
            }
            at = next;
        }
    }

    // Any reference that is not a plan key must already exist. Resolving now
    // means a typo'd problem title fails before a single entity is written.
    let mut external: HashMap<String, String> = HashMap::new();
    let mut resolve_external = |r: &str, kind: &str| -> Result<()> {
        if keys.contains(r) || external.contains_key(r) {
            return Ok(());
        }
        let resolved = match kind {
            "problem" => ctx.resolve_problem(r),
            "solution" => ctx.resolve_solution(r),
            "critique" => ctx.resolve_critique(r),
            "finding" => ctx.resolve_finding(r),
            "milestone" => ctx.resolve_milestone(r),
            other => return Err(invalid(format!("unknown reference kind '{other}'"))),
        }
        .map_err(|_| {
            invalid(format!(
                "'{r}' is neither a key in this plan nor an existing {kind}"
            ))
        })?;
        external.insert(r.to_string(), resolved);
        Ok(())
    };

    for p in &plan.problems {
        if let Some(parent) = &p.parent {
            resolve_external(parent, "problem")?;
        }
        if let Some(m) = &p.milestone {
            resolve_external(m, "milestone")?;
        }
    }
    for f in &plan.findings {
        resolve_external(&f.problem, "problem")?;
        if let Some(raw) = &f.measured_at {
            crate::commands::finding::parse_measured_at(raw, "measured_at")
                .map_err(|m| invalid(format!("finding '{}': {m}", f.key)))?;
        }
    }
    for s in &plan.solutions {
        resolve_external(&s.problem, "problem")?;
        for c in &s.cites {
            resolve_external(c, "finding")?;
        }
    }
    for c in &plan.critiques {
        resolve_external(&c.solution, "solution")?;
    }
    for u in &plan.updates {
        check_update_fields(u)?;
        if let Some(pr) = &u.priority {
            pr.parse::<Priority>()
                .map_err(|e: String| invalid(format!("update on '{}': {e}", u.on)))?;
        }
        resolve_external(&u.on, &u.kind)?;
    }
    for a in &plan.actions {
        resolve_external(a.target(), a.kind())?;
    }

    if dry_run {
        println!(
            "Plan is valid: {total} operation{} would run.",
            if total == 1 { "" } else { "s" }
        );
        for (label, n) in [
            ("milestones", plan.milestones.len()),
            ("problems", plan.problems.len()),
            ("findings", plan.findings.len()),
            ("solutions", plan.solutions.len()),
            ("critiques", plan.critiques.len()),
            ("field updates", plan.updates.len()),
            ("lifecycle actions", plan.actions.len()),
        ] {
            if n > 0 {
                println!("  {n} {label}");
            }
        }
        if !external.is_empty() {
            println!("\nReferences to existing entities:");
            for (r, id) in &external {
                println!("  {r} → {}", crate::display::short_id(id));
            }
        }
        println!("\nNothing was written (--dry-run).");
        return Ok(());
    }

    // ── Write pass. One metadata commit for the whole plan. ──

    let store = &ctx.store;
    // Read once, outside the commit: a critique or finding from a plan is
    // authored by whoever applied it, and without this every entity a plan
    // creates is anonymous — invisible to `--mine` and to author filters.
    let user = store.get_current_user()?;
    let look_up = |r: &str, ids: &HashMap<String, String>| -> String {
        ids.get(r)
            .or_else(|| external.get(r))
            .cloned()
            .unwrap_or_else(|| r.to_string())
    };

    store.with_metadata(&format!("Apply plan ({total} entities)"), || {
        for m in &plan.milestones {
            let mut milestone = Milestone::new(crate::id::generate_id(), &m.title);
            if let Some(b) = &m.body {
                milestone.description = b.clone();
            }
            if let Some(d) = &m.date {
                milestone.target_date = chrono::NaiveDate::parse_from_str(d, "%Y-%m-%d")
                    .ok()
                    .and_then(|nd| nd.and_hms_opt(0, 0, 0))
                    .map(|dt| dt.and_utc());
            }
            store.save_milestone(&milestone)?;
            ids.insert(m.key.clone(), milestone.id);
        }

        // Parents before children, so a child's `parent_id` is a real id. The
        // cycle check above guarantees this terminates.
        let mut pending: Vec<&ProblemNode> = plan.problems.iter().collect();
        while !pending.is_empty() {
            let before = pending.len();
            pending.retain(|p| {
                let parent_ready = match &p.parent {
                    None => true,
                    Some(parent) => ids.contains_key(parent) || external.contains_key(parent),
                };
                if !parent_ready {
                    return true;
                }
                let mut problem = Problem::new(crate::id::generate_id(), &p.title);
                if let Some(b) = &p.body {
                    problem.description = b.clone();
                }
                if let Some(pr) = &p.priority {
                    if let Ok(parsed) = pr.parse::<Priority>() {
                        problem.priority = parsed;
                    }
                }
                problem.tags = p.tags.clone();
                problem.tags.sort();
                problem.tags.dedup();
                if let Some(parent) = &p.parent {
                    problem.parent_id = Some(look_up(parent, &ids));
                }
                if let Some(m) = &p.milestone {
                    problem.milestone_id = Some(look_up(m, &ids));
                }
                let id = problem.id.clone();
                // A save failure here is an I/O fault; record nothing and let
                // the loop below report the node as unresolved rather than
                // claiming a key that does not exist.
                if store.save_problem(&problem).is_ok() {
                    ids.insert(p.key.clone(), id);
                }
                false
            });
            if pending.len() == before {
                return Err(invalid(format!(
                    "could not create {} problem(s) — a parent reference never resolved",
                    pending.len()
                )));
            }
        }

        for f in &plan.findings {
            let mut finding = Finding::new(
                crate::id::generate_id(),
                &f.title,
                look_up(&f.problem, &ids),
            );
            if let Some(b) = &f.body {
                finding.evidence = b.clone();
            }
            finding.method = f.method.clone();
            finding.refs = f.refs.clone();
            // Validated above, so this cannot fail here.
            finding.measured_at = f
                .measured_at
                .as_deref()
                .map(|raw| crate::commands::finding::parse_measured_at(raw, "measured_at"))
                .transpose()
                .map_err(crate::error::JjjError::Validation)?;
            finding.author = Some(user.clone());
            store.save_finding(&finding)?;
            ids.insert(f.key.clone(), finding.id);
        }

        for s in &plan.solutions {
            let mut solution = Solution::new(
                crate::id::generate_id(),
                &s.title,
                look_up(&s.problem, &ids),
            );
            if let Some(b) = &s.body {
                solution.approach = b.clone();
            }
            solution.tags = s.tags.clone();
            solution.tags.sort();
            solution.tags.dedup();
            solution.cites = s.cites.iter().map(|c| look_up(c, &ids)).collect();
            store.save_solution(&solution)?;
            ids.insert(s.key.clone(), solution.id);
        }

        for c in &plan.critiques {
            let mut critique = Critique::new(
                crate::id::generate_id(),
                &c.title,
                look_up(&c.solution, &ids),
            );
            if let Some(b) = &c.body {
                critique.argument = b.clone();
            }
            if let Some(sev) = &c.severity {
                if let Ok(parsed) = sev.parse::<CritiqueSeverity>() {
                    critique.severity = parsed;
                }
            }
            critique.author = Some(user.clone());
            store.save_critique(&critique)?;
            ids.insert(c.key.clone(), critique.id);
        }

        // Field updates. Annotations only — status transitions are `actions`.
        for u in &plan.updates {
            let id = look_up(&u.on, &ids);
            apply_update(store, u, &id)?;
        }

        // Lifecycle, through the same functions the single-entity commands use,
        // so a guard that refuses here refuses there too.
        for a in &plan.actions {
            let id = look_up(a.target(), &ids);
            match a {
                ActionNode::Solve { .. } => crate::domain::solve_problem(store, &id)?,
                ActionNode::Dissolve { reason, .. } => {
                    crate::domain::dissolve_problem(store, &id, reason.as_deref())?
                }
                ActionNode::Reopen { .. } => crate::domain::reopen_problem(store, &id)?,
                ActionNode::Submit { .. } => crate::domain::submit_solution(store, &id)?,
                ActionNode::Withdraw { rationale, .. } => {
                    crate::domain::withdraw_solution(store, &id, rationale.as_deref())?
                }
                ActionNode::Address { .. } => crate::domain::address_critique(store, &id)?,
                ActionNode::Validate { .. } => crate::domain::validate_critique(store, &id)?,
                ActionNode::Dismiss { .. } => crate::domain::dismiss_critique(store, &id)?,
            }
        }

        Ok(())
    })?;

    // The key → id map is the whole point: without it the caller has created
    // work it cannot name.
    if json {
        println!("{}", serde_json::to_string_pretty(&ids)?);
    } else {
        println!(
            "Applied {total} operation{} ({created} created):",
            if total == 1 { "" } else { "s" }
        );
        let mut rows: Vec<_> = ids.iter().collect();
        rows.sort();
        for (key, id) in rows {
            println!("  {key:20} {id}");
        }
    }
    Ok(())
}
