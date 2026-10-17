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
//! [`serde_norway::Value`] tree and walks it recursively:
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
use serde_norway::{Mapping, Value};
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
    let merged = normalize_claim(merged, &l_yaml, &r_yaml);
    let merged = normalize_status_dependents(merged);
    let merged = sort_mapping_keys(merged);

    let yaml_str = serde_norway::to_string(&merged).map_err(JjjError::YamlParse)?;
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

/// Which side of a conflict block to keep when resolving.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictSide {
    /// The local (`<<<<<<< local` … `=======`) hunk — this clone's edit.
    Local,
    /// The remote (`=======` … `>>>>>>> remote`) hunk — the fetched edit.
    Remote,
}

/// Deterministically resolve every conflict block in `content` by keeping one
/// side, dropping the marker lines. The inverse of [`merge_body`]'s wrapping:
/// each `<<<<<<< / ======= / >>>>>>>` run collapses to the chosen hunk so an
/// agent can resolve non-interactively (`jjj resolve <id> --ours/--theirs`)
/// instead of hand-editing markers. Text outside conflict blocks is preserved
/// verbatim. Idempotent on marker-free input (returns it unchanged).
pub fn resolve_conflict_markers(content: &str, side: ConflictSide) -> String {
    let mut out = String::new();
    // States while walking blocks: outside any block, inside the local hunk
    // (before `=======`), inside the remote hunk (after it).
    let mut in_local = false;
    let mut in_remote = false;
    for line in content.lines() {
        if line.starts_with("<<<<<<<") {
            in_local = true;
            in_remote = false;
            continue;
        }
        if in_local && line.starts_with("=======") {
            in_local = false;
            in_remote = true;
            continue;
        }
        if in_remote && line.starts_with(">>>>>>>") {
            in_remote = false;
            continue;
        }
        let keep = if in_local {
            side == ConflictSide::Local
        } else if in_remote {
            side == ConflictSide::Remote
        } else {
            true
        };
        if keep {
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}

/// Append-only union for a single-writer event shard (`events/{user}.jsonl`,
/// Pillar 3). Local lines are preserved verbatim and in order — so already-
/// ingested byte offsets stay valid — and remote lines not already present are
/// appended after them. No sorting: a shard is chronological by construction,
/// and keeping the local prefix byte-stable is what lets the fetch ingest only
/// the newly-appended tail.
pub fn merge_event_shard(local: &str, remote: &str) -> String {
    let mut seen: std::collections::HashSet<&str> = std::collections::HashSet::new();
    let mut out = String::new();
    for line in local.lines() {
        let trimmed = line.trim_end();
        if trimmed.is_empty() {
            continue;
        }
        seen.insert(trimmed);
        out.push_str(trimmed);
        out.push('\n');
    }
    for line in remote.lines() {
        let trimmed = line.trim_end();
        if trimmed.is_empty() {
            continue;
        }
        if seen.insert(trimmed) {
            out.push_str(trimmed);
            out.push('\n');
        }
    }
    out
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
    // Causality first. A Lamport clock is incremented on write and witnessed on
    // read, so a higher value means "written after observing the other", which
    // is what "latest" should mean. Comparing wall clocks instead made the
    // *fastest* clock the winner: a machine a year fast won every merge on every
    // entity it touched, permanently — ten honest later edits by another clone
    // all lost to its single year-ahead record, and one bad NTP sync is enough.
    //
    // A timestamp is informational only: nothing verifies it, and a machine
    // whose clock is wrong is indistinguishable from one that wrote later.
    let l_clock = l.get("lamport").and_then(|v| v.as_u64()).unwrap_or(0);
    let r_clock = r.get("lamport").and_then(|v| v.as_u64()).unwrap_or(0);
    if l_clock != r_clock {
        return if r_clock > l_clock {
            Side::Remote
        } else {
            Side::Local
        };
    }

    // Equal clocks means genuinely concurrent, or both written before clocks
    // existed. Fall back to the timestamp — imperfect, but it is what older
    // repositories have, and the tiebreak below keeps clones converging either
    // way.
    let l_ts = l.get("updated_at").and_then(|v| v.as_str()).unwrap_or("");
    let r_ts = r.get("updated_at").and_then(|v| v.as_str()).unwrap_or("");

    // Compare instants, not strings. RFC3339 is not fixed-width here: chrono
    // drops trailing subsecond zeros, so a write on an exact millisecond is
    // `.191Z` and one 220µs later is `.191220Z`. Lexicographically 'Z' (0x5A)
    // beats '2' (0x32), so the shorter string sorts higher and the *earlier*
    // write won the tiebreak — and stamped its own timestamp on the result.
    // Clones agreed with each other while doing it, so no convergence test
    // could see it.
    //
    // Unparseable values fall back to the string comparison below: it is
    // deterministic, which is the property that actually matters for clones
    // that must agree, and a malformed timestamp has no instant to compare.
    let as_instant = |s: &str| chrono::DateTime::parse_from_rfc3339(s).ok();
    if let (Some(lt), Some(rt)) = (as_instant(l_ts), as_instant(r_ts)) {
        match rt.cmp(&lt) {
            std::cmp::Ordering::Greater => return Side::Remote,
            std::cmp::Ordering::Less => return Side::Local,
            std::cmp::Ordering::Equal => {}
        }
    }

    match r_ts.cmp(l_ts) {
        std::cmp::Ordering::Greater => Side::Remote,
        std::cmp::Ordering::Less => Side::Local,
        std::cmp::Ordering::Equal => {
            // Stable, deterministic tiebreak by full-document serialization.
            let l_repr = serde_norway::to_string(l).unwrap_or_default();
            let r_repr = serde_norway::to_string(r).unwrap_or_default();
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
    let yaml: Value = serde_norway::from_str(yaml_str).map_err(JjjError::YamlParse)?;
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
    keys.sort_by_key(|k| serde_norway::to_string(k).unwrap_or_default());
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
    let key_of = |v: &Value| serde_norway::to_string(v).unwrap_or_default();
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

/// Resolve a contested claim to the *earliest* claimant, identically on every
/// clone.
///
/// Claims are advisory, so two agents genuinely do claim the same item — 42 of
/// 72 contested entities in one trial. Leaving that to the generic
/// last-writer-wins rule made the holder depend on merge order and on how much
/// each clone had fetched, so pods disagreed about who held what until several
/// push/fetch rounds had settled, and agents kept re-claiming work in the
/// meantime.
///
/// Earliest-wins fixes both halves. It is **order-independent**: min() over the
/// same pair gives the same answer whichever side is called "local", so every
/// clone converges in one merge instead of N rounds. And it is **stable**: once
/// an item is claimed, no later claim can displace it, so the field stops
/// changing — where latest-wins would let every fresh claim churn it.
///
/// The claimant's name breaks an exact timestamp tie, so the result is total.
///
/// Earliest-wins applies **only between two leases**, because a bare
/// `assignee` with no `claimed_at` is not a lease at all: `problem assign`
/// writes one deliberately, since handing work to a person is a decision and not
/// something that should expire. Treating "one side has no `claimed_at`" as
/// "only one side claimed it, so adopt that side" got this backwards — it let a
/// lease that merely still existed on another clone revert a later hand-off,
/// and then stamped the reverted owner with the hand-off's clock, so the record
/// claimed a causal position its content never had. Which owner survived
/// depended on nothing the user could see.
///
/// For that mixed case the generic merge has already ordered `assignee` by
/// clock, and that ordering *is* the answer: a hand-off that witnessed a lease
/// overrides it, and a lease taken after a hand-off overrides that. What remains
/// is keeping `claimed_at` attached to the owner it belongs to — like
/// `dissolved_reason` under [`normalize_status_dependents`], it annotates a
/// decision rather than standing on its own, so it follows that decision or goes.
fn normalize_claim(mut merged: Value, local: &Value, remote: &Value) -> Value {
    // The `(claimed_at, assignee)` pair, present only for a timed lease.
    let lease_of = |v: &Value| -> Option<(String, String)> {
        let at = v.get("claimed_at")?.as_str()?.to_string();
        let who = v.get("assignee")?.as_str()?.to_string();
        Some((at, who))
    };

    // Two live leases: earliest wins, identically on every clone.
    if let (Some(a), Some(b)) = (lease_of(local), lease_of(remote)) {
        let (at, who) = if a <= b { a } else { b };
        if let Value::Mapping(ref mut m) = merged {
            m.insert(Value::String("assignee".to_string()), Value::String(who));
            m.insert(Value::String("claimed_at".to_string()), Value::String(at));
        }
        return merged;
    }

    // At most one lease. Let the clock-ordered `assignee` stand, and keep the
    // lease only if it is that owner's.
    let owner = merged
        .get("assignee")
        .and_then(|v| v.as_str())
        .map(str::to_string);
    let lease = lease_of(local).or_else(|| lease_of(remote));

    if let Value::Mapping(ref mut m) = merged {
        match (owner, lease) {
            (Some(owner), Some((at, who))) if owner == who => {
                m.insert(Value::String("claimed_at".to_string()), Value::String(at));
            }
            _ => {
                m.remove(Value::String("claimed_at".to_string()));
            }
        }
    }
    merged
}

/// Drop a field that only has meaning in a status the merge did not choose.
///
/// [`merge_mapping`] merges every key independently, which is the property that
/// lets two clones edit different fields of one entity without conflicting. A
/// few fields are not independent of `status`, though: `dissolved_reason` says
/// why a problem was dissolved, and `superseded_by` which finding replaced this
/// one. Each is meaningless — worse, actively contradictory — under any other
/// status.
///
/// The failure is reachable with two legal transitions. One clone dissolves a
/// problem, writing `status: dissolved` *and* a reason; another solves it,
/// writing `status: solved`. Causal order resolves `status` to one side, but the
/// reason is new on the losing side and absent from the base, so the generic
/// merge keeps it unconditionally — yielding a problem that is `solved` while
/// carrying a dissolution reason. Solved means an approved solution fixed it;
/// dissolved means it was never a real problem to begin with. Both at once is
/// not a state the model has.
///
/// Dropping the dependent field is the right repair rather than picking a winner
/// for the pair: the status *is* the decision and the reason is an annotation on
/// it, so the annotation follows the decision. Nothing is lost either way,
/// because `ProblemDissolved` and `FindingSuperseded` both record the detail in
/// the event log, which merges by line union.
fn normalize_status_dependents(mut merged: Value) -> Value {
    /// `(field, the only status under which it means anything)`
    const DEPENDENTS: &[(&str, &str)] = &[
        ("dissolved_reason", "dissolved"),
        ("superseded_by", "superseded"),
    ];

    let status = merged
        .get("status")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();

    if let Value::Mapping(ref mut m) = merged {
        for (field, required) in DEPENDENTS {
            if status != *required {
                m.remove(Value::String((*field).to_string()));
            }
        }
    }
    merged
}

fn sort_mapping_keys(v: Value) -> Value {
    match v {
        Value::Mapping(m) => {
            let mut entries: Vec<(Value, Value)> = m.into_iter().collect();
            entries.sort_by_key(|(k, _)| serde_norway::to_string(k).unwrap_or_default());
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
    // An already-conflicted body must never be wrapped a second time.
    //
    // Re-merging used to add another marker set every single time: merging a
    // conflicted body against either of its own parents produced 1 → 2 → 3
    // nested markers, and a measured swarm run went from 4,371 single-depth
    // markers to 14,215 nested ones purely through repeated fetches. The
    // conflicted side already contains both versions, so nesting adds no
    // information and destroys what readability was left — and `push` already
    // refuses a body carrying markers, so a conflicted entity is a
    // stop-and-resolve state rather than something to keep merging.
    //
    // beads records the same invariant for its own merge: an operation whose
    // output changes when re-applied "makes the same merge produce different
    // bytes on each replica so the next sync re-conflicts".
    let l_conflicted = has_conflict_markers(local);
    let r_conflicted = has_conflict_markers(remote);
    if l_conflicted || r_conflicted {
        let keep = match (l_conflicted, r_conflicted) {
            (true, false) => local,
            (false, true) => remote,
            // Both carry markers: choose by content, so the answer cannot
            // depend on which clone happens to be called "local".
            _ => std::cmp::max(local, remote),
        };
        return ensure_trailing_newline(keep);
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

    /// One clone dissolves a problem while another solves it. Both transitions
    /// are legal from `open`, so this is reachable without anyone doing anything
    /// wrong — and before the status-dependent normalization it produced a
    /// problem that was `solved` *and* carried a dissolution reason.
    #[test]
    fn solved_problem_does_not_keep_a_dissolution_reason() {
        let local = rewrite(
            BASE,
            &[(
                "status: open\n",
                "status: dissolved\ndissolved_reason: based on a false premise\n",
            )],
        )
        .replace("updated_at: 2026-05-01", "updated_at: 2026-05-02");
        let remote = rewrite(BASE, &[("status: open\n", "status: solved\n")])
            .replace("updated_at: 2026-05-01", "updated_at: 2026-05-03");

        let merged = merge_entity_md(Some(BASE), &local, &remote).unwrap();
        assert!(merged.contains("status: solved"), "{merged}");
        assert!(
            !merged.contains("dissolved_reason"),
            "a solved problem is carrying a dissolution reason: {merged}"
        );
    }

    /// The mirror image: when the dissolution is the side that wins, the reason
    /// is exactly the field that should survive. A fix that drops it whenever
    /// the two sides disagree would pass the test above and still be wrong.
    #[test]
    fn dissolved_problem_keeps_its_reason() {
        let local = rewrite(
            BASE,
            &[(
                "status: open\n",
                "status: dissolved\ndissolved_reason: based on a false premise\n",
            )],
        )
        .replace("updated_at: 2026-05-01", "updated_at: 2026-05-03");
        let remote = rewrite(BASE, &[("status: open\n", "status: solved\n")])
            .replace("updated_at: 2026-05-01", "updated_at: 2026-05-02");

        let merged = merge_entity_md(Some(BASE), &local, &remote).unwrap();
        assert!(merged.contains("status: dissolved"), "{merged}");
        assert!(
            merged.contains("based on a false premise"),
            "the dissolution reason was dropped from a dissolved problem: {merged}"
        );
    }

    /// An uncontested dissolution must not be disturbed: only one side moved, so
    /// the whole-document shortcut should return it untouched, reason included.
    #[test]
    fn an_uncontested_dissolution_is_untouched() {
        let local = rewrite(
            BASE,
            &[(
                "status: open\n",
                "status: dissolved\ndissolved_reason: no longer relevant\n",
            )],
        );
        let merged = merge_entity_md(Some(BASE), &local, BASE).unwrap();
        assert!(merged.contains("status: dissolved"), "{merged}");
        assert!(merged.contains("no longer relevant"), "{merged}");
    }

    /// The same shape on a finding: `superseded_by` points at the measurement
    /// that replaced this one, and means nothing while the finding is current.
    #[test]
    fn current_finding_does_not_keep_a_superseded_by_pointer() {
        const F: &str = "---\n\
id: f1\n\
title: The parse floors at a fixed cost\n\
status: current\n\
created_at: 2026-05-01T00:00:00Z\n\
updated_at: 2026-05-01T00:00:00Z\n\
---\n\n\
Counted over three runs.\n";

        // One clone supersedes it; another edits it and stays current, winning
        // on causal order.
        let local = F
            .replace(
                "status: current\n",
                "status: superseded\nsuperseded_by: f2\n",
            )
            .replace("updated_at: 2026-05-01", "updated_at: 2026-05-02");
        let remote = F
            .replace("Counted over three runs.", "Counted over ten runs.")
            .replace("updated_at: 2026-05-01", "updated_at: 2026-05-03");

        let merged = merge_entity_md(Some(F), &local, &remote).unwrap();
        if merged.contains("status: current") {
            assert!(
                !merged.contains("superseded_by"),
                "a current finding points at a superseding one: {merged}"
            );
        } else {
            // The supersession won instead, in which case the pointer is
            // required — the invariant is that the two agree, not which wins.
            assert!(merged.contains("superseded_by"), "{merged}");
        }
    }

    /// Divergent scalars must survive a re-merge byte-for-byte. If they did
    /// not, every fetch would re-conflict the same entity forever.
    #[test]
    fn remerging_a_scalar_merge_changes_nothing() {
        let local = rewrite(BASE, &[("title: Original", "title: Alice")])
            .replace("updated_at: 2026-05-01", "updated_at: 2026-05-02");
        let remote = rewrite(BASE, &[("priority: medium", "priority: high")])
            .replace("updated_at: 2026-05-01", "updated_at: 2026-05-03");

        let merged = merge_entity_md(Some(BASE), &local, &remote).unwrap();
        for parent in [&local, &remote] {
            let again = merge_entity_md(Some(BASE), &merged, parent).unwrap();
            assert_eq!(
                again, merged,
                "re-merging the result against a parent changed it, so the next \
                 sync would conflict again"
            );
        }
    }

    /// Repeated fetches must not deepen a conflict. This used to add one marker
    /// set per merge — 1, then 2, then 3 — which is how a swarm run turned
    /// 4,371 single-depth markers into 14,215 nested ones.
    #[test]
    fn a_conflicted_body_is_never_wrapped_twice() {
        let local = rewrite(BASE, &[("Original body.", "Alice body.")])
            .replace("updated_at: 2026-05-01", "updated_at: 2026-05-02");
        let remote = rewrite(BASE, &[("Original body.", "Bob body.")])
            .replace("updated_at: 2026-05-01", "updated_at: 2026-05-03");

        let first = merge_entity_md(Some(BASE), &local, &remote).unwrap();
        assert_eq!(
            first.matches("<<<<<<<").count(),
            1,
            "the first merge should conflict exactly once: {first}"
        );
        assert!(first.contains("Alice body.") && first.contains("Bob body."));

        // Fetch again, and again.
        let second = merge_entity_md(Some(BASE), &first, &local).unwrap();
        let third = merge_entity_md(Some(BASE), &second, &remote).unwrap();
        for (n, text) in [(2, &second), (3, &third)] {
            assert_eq!(
                text.matches("<<<<<<<").count(),
                1,
                "merge #{n} nested the markers instead of leaving them alone: {text}"
            );
        }
        assert_eq!(second, first, "merge #2 changed an already-conflicted body");
        assert_eq!(third, first, "merge #3 changed an already-conflicted body");
    }

    /// Keeping the conflicted side must not depend on which clone is "local",
    /// or two clones would store different bytes and never converge.
    #[test]
    fn keeping_a_conflicted_body_is_order_independent() {
        let local = rewrite(BASE, &[("Original body.", "Alice body.")])
            .replace("updated_at: 2026-05-01", "updated_at: 2026-05-02");
        let remote = rewrite(BASE, &[("Original body.", "Bob body.")])
            .replace("updated_at: 2026-05-01", "updated_at: 2026-05-03");
        let conflicted = merge_entity_md(Some(BASE), &local, &remote).unwrap();

        let a = merge_entity_md(Some(BASE), &conflicted, &local).unwrap();
        let b = merge_entity_md(Some(BASE), &local, &conflicted).unwrap();
        assert_eq!(a, b, "the conflicted body depended on argument order");
    }

    /// One clone solves a problem while another claims it. Reviewing beads'
    /// `row_lock` turned up the claim that this merges into "Solved, and held by
    /// someone" — which it does, and which is **fine**, pinned here so the next
    /// person to read beads' rationale does not re-litigate it.
    ///
    /// It is harmless because nothing in jjj ever clears a claim: an assignee
    /// outlives the work by design, so "solved with an assignee" is the ordinary
    /// state after any agent finishes what it claimed. The hazard beads guards
    /// against needs a writer that moves `status` *backward* — their reclaim
    /// reverting a close — and jjj has none: claim expiry is a read-time
    /// function (`claim::classify`) that writes nothing at all.
    ///
    /// What this test is really for is the day that stops being true. A future
    /// command that writes `status` as part of releasing a claim would make the
    /// mix dangerous, and this is where that shows up.
    #[test]
    fn solving_while_another_clone_claims_is_benign_but_pinned() {
        let solved = rewrite(BASE, &[("status: open", "status: solved")])
            .replace("updated_at: 2026-05-01", "updated_at: 2026-05-02");
        let claimed = rewrite(
            BASE,
            &[(
                "status: open\n",
                "status: open\nassignee: agent-02\nclaimed_at: 2026-05-03T00:00:00Z\n",
            )],
        )
        .replace("updated_at: 2026-05-01", "updated_at: 2026-05-03");

        let merged = merge_entity_md(Some(BASE), &solved, &claimed).unwrap();
        assert!(merged.contains("status: solved"), "{merged}");
        assert!(
            merged.contains("agent-02"),
            "the claim should survive — it is not in conflict with the status: {merged}"
        );

        // And the result must not depend on which clone is called local.
        let other = merge_entity_md(Some(BASE), &claimed, &solved).unwrap();
        assert_eq!(merged, other, "the status/claim merge depended on order");
    }

    /// A deliberate hand-off must not be reverted by a lease that merely still
    /// exists on another clone.
    ///
    /// `problem assign` records ownership with no `claimed_at`, because handing
    /// work to someone is a decision and not a lease that expires. A timed claim
    /// records both. `normalize_claim` arbitrates *between claims* by taking the
    /// earliest `claimed_at`, and it treated "one side has no `claimed_at`" as
    /// "only one side claimed it, so adopt that one" — which, for a hand-off, is
    /// backwards. The hand-off is the newer decision; the lease is the thing it
    /// overrides.
    #[test]
    fn a_handoff_is_not_reverted_by_a_surviving_lease() {
        // Both sides diverge from an unowned base: one clone took a lease, the
        // other handed the work to a person. The hand-off is causally later.
        let leased = rewrite(
            BASE,
            &[(
                "status: open\n",
                "status: open\nassignee: agent-01\nclaimed_at: 2026-05-02T00:00:00Z\nlamport: 3\n",
            )],
        );
        let handed_off = rewrite(
            BASE,
            &[(
                "status: open\n",
                "status: open\nassignee: carol\nlamport: 7\n",
            )],
        );

        let merged = merge_entity_md(Some(BASE), &handed_off, &leased).unwrap();
        assert!(
            merged.contains("assignee: carol"),
            "the hand-off lost to the lease it was meant to override: {merged}"
        );
        assert!(
            !merged.contains("claimed_at"),
            "a hand-off has no lease, so the merge must not resurrect one: {merged}"
        );

        let other = merge_entity_md(Some(BASE), &leased, &handed_off).unwrap();
        assert_eq!(merged, other, "the hand-off merge depended on order");
    }

    /// The converse, so the fix cannot be "always prefer the side without a
    /// lease": a *stale* hand-off must lose to a lease taken after it.
    #[test]
    fn a_lease_taken_after_a_handoff_wins() {
        let handed_off = rewrite(
            BASE,
            &[(
                "status: open\n",
                "status: open\nassignee: carol\nlamport: 3\n",
            )],
        );
        let leased = rewrite(
            BASE,
            &[(
                "status: open\n",
                "status: open\nassignee: agent-01\nclaimed_at: 2026-05-02T00:00:00Z\nlamport: 7\n",
            )],
        );

        let merged = merge_entity_md(Some(BASE), &handed_off, &leased).unwrap();
        assert!(
            merged.contains("assignee: agent-01") && merged.contains("claimed_at"),
            "a lease taken after the hand-off should hold: {merged}"
        );
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
        // updated_at must be the max — serde_norway quotes date-like strings.
        assert!(
            merged.contains("2026-05-03T00:00:00Z"),
            "expected 2026-05-03 timestamp in merged output:\n{}",
            merged
        );
        assert!(!merged.contains("2026-05-02T00:00:00Z"));
    }

    /// chrono drops trailing subsecond zeros, so RFC3339 is not fixed-width —
    /// and `pick_side` compares the timestamp as a *string*.
    ///
    /// A write landing on an exact millisecond serializes as `.191Z`, one
    /// 220 microseconds later as `.191220Z`. Lexicographically `'Z'` (0x5A)
    /// beats `'2'` (0x32), so the shorter string sorts *higher* and the earlier
    /// write wins the tiebreak. Same for an exact second against anything in
    /// it. Clones still converge — the comparison is deterministic — so this is
    /// not divergence, it is agreeing on the wrong side, which is why no
    /// convergence test would ever have caught it.
    ///
    /// Raised by an agent re-reading beads' schema history, where the same
    /// class of bug appears as `DATETIME(0)` rounding half-up
    /// (`0069_widen_issue_versions_datetime_precision`).
    #[test]
    fn a_timestamp_tiebreak_compares_instants_not_strings() {
        use chrono::{TimeZone, Utc};

        // The exact pairs chrono's AutoSi formatting produces.
        let cases = [
            (1_792_200_000i64, 191_000_000u32, 191_220_000u32),
            (1_792_200_000, 0, 1_000),
            (1_792_200_000, 0, 500_000_000),
        ];

        for (secs, earlier_nanos, later_nanos) in cases {
            let earlier = Utc.timestamp_opt(secs, earlier_nanos).unwrap();
            let later = Utc.timestamp_opt(secs, later_nanos).unwrap();
            assert!(earlier < later, "fixture is backwards");

            let e_str = earlier.to_rfc3339_opts(chrono::SecondsFormat::AutoSi, true);
            let l_str = later.to_rfc3339_opts(chrono::SecondsFormat::AutoSi, true);

            // The later write must win the tiebreak, whichever side it is on.
            let local = rewrite(
                BASE,
                &[
                    ("title: Original", "title: Earlier"),
                    (
                        "updated_at: 2026-05-01T00:00:00Z",
                        &format!("updated_at: {e_str}"),
                    ),
                ],
            );
            let remote = rewrite(
                BASE,
                &[
                    ("title: Original", "title: Later"),
                    (
                        "updated_at: 2026-05-01T00:00:00Z",
                        &format!("updated_at: {l_str}"),
                    ),
                ],
            );

            let merged = merge_entity_md(Some(BASE), &local, &remote).unwrap();
            assert!(
                merged.contains("title: Later"),
                "the later write ({l_str}) must beat the earlier ({e_str}), \
                 but the merge kept:\n{merged}"
            );

            // And the other way round, so the result does not depend on which
            // clone is called "local".
            let swapped = merge_entity_md(Some(BASE), &remote, &local).unwrap();
            assert!(
                swapped.contains("title: Later"),
                "order-swapped merge of {l_str} vs {e_str} kept:\n{swapped}"
            );
        }
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
    fn event_shard_union_is_append_only() {
        // Local prefix must be preserved byte-for-byte (offset stability), with
        // remote-only lines appended after — never reordered.
        let local = "{\"e\":1}\n{\"e\":2}\n";
        let remote = "{\"e\":1}\n{\"e\":3}\n";
        let merged = merge_event_shard(local, remote);
        assert_eq!(merged, "{\"e\":1}\n{\"e\":2}\n{\"e\":3}\n");
        // The local content is an exact byte prefix of the merge.
        assert!(merged.starts_with(local), "local prefix must be stable");
    }

    #[test]
    fn event_shard_union_no_new_remote_is_noop() {
        // The pod's own shard: remote is a subset of local → unchanged (so the
        // ingest offset stays valid and nothing is re-read).
        let local = "{\"e\":1}\n{\"e\":2}\n";
        let remote = "{\"e\":1}\n";
        assert_eq!(merge_event_shard(local, remote), local);
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

    /// A conflicted body from `merge_body`, resolved to each side and to a no-op
    /// on marker-free input.
    #[test]
    fn resolve_conflict_markers_picks_a_side() {
        let conflicted = merge_body(Some("base"), "my body", "their body");
        assert!(has_conflict_markers(&conflicted));

        let ours = resolve_conflict_markers(&conflicted, ConflictSide::Local);
        assert_eq!(ours.trim(), "my body");
        assert!(!has_conflict_markers(&ours));

        let theirs = resolve_conflict_markers(&conflicted, ConflictSide::Remote);
        assert_eq!(theirs.trim(), "their body");
        assert!(!has_conflict_markers(&theirs));
    }

    #[test]
    fn resolve_conflict_markers_preserves_surrounding_text() {
        let content =
            "Intro line.\n<<<<<<< local\nmine\n=======\ntheirs\n>>>>>>> remote\nOutro line.\n";
        let ours = resolve_conflict_markers(content, ConflictSide::Local);
        assert_eq!(ours, "Intro line.\nmine\nOutro line.\n");
    }

    #[test]
    fn resolve_conflict_markers_is_noop_without_markers() {
        let clean = "Just a body.\nNo markers.\n";
        assert_eq!(resolve_conflict_markers(clean, ConflictSide::Local), clean);
    }

    #[test]
    fn resolve_conflict_markers_handles_multiple_blocks() {
        let content = "\
<<<<<<< local
a-mine
=======
a-theirs
>>>>>>> remote
middle
<<<<<<< local
b-mine
=======
b-theirs
>>>>>>> remote
";
        let theirs = resolve_conflict_markers(content, ConflictSide::Remote);
        assert_eq!(theirs, "a-theirs\nmiddle\nb-theirs\n");
    }
}

#[cfg(test)]
mod convergence_tests {
    use super::*;

    fn doc(assignee: &str, updated: &str) -> String {
        format!(
            "---\nid: p1\ntitle: Contended\nstatus: open\nassignee: {assignee}\n\
             created_at: '2026-08-20T11:00:00Z'\nupdated_at: '{updated}'\n---\n"
        )
    }

    fn assignee_of(md: &str) -> String {
        md.lines()
            .find(|l| l.starts_with("assignee:"))
            .unwrap_or("assignee: <none>")
            .trim()
            .to_string()
    }

    /// Three pods claim the same item, then each merges the others' versions.
    /// Whatever order a pod happens to merge in, every pod must end up agreeing
    /// on who holds it — otherwise the clones permanently disagree and the whole
    /// coordination story is unsound.
    ///
    /// This is not hypothetical: with three pods racing one problem, the clones
    /// ended up seeing agent-c, agent-a and agent-a respectively.
    #[test]
    fn concurrent_scalar_edits_converge_regardless_of_merge_order() {
        let base = doc("<none>", "2026-08-20T11:00:00Z");
        let a = doc("agent-a", "2026-08-20T11:56:02.314999Z");
        let b = doc("agent-b", "2026-08-20T11:56:02.350000Z");
        let c = doc("agent-c", "2026-08-20T11:56:02.379188Z");

        // Pod A merges B then C; pod B merges C then A; pod C merges A then B.
        let a_view = {
            let m = merge_entity_md(Some(&base), &a, &b).expect("merge");
            merge_entity_md(Some(&base), &m, &c).expect("merge")
        };
        let b_view = {
            let m = merge_entity_md(Some(&base), &b, &c).expect("merge");
            merge_entity_md(Some(&base), &m, &a).expect("merge")
        };
        let c_view = {
            let m = merge_entity_md(Some(&base), &c, &a).expect("merge");
            merge_entity_md(Some(&base), &m, &b).expect("merge")
        };

        assert_eq!(
            assignee_of(&a_view),
            assignee_of(&b_view),
            "pods A and B disagree after seeing the same three versions:\n\
             A: {}\nB: {}",
            assignee_of(&a_view),
            assignee_of(&b_view)
        );
        assert_eq!(
            assignee_of(&b_view),
            assignee_of(&c_view),
            "pods B and C disagree after seeing the same three versions"
        );

        // And the winner should be the genuinely newest write, not an artefact
        // of who merged in which order.
        assert_eq!(
            assignee_of(&a_view),
            "assignee: agent-c",
            "last-writer-wins should pick the newest edit"
        );
    }
    /// Identical timestamps must still converge.
    ///
    /// The recency comparison cannot separate these, so the tiebreak decides —
    /// and a tiebreak that depends on which side is "local" would leave two
    /// clones permanently disagreeing about the same pair of edits.
    ///
    /// This is not a hypothetical either. Commands that assigned a field
    /// directly never moved `updated_at`, so an edit and the version it was
    /// derived from arrived at this function with the *same* timestamp, and the
    /// whole outcome rested on this tiebreak being symmetric.
    #[test]
    fn identical_timestamps_still_converge() {
        let ts = "2026-08-20T11:56:02.314999Z";
        let base = doc("<none>", ts);
        let a = doc("agent-a", ts);
        let b = doc("agent-b", ts);

        let a_view = merge_entity_md(Some(&base), &a, &b).expect("merge");
        let b_view = merge_entity_md(Some(&base), &b, &a).expect("merge");
        assert_eq!(
            assignee_of(&a_view),
            assignee_of(&b_view),
            "the tiebreak is not symmetric, so two clones seeing the same pair \
             of edits end up disagreeing forever"
        );
    }

    /// Concurrent tag edits must not lose a tag.
    ///
    /// Tag changes resolve *per tag*, so adding one tag on one clone and a
    /// different tag on another must leave both. jjj merges the `tags` sequence
    /// as a base-aware set union, which should give that —
    /// worth pinning, because merging the sequence as a single scalar would
    /// silently drop one side's tag and the module docs claiming otherwise would
    /// be the only evidence.
    #[test]
    fn concurrent_tag_additions_both_survive() {
        fn doc(tags: &[&str], lamport: u64) -> String {
            let list = if tags.is_empty() {
                "[]".to_string()
            } else {
                format!("[{}]", tags.join(", "))
            };
            format!(
                "---\nid: p1\ntitle: Tagged\nstatus: open\ntags: {list}\n\
                 created_at: '2026-01-01T00:00:00Z'\nupdated_at: '2026-01-02T00:00:00Z'\n\
                 lamport: {lamport}\n---\n\nbody\n"
            )
        }
        /// Read the merged tag set, accepting either YAML style.
        ///
        /// The merger emits a block sequence (`tags:` then `- core`), not the
        /// inline `[core]` the inputs use. A helper that only understood the
        /// inline form reported every tag as lost — the assertion failed for a
        /// fault in the test, not the code.
        fn tags_of(md: &str) -> Vec<String> {
            let mut out = Vec::new();
            let mut in_block = false;
            for line in md.lines() {
                if let Some(rest) = line.strip_prefix("tags:") {
                    let rest = rest.trim();
                    if rest.is_empty() {
                        in_block = true; // block sequence follows
                    } else {
                        // Inline form.
                        out.extend(
                            rest.trim_start_matches('[')
                                .trim_end_matches(']')
                                .split(',')
                                .map(|t| t.trim().trim_matches('\'').to_string())
                                .filter(|t| !t.is_empty()),
                        );
                    }
                    continue;
                }
                if in_block {
                    match line.strip_prefix("- ") {
                        Some(item) => out.push(item.trim().trim_matches('\'').to_string()),
                        None => in_block = false,
                    }
                }
            }
            out
        }

        let base = doc(&["core"], 1);
        let local = doc(&["core", "perf"], 2);
        let remote = doc(&["core", "docs"], 2);

        let merged = merge_entity_md(Some(&base), &local, &remote).expect("merge");
        let mut tags = tags_of(&merged);
        tags.sort();
        assert_eq!(
            tags,
            vec!["core", "docs", "perf"],
            "a concurrently-added tag was lost: {merged}"
        );

        // Symmetric, or the two clones disagree about the tag set.
        let other = merge_entity_md(Some(&base), &remote, &local).expect("merge");
        let mut other_tags = tags_of(&other);
        other_tags.sort();
        assert_eq!(tags, other_tags, "tag union is not symmetric");
    }

    /// A deliberate removal must stick, rather than being re-added by the union.
    #[test]
    fn a_removed_tag_stays_removed_when_the_other_side_did_not_touch_it() {
        fn doc(tags: &str, lamport: u64) -> String {
            format!(
                "---\nid: p1\ntitle: Tagged\nstatus: open\ntags: {tags}\n\
                 created_at: '2026-01-01T00:00:00Z'\nupdated_at: '2026-01-02T00:00:00Z'\n\
                 lamport: {lamport}\n---\n\nbody\n"
            )
        }
        let base = doc("[core, stale]", 1);
        // One side drops `stale`; the other only changes something else.
        let remover = doc("[core]", 2);
        let untouched = doc("[core, stale]", 1);

        let merged = merge_entity_md(Some(&base), &remover, &untouched).expect("merge");
        assert!(
            !merged.contains("stale"),
            "the union resurrected a deliberately removed tag: {merged}"
        );
    }

    /// A fast wall clock must not win, and must not win forever.
    ///
    /// This is the failure the Lamport clock exists to prevent, measured before
    /// it was added: with ordering decided by `updated_at`, a machine one year
    /// fast won *every* merge on every entity it touched. Ten honest later edits
    /// by another clone all lost to its single year-ahead record. One bad NTP
    /// sync is enough — no malice required.
    #[test]
    fn a_skewed_wall_clock_does_not_beat_a_higher_lamport_clock() {
        fn doc(title: &str, updated: &str, lamport: u64) -> String {
            format!(
                "---\nid: p1\ntitle: {title}\nstatus: open\n\
                 created_at: '2026-01-01T00:00:00Z'\nupdated_at: '{updated}'\n\
                 lamport: {lamport}\n---\n\nbody\n"
            )
        }
        fn title_of(md: &str) -> String {
            md.lines()
                .find(|l| l.starts_with("title:"))
                .unwrap_or("title: ?")
                .to_string()
        }

        let base = doc("Original", "2026-01-01T00:00:00Z", 1);
        // B's machine is a year fast, but it has only seen one edit.
        let b_skewed = doc("B, clock a year fast", "2027-01-01T00:00:00Z", 2);
        // A has observed more of the world, so its clock is higher.
        let a = doc("A, honest and later", "2026-06-01T00:00:00Z", 3);

        let a_view = merge_entity_md(Some(&base), &a, &b_skewed).expect("merge");
        let b_view = merge_entity_md(Some(&base), &b_skewed, &a).expect("merge");

        assert_eq!(
            title_of(&a_view),
            "title: A, honest and later",
            "the year-ahead wall clock beat a higher Lamport clock"
        );
        assert_eq!(
            title_of(&a_view),
            title_of(&b_view),
            "clock-ordered merges must still be symmetric, or clones diverge"
        );
    }

    /// Entities written before Lamport clocks existed still order by timestamp.
    #[test]
    fn a_zero_clock_falls_back_to_the_timestamp() {
        let base = doc("<none>", "2026-08-20T11:00:00Z");
        let older = doc("agent-a", "2026-08-20T11:56:02.314999Z");
        let newer = doc("agent-b", "2026-08-20T11:56:02.350000Z");
        let merged = merge_entity_md(Some(&base), &older, &newer).expect("merge");
        assert_eq!(
            assignee_of(&merged),
            "assignee: agent-b",
            "without clocks the timestamp must still decide"
        );
    }

    /// And with no base at all — the cold-start case.
    ///
    /// A fresh clone has no merge base for any bookmark, so `fetch` resolves
    /// entity files with a two-way merge. If that is asymmetric, two agents
    /// cloning the same remote read different data from it.
    #[test]
    fn baseless_merges_converge() {
        let a = doc("agent-a", "2026-08-20T11:56:02.314999Z");
        let b = doc("agent-b", "2026-08-20T11:56:02.350000Z");

        let a_view = merge_entity_md(None, &a, &b).expect("merge");
        let b_view = merge_entity_md(None, &b, &a).expect("merge");
        assert_eq!(
            assignee_of(&a_view),
            assignee_of(&b_view),
            "two clones of one remote disagree at cold start"
        );
        assert_eq!(
            assignee_of(&a_view),
            "assignee: agent-b",
            "with no base, the newer write should still win"
        );
    }
}

#[cfg(test)]
mod claim_merge_tests {
    use super::*;

    fn doc(assignee: &str, claimed_at: &str, updated: &str) -> String {
        format!(
            "---\nid: p1\ntitle: Contended\nstatus: open\n\
             assignee: {assignee}\nclaimed_at: '{claimed_at}'\n\
             created_at: '2026-08-20T11:00:00Z'\nupdated_at: '{updated}'\n---\n"
        )
    }

    fn field(md: &str, key: &str) -> String {
        md.lines()
            .find(|l| l.starts_with(&format!("{key}:")))
            .map(|l| {
                l.split_once(':')
                    .unwrap()
                    .1
                    .trim()
                    .trim_matches('\'')
                    .to_string()
            })
            .unwrap_or_default()
    }

    /// The first claimant holds it, whichever side merges first.
    #[test]
    fn the_earliest_claim_wins_in_either_order() {
        let base = "---\nid: p1\ntitle: Contended\nstatus: open\n\
                    created_at: '2026-08-20T11:00:00Z'\nupdated_at: '2026-08-20T11:00:00Z'\n---\n";
        let early = doc("agent-a", "2026-08-20T12:00:00Z", "2026-08-20T12:00:00Z");
        // agent-b claimed later but wrote later too, so plain LWW would hand it
        // to agent-b — the opposite of first-come-first-served.
        let late = doc("agent-b", "2026-08-20T12:00:05Z", "2026-08-20T12:00:09Z");

        let one = merge_entity_md(Some(base), &early, &late).expect("merge");
        let other = merge_entity_md(Some(base), &late, &early).expect("merge");

        assert_eq!(field(&one, "assignee"), "agent-a");
        assert_eq!(
            field(&one, "assignee"),
            field(&other, "assignee"),
            "the holder must not depend on which side merged first"
        );
        assert_eq!(field(&one, "claimed_at"), field(&other, "claimed_at"));
    }

    /// Three pods contesting one item must agree after a single merge each,
    /// rather than after several push/fetch rounds.
    #[test]
    fn three_way_contention_converges_in_one_round() {
        let base = "---\nid: p1\ntitle: Contended\nstatus: open\n\
                    created_at: '2026-08-20T11:00:00Z'\nupdated_at: '2026-08-20T11:00:00Z'\n---\n";
        let a = doc("agent-a", "2026-08-20T12:00:01Z", "2026-08-20T12:00:01Z");
        let b = doc("agent-b", "2026-08-20T12:00:02Z", "2026-08-20T12:00:02Z");
        let c = doc("agent-c", "2026-08-20T12:00:03Z", "2026-08-20T12:00:03Z");

        let a_view = merge_entity_md(
            Some(base),
            &merge_entity_md(Some(base), &a, &b).unwrap(),
            &c,
        )
        .unwrap();
        let b_view = merge_entity_md(
            Some(base),
            &merge_entity_md(Some(base), &b, &c).unwrap(),
            &a,
        )
        .unwrap();
        let c_view = merge_entity_md(
            Some(base),
            &merge_entity_md(Some(base), &c, &a).unwrap(),
            &b,
        )
        .unwrap();

        assert_eq!(
            field(&a_view, "assignee"),
            "agent-a",
            "first claimant holds it"
        );
        assert_eq!(field(&b_view, "assignee"), "agent-a");
        assert_eq!(field(&c_view, "assignee"), "agent-a");
    }

    /// An uncontested claim is simply adopted.
    #[test]
    fn an_uncontested_claim_is_adopted() {
        let base = "---\nid: p1\ntitle: Free\nstatus: open\n\
                    created_at: '2026-08-20T11:00:00Z'\nupdated_at: '2026-08-20T11:00:00Z'\n---\n";
        let claimed = doc("agent-a", "2026-08-20T12:00:00Z", "2026-08-20T12:00:00Z");

        let merged = merge_entity_md(Some(base), base, &claimed).expect("merge");
        assert_eq!(field(&merged, "assignee"), "agent-a");
    }
}
