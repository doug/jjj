//! `jjj escalate` — say that a person is needed.
//!
//! See [`crate::escalation`] for why this is an event rather than an entity, and
//! for the outage that motivated it.

use crate::context::CommandContext;
use crate::display::short_id;
use crate::error::{JjjError, Result};
use crate::escalation::{open_escalations, OpenEscalation};
use crate::models::{Event, EventType};

#[allow(clippy::too_many_arguments)]
pub fn execute(
    ctx: &CommandContext,
    reason: Option<String>,
    about: Vec<String>,
    list: bool,
    clear: Option<String>,
    respond: Option<String>,
    json: bool,
) -> Result<()> {
    match (clear, respond, reason) {
        (Some(id), _, _) => clear_escalation(ctx, id, json),
        (None, Some(id), message) => respond_to_escalation(ctx, id, message, json),
        (None, None, Some(reason)) if !list => raise(ctx, reason, about, json),
        _ => show_open(ctx, json),
    }
}

/// Record a person's reply, leaving the escalation open.
///
/// Answering and clearing are deliberately separate. A person who has looked at
/// the problem usually knows something useful well before it is fixed — "the key
/// rotates at 09:00", "this needs the prod account, I have asked" — and an
/// escalation that can only be closed forces them to choose between saying
/// nothing and claiming it is solved. Either way the next agent to hit the same
/// wall used to find nothing to read and rediscovered it.
fn respond_to_escalation(
    ctx: &CommandContext,
    input: String,
    message: Option<String>,
    json: bool,
) -> Result<()> {
    let store = &ctx.store;
    let message = message.ok_or_else(|| {
        JjjError::Validation(
            "--respond needs a message: jjj escalate \"what you found\" --respond <id>".to_string(),
        )
    })?;
    if message.trim().is_empty() {
        return Err(JjjError::Validation(
            "an empty answer says nothing — give the message as the reason argument".to_string(),
        ));
    }

    let target = resolve_open(store, &input)?;
    let user = store.get_current_user()?;
    let id = target.id.clone();

    store.with_metadata(&format!("Answer escalation {}", id), || {
        let event = Event::new(EventType::EscalationAnswered, id.clone(), user.clone())
            .with_rationale(message.clone());
        store.set_pending_event(event);
        Ok(())
    })?;

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "id": id,
                "answered_by": user,
                "message": message,
                "still_open": true,
            }))?
        );
    } else {
        println!("Answered escalation {} — still open.", short_id(&id));
        println!(
            "Clear it with `jjj escalate --clear {}` once it is fixed.",
            short_id(&id)
        );
    }
    Ok(())
}

/// Print open escalations. Also the no-argument form, so a bare `jjj escalate`
/// answers "is anyone blocked" rather than erroring on a missing reason.
fn show_open(ctx: &CommandContext, json: bool) -> Result<()> {
    let open = open_escalations(&ctx.store.list_events_cached()?);

    if json {
        println!("{}", serde_json::to_string_pretty(&open)?);
        return Ok(());
    }

    if open.is_empty() {
        println!("No open escalations.");
        return Ok(());
    }

    let now = chrono::Utc::now();
    println!(
        "{} open escalation{}:",
        open.len(),
        if open.len() == 1 { "" } else { "s" }
    );
    for e in &open {
        print_one(e, now);
    }
    println!("\nAnswer with: jjj escalate \"what you found\" --respond <id>");
    println!("Clear with:  jjj escalate --clear <id>");
    Ok(())
}

fn print_one(e: &OpenEscalation, now: chrono::DateTime<chrono::Utc>) {
    let age = e.age(now);
    let waited = if age.num_hours() >= 1 {
        format!("{}h", age.num_hours())
    } else {
        format!("{}m", age.num_minutes().max(0))
    };
    println!(
        "  {} [{} ago, {}] {}",
        short_id(&e.id),
        waited,
        e.by,
        e.reason
    );
    if !e.about.is_empty() {
        println!(
            "      about: {}",
            e.about
                .iter()
                .map(|a| short_id(a))
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    // The reply is the part an agent needs most: it is the difference between
    // "blocked, nobody knows why" and "blocked, and here is what was found".
    for a in &e.answers {
        println!("      ↳ {}: {}", a.by, a.message);
    }
}

fn raise(ctx: &CommandContext, reason: String, about: Vec<String>, json: bool) -> Result<()> {
    let reason = reason.trim().to_string();
    if reason.is_empty() {
        return Err(JjjError::Validation(
            "an escalation needs a reason — say what a person has to do".to_string(),
        ));
    }

    let store = &ctx.store;
    let user = store.get_current_user()?;

    // `--about` takes any entity kind, resolved the same untyped way findings
    // resolve `--ref`: what blocks an agent is rarely known to be one kind of
    // thing in advance.
    let mut refs = Vec::with_capacity(about.len());
    for a in &about {
        let id = ctx
            .resolve_problem(a)
            .or_else(|_| ctx.resolve_solution(a))
            .or_else(|_| ctx.resolve_critique(a))
            .or_else(|_| ctx.resolve_finding(a))
            .map_err(|_| JjjError::Validation(format!("--about '{a}' matched no entity")))?;
        if !refs.contains(&id) {
            refs.push(id);
        }
    }

    // The escalation's own id: it names the escalation, not an entity, which is
    // what makes it clearable without inventing a table to hold a status.
    let id = crate::id::generate_id();

    store.with_metadata(&format!("Escalate: {}", reason), || {
        let event = Event::new(EventType::EscalationRaised, id.clone(), user.clone())
            .with_rationale(reason.clone())
            .with_refs(refs.clone());
        store.set_pending_event(event);
        Ok(())
    })?;

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "id": id,
                "by": user,
                "reason": reason,
                "about": refs,
            }))?
        );
    } else {
        println!("Escalated: {}", reason);
        println!("  id: {}", id);
        // Say what happens next, because the failure mode this command exists to
        // fix is an escalation nobody notices.
        println!("  Surfaces in `jjj status` and `jjj escalate` until cleared.");
        println!("  Push it (`jjj push`) so anyone else can see it.");
    }
    Ok(())
}

/// Resolve a prefix against **open** escalations only.
///
/// Matching a closed one would hide a typo: the caller would be told their
/// action succeeded against something nobody is waiting on.
fn resolve_open(store: &crate::storage::MetadataStore, input: &str) -> Result<OpenEscalation> {
    let open = open_escalations(&store.list_events_cached()?);
    let matches: Vec<&OpenEscalation> = open
        .iter()
        .filter(|e| e.id == input || e.id.starts_with(input))
        .collect();

    match matches.len() {
        0 => Err(JjjError::Validation(format!(
            "no open escalation matches '{input}' — `jjj escalate` lists them"
        ))),
        1 => Ok(matches[0].clone()),
        n => Err(JjjError::Validation(format!(
            "'{input}' matches {n} open escalations; use a longer prefix"
        ))),
    }
}

fn clear_escalation(ctx: &CommandContext, input: String, json: bool) -> Result<()> {
    let store = &ctx.store;
    let target = resolve_open(store, &input)?;

    let user = store.get_current_user()?;
    let id = target.id.clone();

    store.with_metadata(&format!("Clear escalation {}", id), || {
        let event = Event::new(EventType::EscalationCleared, id.clone(), user.clone())
            .with_rationale(target.reason.clone());
        store.set_pending_event(event);
        Ok(())
    })?;

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "id": id,
                "cleared_by": user,
                "reason": target.reason,
            }))?
        );
    } else {
        println!("Cleared escalation {} — {}", short_id(&id), target.reason);
    }
    Ok(())
}
