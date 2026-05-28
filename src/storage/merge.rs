//! Deterministic three-way merge for jjj metadata files.
//!
//! Used by `jjj fetch` to reconcile concurrent edits without losing data.
//! Two operations are provided:
//!
//! - [`merge_events_jsonl`] — line-union dedup for the append-only event log.
//! - [`merge_entity_md`] — three-way merge for problems / solutions / critiques /
//!   milestones markdown files.
//!
//! The entity merger parses YAML frontmatter into a generic
//! [`serde_yml::Value`] tree and walks it recursively:
//!
//! - **Sequences** (`solution_ids`, `tags`, …) → set union, preserving items
//!   that were in the base and items added on either side. Items removed on
//!   one side and unchanged on the other are dropped.
//! - **Mappings** → recursively merged key-by-key.
//! - **Scalars** that diverged on both sides → the side with the later
//!   `updated_at` wins (lexicographic tiebreak on the serialized value).
//! - `created_at` / `updated_at` are always normalized at the end to
//!   min / max across the two sides.
//!
//! The markdown body is handled by [`merge_body`]: clean LWW when only one
//! side changed; if both changed, the whole body is wrapped in standard
//! `<<<<<<<` / `>>>>>>>` conflict markers. (Hunk-level line merging is a
//! future improvement.)
//!
//! Output YAML keys are sorted alphabetically — every clone produces the
//! same canonical byte sequence given the same three inputs.

use crate::error::{JjjError, Result};
use serde_yml::{Mapping, Value};
use std::collections::HashSet;
use std::fs;
use std::path::Path;

use super::ENTITY_DIRS;

/// Mirror the current contents of `meta_path` into `base_path` for every
/// entity file, replacing whatever was previously in `base_path`.
///
/// Called after a successful push (the just-pushed state becomes the merge
/// ancestor for the next fetch) and after a fetch has reconciled remote
/// changes into the local working set.
pub fn snapshot_base(meta_path: &Path, base_path: &Path) -> Result<()> {
    if base_path.exists() {
        // Wipe entity dirs to handle deletions; leave anything else alone.
        for dir in ENTITY_DIRS {
            let p = base_path.join(dir);
            if p.exists() {
                fs::remove_dir_all(&p)?;
            }
        }
    }
    for dir in ENTITY_DIRS {
        let src = meta_path.join(dir);
        if !src.exists() {
            continue;
        }
        let dst = base_path.join(dir);
        fs::create_dir_all(&dst)?;
        for entry in fs::read_dir(&src)?.flatten() {
            let file_name = entry.file_name();
            let dst_file = dst.join(&file_name);
            fs::copy(entry.path(), &dst_file)?;
        }
    }
    Ok(())
}

/// Write a single file into the base snapshot at the given path relative to
/// the entity-meta root (e.g. `problems/01.md`). Used by fetch to advance
/// the ancestor as each remote file is reconciled.
pub fn write_base_file(base_path: &Path, relative: &Path, content: &str) -> Result<()> {
    let dst = base_path.join(relative);
    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&dst, content)?;
    Ok(())
}

/// Read the base copy of an entity file, if it exists.
pub fn read_base_file(base_path: &Path, relative: &Path) -> Option<String> {
    fs::read_to_string(base_path.join(relative)).ok()
}

/// Three-way merge for an entity markdown file.
///
/// `base` is the version both sides shared before diverging (the last fetched
/// or pushed content). It may be `None` if no base exists yet — in that case
/// the merger treats both sides as having diverged from an empty state.
pub fn merge_entity_md(base: Option<&str>, local: &str, remote: &str) -> Result<String> {
    if local == remote {
        return Ok(local.to_string());
    }
    if let Some(b) = base {
        if local == b {
            return Ok(remote.to_string());
        }
        if remote == b {
            return Ok(local.to_string());
        }
    }

    let (l_yaml, l_body) = split_md(local)?;
    let (r_yaml, r_body) = split_md(remote)?;
    let base_split = base.map(split_md).transpose()?;
    let (b_yaml, b_body) = match base_split {
        Some((y, b)) => (Some(y), Some(b)),
        None => (None, None),
    };

    let prefer = pick_side(&l_yaml, &r_yaml);
    let merged = merge_value(b_yaml.as_ref(), &l_yaml, &r_yaml, prefer);
    let merged = normalize_timestamps(merged, &l_yaml, &r_yaml);
    let merged = sort_mapping_keys(merged);

    let yaml_str = serde_yml::to_string(&merged).map_err(JjjError::YamlParse)?;
    let body = merge_body(b_body.as_deref(), &l_body, &r_body);
    Ok(format!("---\n{}---\n\n{}", yaml_str, body))
}

