//! Interactive picker for disambiguation.

use crate::display::truncated_prefixes;
use crate::error::{JjjError, Result};
use crate::resolve::ResolveMatch;
use std::io::{self, IsTerminal, Write};

/// Pick one entity from multiple matches.
///
/// If stdout is a TTY, shows an interactive numbered list.
/// Otherwise, returns an error with suggestions.
pub fn pick_one(matches: &[ResolveMatch], entity_type: &str) -> Result<String> {
    if matches.is_empty() {
        return Err(JjjError::EntityNotFound(format!(
            "No {}s found",
            entity_type
        )));
    }

    if matches.len() == 1 {
        return Ok(matches[0].id.clone());
    }

    if std::io::stdout().is_terminal() {
        pick_interactive(matches, entity_type)
    } else {
        pick_non_interactive(matches, entity_type)
    }
}

fn pick_interactive(matches: &[ResolveMatch], entity_type: &str) -> Result<String> {
    let uuids: Vec<&str> = matches.iter().map(|m| m.id.as_str()).collect();
    let prefixes = truncated_prefixes(&uuids);

    eprintln!("Multiple {}s match. Select one:", entity_type);
    for (i, (m, (_, prefix))) in matches.iter().zip(prefixes.iter()).enumerate() {
        eprintln!("  {}. {}  {}", i + 1, prefix, m.title);
    }
    eprint!("Enter number (1-{}): ", matches.len());
    io::stderr().flush().map_err(JjjError::Io)?;

    let mut input = String::new();
    io::stdin().read_line(&mut input).map_err(JjjError::Io)?;

    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(JjjError::Cancelled("Selection cancelled".to_string()));
    }

    match trimmed.parse::<usize>() {
        Ok(n) if n >= 1 && n <= matches.len() => Ok(matches[n - 1].id.clone()),
        _ => Err(JjjError::Cancelled(format!(
            "Invalid selection: '{}'. Expected a number between 1 and {}.",
            trimmed,
            matches.len()
        ))),
    }
}

fn pick_non_interactive(matches: &[ResolveMatch], entity_type: &str) -> Result<String> {
    let uuids: Vec<&str> = matches.iter().map(|m| m.id.as_str()).collect();
    let prefixes = truncated_prefixes(&uuids);

    let mut msg = format!(
        "Multiple {}s match. Be more specific or use the short ID:\n",
        entity_type
    );

    let display_count = matches.len().min(10);
    for (m, (_, prefix)) in matches.iter().zip(prefixes.iter()).take(display_count) {
        msg.push_str(&format!("  {}  {}\n", prefix, m.title));
    }

    if matches.len() > 10 {
        msg.push_str(&format!("  ... and {} more\n", matches.len() - 10));
    }

    Err(JjjError::AmbiguousMatch(msg))
}
