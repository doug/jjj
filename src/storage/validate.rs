//! Referential integrity validation over the markdown entities.
//!
//! This is the same gate `jjj push` has always run, moved off SQLite.
//!
//! The check is the one place the cache was load-bearing for *correctness*
//! rather than for speed: `push` opened the database solely to run
//! `db::validate`, so a plan to remove the cache would have quietly removed a
//! pre-publish check that stops a dangling reference reaching the shared
//! bookmark. Reimplementing it against the markdown — which is the data being
//! pushed, and the only thing the remote will ever see — removes the dependency
//! without weakening the gate, and drops a layer of indirection: the SQL
//! version validated whatever the last `dump_to_markdown` happened to put in
//! the cache, one copy removed from the files on disk.

use std::collections::{HashMap, HashSet};

use crate::error::Result;
use crate::models::{Critique, Finding, Milestone, Problem, Solution};
use crate::storage::MetadataStore;

/// A broken reference found before publishing.
#[derive(Debug, Clone)]
pub struct ValidationError {
    pub entity_id: String,
    pub message: String,
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.entity_id, self.message)
    }
}

impl std::error::Error for ValidationError {}

/// Validate every reference between entities on disk.
///
/// Checks, in the order their errors are reported:
/// 1. a problem's `parent_id` names a problem that exists
/// 2. a problem's `milestone_id` names a milestone that exists
/// 3. a solution's `problem_id` names a problem that exists
/// 4. a solution's `supersedes` names a solution that exists
/// 5. a critique's `solution_id` names a solution that exists
/// 6. a finding's `problem_id` names a problem that exists — evidence about a
///    problem that is gone would never surface in any listing
/// 7. no cycle in any problem's parent chain
/// 8. no body still carries three-way merge conflict markers
///
/// The last one is not a reference check but belongs here all the same: without
/// it the `<<<<<<<` text is pushed to the shared bookmark and every other clone
/// inherits the conflict.
pub fn validate(store: &MetadataStore) -> Result<Vec<ValidationError>> {
    // `list_fs`, not `list_problems` and friends. The public listings are
    // DB-primary: when a cache exists and is clean they are served straight out
    // of it, so an entity written to markdown but not yet loaded into the cache
    // is simply invisible to them. Validating through those would have meant
    // checking the cache's copy of the data while claiming to check the files —
    // which is both the dependency this change exists to remove and a weaker
    // gate than the one it replaced, since the SQL version at least reloaded
    // from markdown first.
    //
    // `list_fs` is the authoritative canonical read, and its own documentation
    // warns about precisely this: the first version of `unknown_frontmatter_keys`
    // walked the public listings and therefore found nothing in any repository
    // that had a cache, which is every repository that has run a command.
    let problems = store.list_fs::<Problem>()?;
    let solutions = store.list_fs::<Solution>()?;
    let critiques = store.list_fs::<Critique>()?;
    let findings = store.list_fs::<Finding>()?;
    let milestones = store.list_fs::<Milestone>()?;

    let problem_ids: HashSet<&str> = problems.iter().map(|p| p.id.as_str()).collect();
    let solution_ids: HashSet<&str> = solutions.iter().map(|s| s.id.as_str()).collect();
    let milestone_ids: HashSet<&str> = milestones.iter().map(|m| m.id.as_str()).collect();

    let mut errors = Vec::new();
    let mut bad = |id: &str, message: String| {
        errors.push(ValidationError {
            entity_id: id.to_string(),
            message,
        })
    };

    for p in &problems {
        if let Some(parent) = &p.parent_id {
            if !problem_ids.contains(parent.as_str()) {
                bad(
                    &p.id,
                    format!("references non-existent parent problem '{}'", parent),
                );
            }
        }
        if let Some(m) = &p.milestone_id {
            if !milestone_ids.contains(m.as_str()) {
                bad(&p.id, format!("references non-existent milestone '{}'", m));
            }
        }
    }

    for s in &solutions {
        if !problem_ids.contains(s.problem_id.as_str()) {
            bad(
                &s.id,
                format!("references non-existent problem '{}'", s.problem_id),
            );
        }
        if let Some(sup) = &s.supersedes {
            if !solution_ids.contains(sup.as_str()) {
                bad(&s.id, format!("supersedes non-existent solution '{}'", sup));
            }
        }
    }

    for c in &critiques {
        if !solution_ids.contains(c.solution_id.as_str()) {
            bad(
                &c.id,
                format!("references non-existent solution '{}'", c.solution_id),
            );
        }
    }

    for f in &findings {
        if !problem_ids.contains(f.problem_id.as_str()) {
            bad(
                &f.id,
                format!("references non-existent problem '{}'", f.problem_id),
            );
        }
    }

    // Parent chain cycles. Built once as a map rather than re-queried per hop,
    // which is what the SQL version did — one statement per step of every
    // chain, for every problem.
    let parent_of: HashMap<&str, &str> = problems
        .iter()
        .filter_map(|p| p.parent_id.as_deref().map(|parent| (p.id.as_str(), parent)))
        .collect();
    for p in &problems {
        if has_parent_cycle(p.id.as_str(), &parent_of, &problem_ids) {
            bad(&p.id, "has a cycle in its parent chain".to_string());
        }
    }

    // Unresolved merge markers in any body. Kept in the same order the SQL
    // version used, so the reported set is identical for an identical repo.
    for p in &problems {
        check_markers(&p.id, &p.description, &mut errors);
    }
    for s in &solutions {
        check_markers(&s.id, &s.approach, &mut errors);
    }
    for c in &critiques {
        check_markers(&c.id, &c.argument, &mut errors);
    }
    for f in &findings {
        check_markers(&f.id, &f.evidence, &mut errors);
    }
    for m in &milestones {
        check_markers(&m.id, &m.description, &mut errors);
    }

    Ok(errors)
}

fn check_markers(id: &str, body: &str, errors: &mut Vec<ValidationError>) {
    if crate::storage::merge::has_conflict_markers(body) {
        errors.push(ValidationError {
            entity_id: id.to_string(),
            message: "has unresolved merge conflict markers (<<<<<<< / >>>>>>>); \
                      resolve them before pushing"
                .to_string(),
        });
    }
}

/// Walk a problem's parent chain, reporting whether it revisits an id.
///
/// A chain that runs into a parent which does not exist is not a cycle; the
/// dangling reference is reported separately, and treating it as a cycle too
/// would name the same fault twice with the less useful message.
fn has_parent_cycle(start: &str, parent_of: &HashMap<&str, &str>, valid: &HashSet<&str>) -> bool {
    let mut visited = HashSet::new();
    let mut current = start;
    loop {
        if !visited.insert(current) {
            return true;
        }
        match parent_of.get(current) {
            Some(parent) => {
                if !valid.contains(*parent) {
                    return false;
                }
                current = parent;
            }
            None => return false,
        }
    }
}