/// Two-way union of events.jsonl-style files. Lines are deduped exactly.
/// Output is sorted by the `when` field of each event (ascending), with
/// tiebreak by raw line bytes for determinism. Unparseable lines are kept
/// and sorted to the end.
pub fn merge_events_jsonl(local: &str, remote: &str) -> String {
    let mut seen: HashSet<&str> = HashSet::new();
    let mut lines: Vec<&str> = Vec::new();
    for src in [local, remote] {
        for line in src.lines() {
            let trimmed = line.trim_end();
            if trimmed.is_empty() {
                continue;
            }
            if seen.insert(trimmed) {
                lines.push(trimmed);
            }
        }
    }
    lines.sort_by(|a, b| {
        let when_a = extract_when(a);
        let when_b = extract_when(b);
        when_a.cmp(&when_b).then_with(|| a.cmp(b))
    });
    let mut out = lines.join("\n");
    if !out.is_empty() {
        out.push('\n');
    }
    out
}

fn extract_when(line: &str) -> Option<String> {
    serde_json::from_str::<serde_json::Value>(line)
        .ok()
        .and_then(|v| v.get("when").and_then(|w| w.as_str()).map(String::from))
}

#[derive(Clone, Copy, Debug)]
enum Side {
    Local,
    Remote,
}

fn pick_side(l: &Value, r: &Value) -> Side {
    let l_ts = l.get("updated_at").and_then(|v| v.as_str()).unwrap_or("");
    let r_ts = r.get("updated_at").and_then(|v| v.as_str()).unwrap_or("");
    match r_ts.cmp(l_ts) {
        std::cmp::Ordering::Greater => Side::Remote,
        std::cmp::Ordering::Less => Side::Local,
        std::cmp::Ordering::Equal => {
            // Stable, deterministic tiebreak by full-document serialization.
            let l_repr = serde_yml::to_string(l).unwrap_or_default();
            let r_repr = serde_yml::to_string(r).unwrap_or_default();
            if r_repr > l_repr {
                Side::Remote
            } else {
                Side::Local
            }
        }
    }
}

fn split_md(content: &str) -> Result<(Value, String)> {
    let content = content.trim_start();
    if !content.starts_with("---") {
        return Err(JjjError::FrontmatterParse {
            entity_type: String::new(),
            entity_id: String::new(),
            message: "merge: missing leading frontmatter delimiter".to_string(),
        });
    }
    let rest = &content[3..];
    let end = rest
        .find("\n---")
        .ok_or_else(|| JjjError::FrontmatterParse {
            entity_type: String::new(),
            entity_id: String::new(),
            message: "merge: missing closing frontmatter delimiter".to_string(),
        })?;
    let yaml_str = rest[..end].trim();
    let body = rest[end + 4..].trim_start_matches('\n').to_string();
    let yaml: Value = serde_yml::from_str(yaml_str).map_err(JjjError::YamlParse)?;
    Ok((yaml, body))
}

fn merge_value(base: Option<&Value>, local: &Value, remote: &Value, prefer: Side) -> Value {
    if local == remote {
        return local.clone();
    }
    if let Some(b) = base {
        if local == b {
            return remote.clone();
        }
        if remote == b {
            return local.clone();
        }
    }
    match (local, remote) {
        (Value::Mapping(l), Value::Mapping(r)) => {
            let b_map = base.and_then(|b| match b {
                Value::Mapping(m) => Some(m),
                _ => None,
            });
            merge_mapping(b_map, l, r, prefer)
        }
        (Value::Sequence(l), Value::Sequence(r)) => {
            let b_seq = base.and_then(|b| match b {
                Value::Sequence(s) => Some(s),
                _ => None,
            });
            merge_sequence(b_seq, l, r)
        }
        _ => match prefer {
            Side::Local => local.clone(),
            Side::Remote => remote.clone(),
        },
    }
}

