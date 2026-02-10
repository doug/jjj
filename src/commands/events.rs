use crate::cli::EventsAction;
use crate::error::Result;
use crate::jj::JjClient;
use crate::storage::MetadataStore;
use chrono::{NaiveDate, TimeZone, Utc};

pub fn execute(
    action: Option<EventsAction>,
    from: Option<String>,
    to: Option<String>,
    problem: Option<String>,
    solution: Option<String>,
    event_type: Option<String>,
    search: Option<String>,
    json: bool,
    limit: usize,
) -> Result<()> {
    match action {
        Some(EventsAction::Rebuild) => rebuild_events(),
        Some(EventsAction::Validate) => validate_events(),
        None => list_events(from, to, problem, solution, event_type, search, json, limit),
    }
}

fn list_events(
    from: Option<String>,
    to: Option<String>,
    problem: Option<String>,
    solution: Option<String>,
    event_type: Option<String>,
    search: Option<String>,
    json: bool,
    limit: usize,
) -> Result<()> {
    let jj_client = JjClient::new()?;
    let store = MetadataStore::new(jj_client)?;

    let mut events = store.list_events()?;

    // Parse date filters
    let from_date = from.as_ref().and_then(|s| parse_date_filter(s));
    let to_date = to.as_ref().and_then(|s| parse_date_filter(s));

    // Apply filters
    events.retain(|e| {
        // Date filters
        if let Some(ref fd) = from_date {
            if e.when < *fd {
                return false;
            }
        }
        if let Some(ref td) = to_date {
            if e.when > *td {
                return false;
            }
        }

        // Entity filters
        if let Some(ref p) = problem {
            if !e.entity.starts_with('p') || e.entity != *p {
                if !e.refs.contains(p) {
                    return false;
                }
            }
        }
        if let Some(ref s) = solution {
            if !e.entity.starts_with('s') || e.entity != *s {
                if !e.refs.contains(s) {
                    return false;
                }
            }
        }

        // Type filter
        if let Some(ref t) = event_type {
            if !e.event_type.to_string().contains(t) {
                return false;
            }
        }

        // Search filter
        if let Some(ref q) = search {
            let q_lower = q.to_lowercase();
            let matches = e.rationale.as_ref()
                .map(|r| r.to_lowercase().contains(&q_lower))
                .unwrap_or(false);
            if !matches {
                return false;
            }
        }

        true
    });

    // Reverse to show most recent first, then limit
    events.reverse();
    events.truncate(limit);

    if json {
        println!("{}", serde_json::to_string_pretty(&events)?);
        return Ok(());
    }

    if events.is_empty() {
        println!("No events found");
        return Ok(());
    }

    for event in &events {
        let date = event.when.format("%Y-%m-%d %H:%M");
        let rationale = event.rationale.as_ref()
            .map(|r| format!(" - {}", truncate(r, 50)))
            .unwrap_or_default();

        println!("{} {:20} {:8} {}{}",
            date,
            event.event_type.to_string(),
            event.entity,
            event.by,
            rationale
        );
    }

    Ok(())
}

fn parse_date_filter(s: &str) -> Option<chrono::DateTime<Utc>> {
    // Try YYYY-MM-DD
    if let Ok(date) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        return Some(Utc.from_utc_datetime(&date.and_hms_opt(0, 0, 0).unwrap()));
    }
    // Try YYYY-MM (first of month)
    if let Ok(date) = NaiveDate::parse_from_str(&format!("{}-01", s), "%Y-%m-%d") {
        return Some(Utc.from_utc_datetime(&date.and_hms_opt(0, 0, 0).unwrap()));
    }
    None
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max-3])
    }
}

fn rebuild_events() -> Result<()> {
    println!("Rebuild not yet implemented");
    // TODO: Parse commit history for jjj: lines
    Ok(())
}

fn validate_events() -> Result<()> {
    println!("Validate not yet implemented");
    // TODO: Cross-check events with entity states
    Ok(())
}
