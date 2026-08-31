//! `jjj contention` — where the fleet is piled up, and where nobody is.
//!
//! # Why this is a re-ranking and not an assignment
//!
//! Firstmate prevents duplicated work by construction: one first mate routes
//! each request, so two crewmates cannot pick the same thing. That is a better
//! answer than claims and staggering to the problem of two agents building the
//! same idea — and it is refused here, because it would also prevent two agents
//! independently attacking one problem with rival conjectures, which is the
//! thing a swarm is *for*. Duplication is the price of decentralisation plus
//! rivalry, not a defect to eliminate.
//!
//! So this command reports; it does not route. It gives the integrator — the
//! one actor with a whole-queue view — the two facts it needs to nudge the
//! fleet with `jjj rank move`: which problems several actors are already on, and
//! which have nobody. A re-ranking is deliberately weaker than an assignment: it
//! changes what agents see first, and any of them may still disagree, because
//! every actor's ordering is aggregated rather than overwritten.
//!
//! # Contention is not the same as rivalry
//!
//! Three actors on one problem may be three rival conjectures, which is the
//! method working. What makes it waste is the *rest* of the queue sitting
//! untouched at the same time. The report therefore pairs the two, and says
//! nothing is wrong when there is nowhere else for the fleet to go.

use crate::context::CommandContext;
use crate::display::truncated_prefixes;
use crate::error::Result;
use crate::models::{Problem, ProblemStatus, Solution, SolutionStatus};
use std::collections::{HashMap, HashSet};

/// One problem's engagement: who is on it, and how.
#[derive(Debug, serde::Serialize)]
pub struct Engagement {
    pub problem_id: String,
    pub title: String,
    /// Distinct actors with a live claim or a standing solution.
    pub actors: Vec<String>,
    /// Solutions not withdrawn.
    pub live_solutions: usize,
    /// Whether someone holds a claim whose lease has not lapsed.
    pub claimed: bool,
}

pub fn execute(ctx: &CommandContext, json: bool) -> Result<()> {
    let store = &ctx.store;
    let problems = store.list_problems()?;
    let solutions = store.list_solutions()?;
    let events = store.list_events_cached()?;
    let ttl = crate::claim::claim_ttl(&store.load_config().unwrap_or_default().settings);
    let now = chrono::Utc::now();

    // A solution carries an assignee but not an author, so recover who proposed
    // what from the creation event. Assignee is the fallback, not the answer:
    // reassignment would otherwise rewrite who was working on a problem.
    let mut author_of: HashMap<&str, &str> = HashMap::new();
    for e in &events {
        if e.event_type == crate::models::EventType::SolutionCreated && !e.by.is_empty() {
            author_of.insert(e.entity.as_str(), e.by.as_str());
        }
    }

    let engagements = engagements(&problems, &solutions, &author_of, ttl, now);

    let contended: Vec<&Engagement> = engagements.iter().filter(|e| e.actors.len() > 1).collect();
    let untouched: Vec<&Engagement> = engagements
        .iter()
        .filter(|e| e.actors.is_empty() && e.live_solutions == 0)
        .collect();

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "contended": contended,
                "untouched": untouched,
                // The judgement, made once here rather than re-derived by every
                // caller: several actors on one problem is only waste when the
                // fleet has somewhere else it could be.
                "should_rebalance": !contended.is_empty() && !untouched.is_empty(),
            }))?
        );
        return Ok(());
    }

    if contended.is_empty() && untouched.is_empty() {
        println!("The queue is neither contended nor idle — nothing to rebalance.");
        return Ok(());
    }

    // Prefixes auto-extended for uniqueness across every problem, not the fixed
    // short_id. This report's whole output is commands meant to be pasted, and
    // UUID7 ids are time-ordered — sibling problems seeded in the same second
    // share their first six characters, so a fixed prefix printed three
    // identical, ambiguous `rank move` lines.
    let all_ids: Vec<&str> = problems.iter().map(|p| p.id.as_str()).collect();
    // Built from the returned (uuid, prefix) pairs rather than by zipping
    // against the input, so the mapping cannot desync if the helper ever
    // reorders its output.
    let prefixes: HashMap<String, String> = truncated_prefixes(&all_ids).into_iter().collect();
    let pre = |id: &str| prefixes.get(id).cloned().unwrap_or_else(|| id.to_string());

    if !contended.is_empty() {
        println!(
            "Contended ({} problem{}):",
            contended.len(),
            if contended.len() == 1 { "" } else { "s" }
        );
        for e in &contended {
            println!(
                "  p/{}  {} actors  {}",
                pre(&e.problem_id),
                e.actors.len(),
                crate::utils::truncate(&e.title, 52)
            );
            println!("      {}", e.actors.join(", "));
        }
    }

    if !untouched.is_empty() {
        println!(
            "\nUntouched ({} open problem{}, nobody on them):",
            untouched.len(),
            if untouched.len() == 1 { "" } else { "s" }
        );
        for e in untouched.iter().take(10) {
            println!(
                "  p/{}  {}",
                pre(&e.problem_id),
                crate::utils::truncate(&e.title, 60)
            );
        }
        if untouched.len() > 10 {
            println!("  … and {} more", untouched.len() - 10);
        }
    }

    println!();
    if !contended.is_empty() && !untouched.is_empty() {
        println!("Several actors are on one problem while others have nobody.");
        println!("Nudge the fleet by re-ranking, not by reassigning:");
        for e in untouched.iter().take(3) {
            println!("  jjj rank move {} top", pre(&e.problem_id));
        }
        println!();
        println!("A ranking changes what agents see first and any of them may still");
        println!("disagree. Rival conjectures on one problem are the method; the waste");
        println!("is only that the rest of the queue is sitting idle at the same time.");
    } else if contended.is_empty() {
        println!("Nobody is doubled up. The untouched problems are simply unstarted.");
    } else {
        println!("Actors are doubled up, but there is nowhere else for them to go —");
        println!("that is rivalry on the only work there is, not a misallocation.");
    }

    Ok(())
}