fn merge_mapping(base: Option<&Mapping>, local: &Mapping, remote: &Mapping, prefer: Side) -> Value {
    let mut out = Mapping::new();
    let mut keys: Vec<Value> = local.keys().chain(remote.keys()).cloned().collect();
    keys.sort_by_key(|k| serde_yml::to_string(k).unwrap_or_default());
    keys.dedup();

    for key in keys {
        let lv = local.get(&key);
        let rv = remote.get(&key);
        let bv = base.and_then(|m| m.get(&key));

        let merged: Option<Value> = match (lv, rv, bv) {
            (Some(lv), Some(rv), _) => Some(merge_value(bv, lv, rv, prefer)),
            (Some(lv), None, None) => Some(lv.clone()), // new on local
            (None, Some(rv), None) => Some(rv.clone()), // new on remote
            (Some(lv), None, Some(bv)) if lv == bv => None, // remote deleted, local unchanged
            (Some(lv), None, Some(_)) => Some(lv.clone()), // remote deleted, local edited → keep
            (None, Some(rv), Some(bv)) if rv == bv => None, // local deleted, remote unchanged
            (None, Some(rv), Some(_)) => Some(rv.clone()), // local deleted, remote edited → keep
            (None, None, _) => None,
        };

        if let Some(v) = merged {
            out.insert(key, v);
        }
    }
    Value::Mapping(out)
}

fn merge_sequence(base: Option<&Vec<Value>>, local: &[Value], remote: &[Value]) -> Value {
    let key_of = |v: &Value| serde_yml::to_string(v).unwrap_or_default();
    let base_items: Vec<&Value> = base.map(|s| s.iter().collect()).unwrap_or_default();
    let local_contains = |v: &Value| local.iter().any(|x| x == v);
    let remote_contains = |v: &Value| remote.iter().any(|x| x == v);

    let mut out: Vec<Value> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();

    // Items retained by both sides keep their original base ordering.
    for item in &base_items {
        if local_contains(item) && remote_contains(item) && seen.insert(key_of(item)) {
            out.push((*item).clone());
        }
    }

    // Items new on either side are appended in a content-sorted order so the
    // merge is invariant under swapping local↔remote.
    let mut additions: Vec<Value> = Vec::new();
    for item in local.iter().chain(remote.iter()) {
        if !base_items.contains(&item) && seen.insert(key_of(item)) {
            additions.push(item.clone());
        }
    }
    additions.sort_by_key(key_of);
    out.extend(additions);

    Value::Sequence(out)
}

/// Force `created_at` to min(local, remote) and `updated_at` to max(local, remote)
/// so timestamp drift never breaks monotonicity.
fn normalize_timestamps(mut merged: Value, local: &Value, remote: &Value) -> Value {
    if let Value::Mapping(ref mut m) = merged {
        if let (Some(l), Some(r)) = (
            local.get("created_at").and_then(|v| v.as_str()),
            remote.get("created_at").and_then(|v| v.as_str()),
        ) {
            let earliest = if l <= r { l } else { r };
            m.insert(
                Value::String("created_at".to_string()),
                Value::String(earliest.to_string()),
            );
        }
        if let (Some(l), Some(r)) = (
            local.get("updated_at").and_then(|v| v.as_str()),
            remote.get("updated_at").and_then(|v| v.as_str()),
        ) {
            let latest = if l >= r { l } else { r };
            m.insert(
                Value::String("updated_at".to_string()),
                Value::String(latest.to_string()),
            );
        }
    }
    merged
}

fn sort_mapping_keys(v: Value) -> Value {
    match v {
        Value::Mapping(m) => {
            let mut entries: Vec<(Value, Value)> = m.into_iter().collect();
            entries.sort_by_key(|(k, _)| serde_yml::to_string(k).unwrap_or_default());
            let mut sorted = Mapping::new();
            for (k, val) in entries {
                sorted.insert(k, sort_mapping_keys(val));
            }
            Value::Mapping(sorted)
        }
        Value::Sequence(s) => Value::Sequence(s.into_iter().map(sort_mapping_keys).collect()),
        other => other,
    }
}

fn merge_body(base: Option<&str>, local: &str, remote: &str) -> String {
    if local == remote {
        return ensure_trailing_newline(local);
    }
    if let Some(b) = base {
        if local == b {
            return ensure_trailing_newline(remote);
        }
        if remote == b {
            return ensure_trailing_newline(local);
        }
    }
    // Both sides diverged from base — wrap the entire body in conflict markers
    // and let the user resolve. Hunk-level merging is a follow-up.
    format!(
        "<<<<<<< local\n{}\n=======\n{}\n>>>>>>> remote\n",
        local.trim_end(),
        remote.trim_end()
    )
}

