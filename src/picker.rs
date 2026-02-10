//! Interactive picker for disambiguation.

use crate::display::truncated_prefixes;
use crate::error::{JjjError, Result};
use crate::resolve::ResolveMatch;
use dialoguer::{theme::ColorfulTheme, FuzzySelect};
use std::io::IsTerminal;

/// Pick one entity from multiple matches.
///
/// If stdout is a TTY, shows an interactive picker.
/// Otherwise, returns an error with suggestions.
pub fn pick_one(matches: &[ResolveMatch], entity_type: &str) -> Result<String> {
    if matches.is_empty() {
        return Err(JjjError::EntityNotFound(format!("No {}s found", entity_type)));
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
    // Calculate truncated prefixes for display
    let uuids: Vec<&str> = matches.iter().map(|m| m.id.as_str()).collect();
    let prefixes = truncated_prefixes(&uuids);

    // Build display strings
    let items: Vec<String> = matches
        .iter()
        .zip(prefixes.iter())
        .map(|(m, (_, prefix))| format!("{}  {}", prefix, m.title))
        .collect();

    let selection = FuzzySelect::with_theme(&ColorfulTheme::default())
        .with_prompt(format!("Select {}:", entity_type))
        .items(&items)
        .default(0)
        .interact_opt()
        .map_err(|e| JjjError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;

    match selection {
        Some(index) => Ok(matches[index].id.clone()),
        None => Err(JjjError::Cancelled("Selection cancelled".to_string())),
    }
}

fn pick_non_interactive(matches: &[ResolveMatch], entity_type: &str) -> Result<String> {
    // Calculate truncated prefixes for display
    let uuids: Vec<&str> = matches.iter().map(|m| m.id.as_str()).collect();
    let prefixes = truncated_prefixes(&uuids);

    let mut msg = format!("Multiple {}s match. Be more specific or use the short ID:\n", entity_type);

    let display_count = matches.len().min(10);
    for (m, (_, prefix)) in matches.iter().zip(prefixes.iter()).take(display_count) {
        msg.push_str(&format!("  {}  {}\n", prefix, m.title));
    }

    if matches.len() > 10 {
        msg.push_str(&format!("  ... and {} more\n", matches.len() - 10));
    }

    Err(JjjError::AmbiguousMatch(msg))
}
