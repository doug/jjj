use crate::error::Result;
use std::io::{self, Write};

/// Prompt the user for input
pub fn prompt(message: &str) -> Result<String> {
    print!("{}", message);
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    Ok(input.trim().to_string())
}

/// Prompt the user for confirmation (y/n)
pub fn confirm(message: &str) -> Result<bool> {
    loop {
        let input = prompt(&format!("{} (y/n): ", message))?;
        match input.to_lowercase().as_str() {
            "y" | "yes" => return Ok(true),
            "n" | "no" => return Ok(false),
            _ => println!("Please enter 'y' or 'n'"),
        }
    }
}

/// Format a change ID for display (truncate to 7 chars)
pub fn format_change_id(change_id: &str) -> String {
    if change_id.len() > 7 {
        format!("{}...", &change_id[..7])
    } else {
        change_id.to_string()
    }
}

/// Parse a user mention (e.g., "@alice" -> "alice")
pub fn parse_mention(mention: &str) -> String {
    mention.trim_start_matches('@').to_string()
}

/// Format a relative time (e.g., "2 hours ago")
pub fn format_relative_time(timestamp: &chrono::DateTime<chrono::Utc>) -> String {
    let now = chrono::Utc::now();
    let duration = now.signed_duration_since(*timestamp);

    if duration.num_seconds() < 60 {
        "just now".to_string()
    } else if duration.num_minutes() < 60 {
        let mins = duration.num_minutes();
        if mins == 1 {
            "1 minute ago".to_string()
        } else {
            format!("{} minutes ago", mins)
        }
    } else if duration.num_hours() < 24 {
        let hours = duration.num_hours();
        if hours == 1 {
            "1 hour ago".to_string()
        } else {
            format!("{} hours ago", hours)
        }
    } else if duration.num_days() < 7 {
        let days = duration.num_days();
        if days == 1 {
            "1 day ago".to_string()
        } else {
            format!("{} days ago", days)
        }
    } else if duration.num_weeks() < 4 {
        let weeks = duration.num_weeks();
        if weeks == 1 {
            "1 week ago".to_string()
        } else {
            format!("{} weeks ago", weeks)
        }
    } else {
        let months = duration.num_days() / 30;
        if months == 1 {
            "1 month ago".to_string()
        } else if months < 12 {
            format!("{} months ago", months)
        } else {
            let years = duration.num_days() / 365;
            if years == 1 {
                "1 year ago".to_string()
            } else {
                format!("{} years ago", years)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_change_id() {
        assert_eq!(format_change_id("abc123"), "abc123");
        assert_eq!(format_change_id("abc123def456"), "abc123d...");
    }

    #[test]
    fn test_parse_mention() {
        assert_eq!(parse_mention("@alice"), "alice");
        assert_eq!(parse_mention("alice"), "alice");
    }
}

/// Truncate a string to a maximum length, appending "..." if truncated.
pub fn truncate(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_len.saturating_sub(3)).collect();
        format!("{}...", truncated)
    }
}
