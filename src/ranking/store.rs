use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

use crate::error::Result;
use crate::ranking::glicko2::Comparison;

const RANKINGS_DIR: &str = "rankings";

/// Convert a user identity like "Alice Smith <alice@example.com>" to a
/// filename-safe slug like "alice-smith".
///
/// Takes the name part before `<`, lowercases it, replaces non-alphanumeric
/// characters with `-`, and trims leading/trailing `-`.
pub fn sanitize_user(user: &str) -> String {
    let name_part = if let Some(idx) = user.find('<') {
        &user[..idx]
    } else {
        user
    };

    let slug: String = name_part
        .trim()
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect();

    slug.trim_matches('-').to_string()
}

/// Append one comparison as a JSON line to
/// `{base}/rankings/{milestone_id}/{sanitize_user(user)}.jsonl`.
///
/// Creates directories as needed. Opens the file in append mode.
pub fn append_comparison(
    base: &Path,
    milestone_id: &str,
    user: &str,
    comparison: &Comparison,
) -> Result<()> {
    let dir = base.join(RANKINGS_DIR).join(milestone_id);
    fs::create_dir_all(&dir)?;

    let file_path = dir.join(format!("{}.jsonl", sanitize_user(user)));
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(file_path)?;

    let line = serde_json::to_string(comparison)?;
    writeln!(file, "{line}")?;

    Ok(())
}

/// Load all comparisons for a milestone from ALL user files in
/// `{base}/rankings/{milestone_id}/`.
///
/// Returns comparisons sorted by timestamp. Returns an empty vec if the
/// directory does not exist.
pub fn load_comparisons(base: &Path, milestone_id: &str) -> Result<Vec<Comparison>> {
    let attributed = load_attributed_comparisons(base, milestone_id)?;
    let comparisons: Vec<Comparison> = attributed.into_iter().map(|(c, _)| c).collect();
    Ok(comparisons)
}

/// Load all comparisons for a milestone with user attribution.
///
/// Returns `(Comparison, user_slug)` pairs sorted by timestamp. Returns an
/// empty vec if the directory does not exist.
pub fn load_attributed_comparisons(
    base: &Path,
    milestone_id: &str,
) -> Result<Vec<(Comparison, String)>> {
    let dir = base.join(RANKINGS_DIR).join(milestone_id);

    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut results: Vec<(Comparison, String)> = Vec::new();

    for entry in fs::read_dir(&dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
            continue;
        }

        let user_slug = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();

        let file = fs::File::open(&path)?;
        let reader = BufReader::new(file);

        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            let comparison: Comparison = serde_json::from_str(&line)?;
            results.push((comparison, user_slug.clone()));
        }
    }

    results.sort_by_key(|(c, _)| c.ts);

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use tempfile::TempDir;

    #[test]
    fn test_sanitize_user() {
        assert_eq!(
            sanitize_user("Alice Smith <alice@test.com>"),
            "alice-smith"
        );
        assert_eq!(sanitize_user("bob"), "bob");
    }

    #[test]
    fn test_append_and_load_comparisons() {
        let tmp = TempDir::new().unwrap();
        let base = tmp.path();
        let milestone = "m1";

        let c1 = Comparison {
            winner: "A".to_string(),
            loser: "B".to_string(),
            ts: Utc::now(),
        };
        let c2 = Comparison {
            winner: "B".to_string(),
            loser: "C".to_string(),
            ts: Utc::now(),
        };

        append_comparison(base, milestone, "alice", &c1).unwrap();
        append_comparison(base, milestone, "alice", &c2).unwrap();

        let loaded = load_comparisons(base, milestone).unwrap();
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].winner, "A");
        assert_eq!(loaded[1].winner, "B");
    }

    #[test]
    fn test_load_comparisons_empty() {
        let tmp = TempDir::new().unwrap();
        let loaded = load_comparisons(tmp.path(), "nonexistent").unwrap();
        assert!(loaded.is_empty());
    }

    #[test]
    fn test_load_comparisons_multiple_users() {
        let tmp = TempDir::new().unwrap();
        let base = tmp.path();
        let milestone = "m2";

        let c1 = Comparison {
            winner: "X".to_string(),
            loser: "Y".to_string(),
            ts: Utc::now(),
        };
        let c2 = Comparison {
            winner: "Y".to_string(),
            loser: "Z".to_string(),
            ts: Utc::now(),
        };

        append_comparison(base, milestone, "alice", &c1).unwrap();
        append_comparison(base, milestone, "bob", &c2).unwrap();

        let loaded = load_comparisons(base, milestone).unwrap();
        assert_eq!(loaded.len(), 2);
    }

    #[test]
    fn test_load_attributed_comparisons() {
        let tmp = TempDir::new().unwrap();
        let base = tmp.path();
        let milestone = "m3";

        let c1 = Comparison {
            winner: "A".to_string(),
            loser: "B".to_string(),
            ts: Utc::now(),
        };
        let c2 = Comparison {
            winner: "C".to_string(),
            loser: "D".to_string(),
            ts: Utc::now(),
        };

        append_comparison(base, milestone, "Alice Smith <alice@test.com>", &c1).unwrap();
        append_comparison(base, milestone, "bob", &c2).unwrap();

        let loaded = load_attributed_comparisons(base, milestone).unwrap();
        assert_eq!(loaded.len(), 2);

        let users: Vec<&str> = loaded.iter().map(|(_, u)| u.as_str()).collect();
        assert!(users.contains(&"alice-smith"));
        assert!(users.contains(&"bob"));
    }
}
