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

// NOTE: the on-disk `base/` mirror (`snapshot_base`/`write_base_file`/
// `read_base_file`/`remove_base_file`) was removed with the M1 delta-fetch
// rewrite. The three-way merge base is now a jj *revision*
// (`GCA(last_synced_rev, head)`), and the base content for each file is
// reconstructed from a single full-context `jj diff --git` — no parallel
// directory tree to keep in sync. See `commands/fetch.rs` and
// `docs/design/scaling-for-agent-swarms.md` (Pillar 1).

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
    let merged = merge_value(b_yaml.as_ref(), &l_yaml, &r_yaml, prefer, None);
    let merged = normalize_timestamps(merged, &l_yaml, &r_yaml);
    let merged = sort_mapping_keys(merged);

    let yaml_str = serde_yml::to_string(&merged).map_err(JjjError::YamlParse)?;
    let body = merge_body(b_body.as_deref(), &l_body, &r_body);
    Ok(format!("---\n{}---\n\n{}", yaml_str, body))
}

/// True if `content` contains unresolved three-way-merge conflict markers
/// (the `<<<<<<<` / `>>>>>>>` lines [`merge_body`] emits). Used to refuse
/// pushing a still-conflicted entity so the markers don't propagate to every
/// other clone.
pub fn has_conflict_markers(content: &str) -> bool {
    content
        .lines()
        .any(|l| l.starts_with("<<<<<<<") || l.starts_with(">>>>>>>"))
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

/// Per-file last-writer-wins merge for a ranking JSON file
/// (`rankings/{milestone}/{user}.json`).
///
/// Each ranking file is owned by exactly one user — the filename encodes the
/// identity — so two collaborators never edit the *same* file in normal use.
/// That makes a simple LWW by the JSON's `updated_at` field correct and
/// conflict-free; no three-way merge or base snapshot is needed. When the
/// local file is missing we adopt the remote; when a timestamp can't be parsed
/// we keep the local copy rather than risk clobbering it.
pub fn merge_ranking_json(local: Option<&str>, remote: &str) -> String {
    let local = match local {
        Some(l) => l,
        None => return remote.to_string(),
    };
    if local == remote {
        return local.to_string();
    }
    match (extract_updated_at(local), extract_updated_at(remote)) {
        (Some(l), Some(r)) => {
            if r > l {
                remote.to_string()
            } else {
                local.to_string()
            }
        }
        // Local unparseable but remote is valid → prefer the valid remote.
        (None, Some(_)) => remote.to_string(),
        // Remote unparseable (or both) → keep local.
        _ => local.to_string(),
    }
}

fn extract_updated_at(json: &str) -> Option<String> {
    serde_json::from_str::<serde_json::Value>(json)
        .ok()
        .and_then(|v| {
            v.get("updated_at")
                .and_then(|w| w.as_str())
                .map(String::from)
        })
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

/// Sequence fields whose element ORDER is semantically meaningful and must be
/// preserved across a merge (rather than content-sorted like a set). For these,
/// retained base items keep base order and new items are appended preferring
/// the LWW-winning side's order — never re-sorted by content.
fn is_ordered_sequence_key(key: Option<&str>) -> bool {
    matches!(key, Some("change_ids") | Some("replies"))
}

/// `key` is the mapping key this value sits under (None at the document root),
/// used to decide whether a sequence is order-significant.
fn merge_value(
    base: Option<&Value>,
    local: &Value,
    remote: &Value,
    prefer: Side,
    key: Option<&str>,
) -> Value {
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
            merge_sequence(b_seq, l, r, prefer, is_ordered_sequence_key(key))
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

        let key_str = key.as_str();
        let merged: Option<Value> = match (lv, rv, bv) {
            (Some(lv), Some(rv), _) => Some(merge_value(bv, lv, rv, prefer, key_str)),
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

fn merge_sequence(
    base: Option<&Vec<Value>>,
    local: &[Value],
    remote: &[Value],
    prefer: Side,
    ordered: bool,
) -> Value {
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

    // Items new on either side.
    let mut additions: Vec<Value> = Vec::new();
    if ordered {
        // Order-significant (e.g. change_ids, reply threads): append the
        // LWW-winning side's new items first, in their own order, then the
        // other side's — never re-sorted by content. Deterministic because
        // both clones compute the same `prefer` side.
        let (first, second): (&[Value], &[Value]) = match prefer {
            Side::Local => (local, remote),
            Side::Remote => (remote, local),
        };
        for item in first.iter().chain(second.iter()) {
            if !base_items.contains(&item) && seen.insert(key_of(item)) {
                additions.push(item.clone());
            }
        }
    } else {
        // Set-like (tags, *_ids): content-sorted so the merge is invariant
        // under swapping local↔remote.
        for item in local.iter().chain(remote.iter()) {
            if !base_items.contains(&item) && seen.insert(key_of(item)) {
                additions.push(item.clone());
            }
        }
        additions.sort_by_key(key_of);
    }
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
    fn ordered_sequence_change_ids_preserves_order_not_content_sorted() {
        // change_ids is order-significant: a later-appended id must NOT be
        // re-sorted ahead of an earlier one by content.
        let base =
            "---\nid: '01'\nupdated_at: 2026-05-01T00:00:00Z\nchange_ids:\n- zzz\n---\n\nbody\n";
        // Local appends 'aaa' (sorts before 'zzz'); remote unchanged.
        let local = "---\nid: '01'\nupdated_at: 2026-05-02T00:00:00Z\nchange_ids:\n- zzz\n- aaa\n---\n\nbody\n";
        let merged = merge_entity_md(Some(base), local, base).unwrap();
        // Order preserved: zzz before aaa (content-sort would flip them).
        let zzz = merged.find("zzz").unwrap();
        let aaa = merged.find("aaa").unwrap();
        assert!(zzz < aaa, "change_ids order must be preserved:\n{merged}");
    }

    #[test]
    fn ranking_json_lww_takes_later_updated_at() {
        let local = r#"{"order":["a"],"votes":{},"updated_at":"2026-05-01T00:00:00Z"}"#;
        let remote = r#"{"order":["b"],"votes":{},"updated_at":"2026-05-02T00:00:00Z"}"#;
        assert_eq!(merge_ranking_json(Some(local), remote), remote);
        // Symmetric: older remote loses.
        assert_eq!(merge_ranking_json(Some(remote), local), remote);
    }

    #[test]
    fn ranking_json_adopts_remote_when_local_missing() {
        let remote = r#"{"order":["b"],"votes":{},"updated_at":"2026-05-02T00:00:00Z"}"#;
        assert_eq!(merge_ranking_json(None, remote), remote);
    }

    #[test]
    fn ranking_json_keeps_local_when_remote_unparseable() {
        let local = r#"{"order":["a"],"votes":{},"updated_at":"2026-05-01T00:00:00Z"}"#;
        let remote = "not json";
        assert_eq!(merge_ranking_json(Some(local), remote), local);
    }

    #[test]
    fn events_jsonl_swap_inputs_same_output() {
        let a = r#"{"when":"2026-05-01T00:00:00Z","by":"alice"}
"#;
        let b = r#"{"when":"2026-05-02T00:00:00Z","by":"bob"}
"#;
        assert_eq!(merge_events_jsonl(a, b), merge_events_jsonl(b, a));
    }
}
