//! The operation shape, modelled on git-bug's data model as summarised in
//! design note §3, and the replay that turns a set of packs into current state.
//!
//! What is faithful to git-bug:
//!
//! * An operation carries a type, an author *reference* (an identity id, not an
//!   inlined identity), a Lamport clock, a wall-clock timestamp, its payload and
//!   a random nonce.
//! * Operations from one editing session group into an `OperationPack`,
//!   serialised as JSON.
//! * A pack's id is **not** inside the pack. In git-bug it is the hash of the
//!   stored object, so replay learns it from storage. Here it is the filename.
//! * Replay sorts by Lamport clock and tie-breaks concurrent operations by
//!   lexicographic pack id, then applies in order.
//!
//! What is not faithful, and why it matters to the numbers:
//!
//! * No signature verification. git-bug can sign packs; verifying signatures on
//!   every read would add cost this spike does not measure.
//! * No zlib. git-bug's packs are git blobs, so a real read inflates them. See
//!   the note in `main.rs` where that is estimated from the measured sizes.

use serde::{Deserialize, Serialize};

/// The payload of an operation. Nested under `data` in [`WireFormat::Nested`],
/// flattened into the operation object in [`WireFormat::Flat`] — git-bug uses
/// the flat form, and serde's `flatten` buffers through a map on the way in, so
/// the two are measured separately rather than assumed equivalent.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum OpData {
    SetTitle { title: String },
    SetDescription { description: String },
    SetStatus { status: String },
    SetPriority { priority: String },
    AddTag { tag: String },
    RemoveTag { tag: String },
    SetAssignee { assignee: Option<String> },
    SetParent { parent_id: Option<String> },
}

/// One operation, nested-payload wire form.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNested {
    /// Identity *reference* — git-bug stores the identity's id, not the identity.
    pub author: String,
    pub lamport: u64,
    /// Informational only, exactly as in jjj today: nothing verifies it.
    pub timestamp: i64,
    /// Random per-operation nonce, so two identical edits are distinct objects.
    pub nonce: String,
    pub data: OpData,
}

/// One operation, git-bug's flattened wire form.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpFlat {
    pub author: String,
    pub lamport: u64,
    pub timestamp: i64,
    pub nonce: String,
    #[serde(flatten)]
    pub data: OpData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackNested {
    pub author: String,
    pub ops: Vec<OpNested>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackFlat {
    pub author: String,
    pub ops: Vec<OpFlat>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WireFormat {
    Nested,
    Flat,
}

impl WireFormat {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "nested" => Some(Self::Nested),
            "flat" => Some(Self::Flat),
            _ => None,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::Nested => "nested",
            Self::Flat => "flat",
        }
    }
}

/// A pack as it arrives from storage: its id comes from outside the bytes.
pub struct StoredPack {
    pub id: String,
    pub bytes: Vec<u8>,
}

/// A pack after parsing, normalised across the two wire formats so replay is
/// measured on identical work either way.
pub struct LoadedPack {
    pub id: String,
    pub ops: Vec<OpNested>,
}

pub fn parse_pack(p: &StoredPack, fmt: WireFormat) -> LoadedPack {
    let ops = match fmt {
        WireFormat::Nested => {
            let pack: PackNested = serde_json::from_slice(&p.bytes).expect("pack json");
            pack.ops
        }
        WireFormat::Flat => {
            let pack: PackFlat = serde_json::from_slice(&p.bytes).expect("pack json");
            pack.ops
                .into_iter()
                .map(|o| OpNested {
                    author: o.author,
                    lamport: o.lamport,
                    timestamp: o.timestamp,
                    nonce: o.nonce,
                    data: o.data,
                })
                .collect()
        }
    };
    LoadedPack {
        id: p.id.clone(),
        ops,
    }
}

/// The replayed state of a jjj Problem. Same seven authored fields the plan
/// names; derived fields (`solution_ids`, `child_ids`) are excluded because they
/// are not written by operations today either.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProblemState {
    pub title: String,
    pub description: String,
    pub status: String,
    pub priority: String,
    pub tags: Vec<String>,
    pub parent_id: Option<String>,
    pub assignee: Option<String>,
    /// Highest Lamport clock seen, i.e. what a guarded `edit --if-lamport` reads.
    pub lamport: u64,
}

impl ProblemState {
    fn apply(&mut self, op: &OpNested) {
        if op.lamport > self.lamport {
            self.lamport = op.lamport;
        }
        match &op.data {
            OpData::SetTitle { title } => self.title.clone_from(title),
            OpData::SetDescription { description } => self.description.clone_from(description),
            OpData::SetStatus { status } => self.status.clone_from(status),
            OpData::SetPriority { priority } => self.priority.clone_from(priority),
            OpData::AddTag { tag } => {
                if !self.tags.iter().any(|t| t == tag) {
                    self.tags.push(tag.clone());
                }
            }
            OpData::RemoveTag { tag } => self.tags.retain(|t| t != tag),
            OpData::SetAssignee { assignee } => self.assignee.clone_from(assignee),
            OpData::SetParent { parent_id } => self.parent_id.clone_from(parent_id),
        }
    }
}

/// Deterministic replay: order by Lamport clock, tie-break concurrent operations
/// by lexicographic pack id, then by position within the pack, and apply.
///
/// The index pass exists because sorting the operations themselves would move
/// multi-kilobyte bodies around; sorting `(clock, pack, position)` triples keeps
/// the sort honest about what an implementation would actually do.
pub fn replay(packs: &[LoadedPack]) -> ProblemState {
    let mut index: Vec<(u64, &str, u32, u32)> = Vec::with_capacity(packs.len() * 4);
    for (pi, pack) in packs.iter().enumerate() {
        for (oi, op) in pack.ops.iter().enumerate() {
            index.push((op.lamport, pack.id.as_str(), pi as u32, oi as u32));
        }
    }
    index.sort_unstable_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(b.1)).then(a.3.cmp(&b.3)));

    let mut state = ProblemState::default();
    for (_, _, pi, oi) in &index {
        state.apply(&packs[*pi as usize].ops[*oi as usize]);
    }
    state
}

// ---------------------------------------------------------------------------
// The state-based baseline: what jjj reads today.
// ---------------------------------------------------------------------------

/// The frontmatter subset that corresponds to the replayed fields. Parsed with
/// `serde_norway`, the same parser `src/storage/` uses, so the comparison is
/// against jjj's real read path.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Frontmatter {
    pub id: String,
    pub title: String,
    pub status: String,
    pub priority: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assignee: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub lamport: u64,
    #[serde(flatten)]
    pub extra: std::collections::BTreeMap<String, serde_norway::Value>,
}

/// Split `---`-delimited frontmatter from the markdown body and parse both,
/// mirroring what `MetadataStore::load_problem` does.
pub fn read_state_file(bytes: &[u8]) -> ProblemState {
    let text = std::str::from_utf8(bytes).expect("utf8");
    let rest = text.strip_prefix("---\n").expect("frontmatter open");
    let end = rest.find("\n---\n").expect("frontmatter close");
    let fm: Frontmatter = serde_norway::from_str(&rest[..end]).expect("frontmatter yaml");
    let body = rest[end + 5..].trim_start_matches('\n');
    ProblemState {
        title: fm.title,
        description: body.to_string(),
        status: fm.status,
        priority: fm.priority,
        tags: fm.tags,
        parent_id: fm.parent_id,
        assignee: fm.assignee,
        lamport: fm.lamport,
    }
}