fn ensure_trailing_newline(s: &str) -> String {
    if s.ends_with('\n') {
        s.to_string()
    } else {
        format!("{}\n", s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BASE: &str = "---\n\
id: 01\n\
title: Original\n\
status: open\n\
priority: medium\n\
created_at: 2026-05-01T00:00:00Z\n\
updated_at: 2026-05-01T00:00:00Z\n\
tags:\n\
- backend\n\
---\n\n\
Original body.\n";

    fn rewrite(input: &str, replacements: &[(&str, &str)]) -> String {
        let mut s = input.to_string();
        for (from, to) in replacements {
            s = s.replace(from, to);
        }
        s
    }

    #[test]
    fn identical_local_and_remote_returns_either() {
        let merged = merge_entity_md(Some(BASE), BASE, BASE).unwrap();
        assert!(merged.contains("title: Original"));
    }

    #[test]
    fn only_local_changed_takes_local() {
        let local = rewrite(BASE, &[("title: Original", "title: Alice's title")]);
        let merged = merge_entity_md(Some(BASE), &local, BASE).unwrap();
        assert!(merged.contains("title: Alice's title"));
    }

    #[test]
    fn only_remote_changed_takes_remote() {
        let remote = rewrite(BASE, &[("title: Original", "title: Bob's title")]);
        let merged = merge_entity_md(Some(BASE), BASE, &remote).unwrap();
        assert!(merged.contains("title: Bob's title"));
    }

    #[test]
    fn tags_added_on_both_sides_unioned() {
        let local = rewrite(
            BASE,
            &[("tags:\n- backend\n", "tags:\n- backend\n- urgent\n")],
        );
        let remote = rewrite(
            BASE,
            &[("tags:\n- backend\n", "tags:\n- backend\n- frontend\n")],
        );
        let merged = merge_entity_md(Some(BASE), &local, &remote).unwrap();
        assert!(merged.contains("backend"));
        assert!(merged.contains("urgent"));
        assert!(merged.contains("frontend"));
    }

    #[test]
    fn scalar_conflict_resolves_by_later_updated_at() {
        // Local updated at 02; remote updated at 03 → remote's title wins
        let local = rewrite(
            BASE,
            &[
                ("title: Original", "title: Alice"),
                (
                    "updated_at: 2026-05-01T00:00:00Z",
                    "updated_at: 2026-05-02T00:00:00Z",
                ),
            ],
        );
        let remote = rewrite(
            BASE,
            &[
                ("title: Original", "title: Bob"),
                (
                    "updated_at: 2026-05-01T00:00:00Z",
                    "updated_at: 2026-05-03T00:00:00Z",
                ),
            ],
        );
        let merged = merge_entity_md(Some(BASE), &local, &remote).unwrap();
        assert!(merged.contains("title: Bob"), "merged was:\n{}", merged);
        // updated_at must be the max — serde_yml quotes date-like strings.
        assert!(
            merged.contains("2026-05-03T00:00:00Z"),
            "expected 2026-05-03 timestamp in merged output:\n{}",
            merged
        );
        assert!(!merged.contains("2026-05-02T00:00:00Z"));
    }

    #[test]
    fn body_lww_when_only_one_side_changed() {
        let local = rewrite(BASE, &[("Original body.", "Alice edited body.")]);
        let merged = merge_entity_md(Some(BASE), &local, BASE).unwrap();
        assert!(merged.contains("Alice edited body."));
        assert!(!merged.contains("Original body."));
    }

    #[test]
    fn body_both_changed_emits_conflict_markers() {
        let local = rewrite(BASE, &[("Original body.", "Alice version.")]);
        let remote = rewrite(BASE, &[("Original body.", "Bob version.")]);
        let merged = merge_entity_md(Some(BASE), &local, &remote).unwrap();
        assert!(merged.contains("<<<<<<< local"));
        assert!(merged.contains("Alice version."));
        assert!(merged.contains("======="));
        assert!(merged.contains("Bob version."));
        assert!(merged.contains(">>>>>>> remote"));
    }

    #[test]
    fn no_base_treats_both_as_diverged() {
        let local = rewrite(BASE, &[("title: Original", "title: Alice")]);
        let remote = rewrite(BASE, &[("title: Original", "title: Bob")]);
        // Without base: scalar conflict picked by updated_at tiebreak (equal → lex)
        let merged = merge_entity_md(None, &local, &remote).unwrap();
        // Deterministic: same inputs always produce same output
        let merged2 = merge_entity_md(None, &local, &remote).unwrap();
        assert_eq!(merged, merged2);
    }

    #[test]
    fn output_is_byte_identical_regardless_of_argument_order() {
        // Swapping local and remote should produce identical output for the
        // (tags-union, no scalar conflicts) case.
        let local = rewrite(
            BASE,
            &[("tags:\n- backend\n", "tags:\n- backend\n- urgent\n")],
        );
        let remote = rewrite(
            BASE,
            &[("tags:\n- backend\n", "tags:\n- backend\n- frontend\n")],
        );
        let m1 = merge_entity_md(Some(BASE), &local, &remote).unwrap();
        let m2 = merge_entity_md(Some(BASE), &remote, &local).unwrap();
        assert_eq!(m1, m2);
    }

    #[test]
    fn events_jsonl_union_dedups_and_sorts() {
        let local = r#"{"when":"2026-05-01T00:00:00Z","type":"problem_created","entity":"01","by":"alice"}
{"when":"2026-05-02T00:00:00Z","type":"solution_created","entity":"02","by":"alice"}
"#;
        let remote = r#"{"when":"2026-05-01T12:00:00Z","type":"problem_created","entity":"03","by":"bob"}
{"when":"2026-05-02T00:00:00Z","type":"solution_created","entity":"02","by":"alice"}
"#;
        let merged = merge_events_jsonl(local, remote);
        // 3 unique lines (the duplicate "02 alice" appears once)
        assert_eq!(merged.lines().count(), 3);
        // Sorted ascending by when
        let lines: Vec<&str> = merged.lines().collect();
        assert!(lines[0].contains("2026-05-01T00:00:00Z"));
        assert!(lines[1].contains("2026-05-01T12:00:00Z"));
        assert!(lines[2].contains("2026-05-02T00:00:00Z"));
    }

    #[test]
    fn events_jsonl_swap_inputs_same_output() {
        let a = r#"{"when":"2026-05-01T00:00:00Z","by":"alice"}
"#;
        let b = r#"{"when":"2026-05-02T00:00:00Z","by":"bob"}
"#;
        assert_eq!(merge_events_jsonl(a, b), merge_events_jsonl(b, a));
    }

    #[test]
    fn snapshot_base_mirrors_entity_dirs() {
        let tmp = tempfile::tempdir().unwrap();
        let meta = tmp.path().join("meta");
        let base = tmp.path().join("base");
        fs::create_dir_all(meta.join("problems")).unwrap();
        fs::create_dir_all(meta.join("solutions")).unwrap();
        fs::write(meta.join("problems/01.md"), "alpha").unwrap();
        fs::write(meta.join("solutions/02.md"), "beta").unwrap();

        snapshot_base(&meta, &base).unwrap();

        assert_eq!(
            fs::read_to_string(base.join("problems/01.md")).unwrap(),
            "alpha"
        );
        assert_eq!(
            fs::read_to_string(base.join("solutions/02.md")).unwrap(),
            "beta"
        );
    }

    #[test]
    fn snapshot_base_handles_deletions_in_entity_dirs() {
        let tmp = tempfile::tempdir().unwrap();
        let meta = tmp.path().join("meta");
        let base = tmp.path().join("base");
        fs::create_dir_all(meta.join("problems")).unwrap();
        fs::create_dir_all(base.join("problems")).unwrap();
        // base had a stale file that no longer exists in meta
        fs::write(base.join("problems/STALE.md"), "old").unwrap();
        fs::write(meta.join("problems/01.md"), "fresh").unwrap();

        snapshot_base(&meta, &base).unwrap();

        assert!(!base.join("problems/STALE.md").exists());
        assert_eq!(
            fs::read_to_string(base.join("problems/01.md")).unwrap(),
            "fresh"
        );
    }

    #[test]
    fn write_and_read_base_file_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let base = tmp.path().to_path_buf();
        let relative = Path::new("problems/01.md");
        write_base_file(&base, relative, "hello").unwrap();
        assert_eq!(read_base_file(&base, relative).as_deref(), Some("hello"));
        assert_eq!(
            read_base_file(&base, Path::new("problems/missing.md")),
            None
        );
    }
}