/// Who is engaged on each open problem.
///
/// Pure, and takes `now` rather than reading the clock, so the claim-expiry
/// boundary is testable without waiting an hour — the same reason
/// [`crate::claim::classify`] does.
pub fn engagements(
    problems: &[Problem],
    solutions: &[Solution],
    author_of: &HashMap<&str, &str>,
    ttl: chrono::Duration,
    now: chrono::DateTime<chrono::Utc>,
) -> Vec<Engagement> {
    let mut by_problem: HashMap<&str, (HashSet<String>, usize)> = HashMap::new();

    for s in solutions {
        // A withdrawn solution is effort already abandoned; counting it would
        // report contention on a problem everyone has left.
        if s.status == SolutionStatus::Withdrawn {
            continue;
        }
        let entry = by_problem
            .entry(s.problem_id.as_str())
            .or_insert_with(|| (HashSet::new(), 0));
        entry.1 += 1;
        if let Some(author) = author_of
            .get(s.id.as_str())
            .copied()
            .or(s.assignee.as_deref())
        {
            entry.0.insert(author.to_string());
        }
    }

    let mut out = Vec::new();
    for p in problems {
        // Only work still in play. A solved problem with four solutions on it is
        // history, not a pile-up.
        if !matches!(p.status, ProblemStatus::Open | ProblemStatus::InProgress) {
            continue;
        }
        let (mut actors, live) = by_problem
            .get(p.id.as_str())
            .map(|(a, n)| (a.clone(), *n))
            .unwrap_or_default();

        // A live claim counts as engagement even with nothing proposed yet —
        // that is the whole point of claiming before building. A lapsed one does
        // not: the agent holding it may well be dead.
        let claimed = matches!(
            crate::claim::classify(p.assignee.as_deref(), p.claimed_at, "\u{0}none", ttl, now),
            crate::claim::ClaimState::HeldByOther
        );
        if claimed {
            if let Some(a) = p.assignee.clone() {
                actors.insert(a);
            }
        }

        let mut actors: Vec<String> = actors.into_iter().collect();
        actors.sort();
        out.push(Engagement {
            problem_id: p.id.clone(),
            title: p.title.clone(),
            actors,
            live_solutions: live,
            claimed,
        });
    }

    // Most-contended first: the report is read top-down by someone deciding
    // where to nudge the fleet.
    out.sort_by(|a, b| {
        b.actors
            .len()
            .cmp(&a.actors.len())
            .then_with(|| a.problem_id.cmp(&b.problem_id))
    });
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};

    fn problem(id: &str, assignee: Option<&str>, claimed_ago: Option<i64>) -> Problem {
        let mut p = Problem::new(id, format!("Problem {id}"));
        p.assignee = assignee.map(|s| s.to_string());
        p.claimed_at = claimed_ago.map(|m| Utc::now() - Duration::minutes(m));
        p
    }

    fn solution(id: &str, problem_id: &str, status: SolutionStatus) -> Solution {
        let mut s = Solution::new(id, format!("Solution {id}"), problem_id);
        s.status = status;
        s
    }

    fn ttl() -> Duration {
        Duration::minutes(60)
    }

    #[test]
    fn distinct_authors_on_one_problem_are_contention() {
        let problems = vec![problem("p1", None, None)];
        let solutions = vec![
            solution("s1", "p1", SolutionStatus::Proposed),
            solution("s2", "p1", SolutionStatus::Submitted),
        ];
        let mut authors = HashMap::new();
        authors.insert("s1", "agent-a");
        authors.insert("s2", "agent-b");

        let e = engagements(&problems, &solutions, &authors, ttl(), Utc::now());
        assert_eq!(e[0].actors, vec!["agent-a", "agent-b"]);
        assert_eq!(e[0].live_solutions, 2);
    }

    #[test]
    fn one_agent_iterating_is_not_contention() {
        // Two attempts from one actor is iteration. Reporting it as a pile-up
        // would send the integrator rebalancing a queue that is fine.
        let problems = vec![problem("p1", None, None)];
        let solutions = vec![
            solution("s1", "p1", SolutionStatus::Withdrawn),
            solution("s2", "p1", SolutionStatus::Proposed),
        ];
        let mut authors = HashMap::new();
        authors.insert("s1", "agent-a");
        authors.insert("s2", "agent-a");

        let e = engagements(&problems, &solutions, &authors, ttl(), Utc::now());
        assert_eq!(e[0].actors.len(), 1);
    }

    #[test]
    fn withdrawn_solutions_do_not_count() {
        // Effort already abandoned. Counting it reports contention on a problem
        // everyone has left.
        let problems = vec![problem("p1", None, None)];
        let solutions = vec![
            solution("s1", "p1", SolutionStatus::Withdrawn),
            solution("s2", "p1", SolutionStatus::Withdrawn),
        ];
        let mut authors = HashMap::new();
        authors.insert("s1", "agent-a");
        authors.insert("s2", "agent-b");

        let e = engagements(&problems, &solutions, &authors, ttl(), Utc::now());
        assert!(e[0].actors.is_empty());
        assert_eq!(e[0].live_solutions, 0);
    }

    #[test]
    fn a_live_claim_counts_but_a_lapsed_one_does_not() {
        let problems = vec![
            problem("p1", Some("agent-a"), Some(5)),
            problem("p2", Some("agent-b"), Some(120)),
        ];
        let e = engagements(&problems, &[], &HashMap::new(), ttl(), Utc::now());
        let by_id: HashMap<_, _> = e.iter().map(|x| (x.problem_id.as_str(), x)).collect();
        assert!(by_id["p1"].claimed, "a five-minute-old claim is live");
        assert!(
            !by_id["p2"].claimed,
            "a two-hour-old claim has lapsed; the agent holding it may be dead"
        );
    }

    #[test]
    fn solved_problems_are_history_not_a_pile_up() {
        let mut p = problem("p1", None, None);
        p.status = ProblemStatus::Solved;
        let solutions = vec![
            solution("s1", "p1", SolutionStatus::Approved),
            solution("s2", "p1", SolutionStatus::Proposed),
        ];
        let mut authors = HashMap::new();
        authors.insert("s1", "agent-a");
        authors.insert("s2", "agent-b");

        let e = engagements(&[p], &solutions, &authors, ttl(), Utc::now());
        assert!(e.is_empty(), "a solved problem is not in play");
    }

    #[test]
    fn most_contended_comes_first() {
        let problems = vec![
            problem("p1", None, None),
            problem("p2", None, None),
            problem("p3", None, None),
        ];
        let solutions = vec![
            solution("s1", "p2", SolutionStatus::Proposed),
            solution("s2", "p2", SolutionStatus::Proposed),
            solution("s3", "p3", SolutionStatus::Proposed),
        ];
        let mut authors = HashMap::new();
        authors.insert("s1", "a");
        authors.insert("s2", "b");
        authors.insert("s3", "a");

        let e = engagements(&problems, &solutions, &authors, ttl(), Utc::now());
        assert_eq!(e[0].problem_id, "p2");
        assert_eq!(e.last().unwrap().actors.len(), 0);
    }
}
