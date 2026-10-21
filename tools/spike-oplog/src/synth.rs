//! Corpus synthesis. Deterministic: same seed, same corpus, so two runs are
//! comparable and the op mix stays constant across operation counts.

use crate::model::{OpData, OpFlat, OpNested, PackFlat, PackNested, StoredPack, WireFormat};

/// xorshift64*. A PRNG rather than `rand` because the spike should not pull a
/// dependency to pick integers, and determinism matters more than quality here.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed | 1)
    }
    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
    fn unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }
    fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.below(xs.len() as u64) as usize]
    }
    fn hex(&mut self, chars: usize) -> String {
        let mut s = String::with_capacity(chars);
        while s.len() < chars {
            s.push_str(&format!("{:016x}", self.next_u64()));
        }
        s.truncate(chars);
        s
    }
}

/// FNV-1a over 128 bits, rendered as 32 hex characters. Stands in for the git
/// object hash that git-bug uses as the pack id; what replay needs from it is
/// that it is opaque, fixed-width and lexicographically comparable.
fn pack_id(bytes: &[u8]) -> String {
    let mut h: u128 = 0x6c62_272e_07bb_0142_62b8_2175_6295_c58d;
    for b in bytes {
        h ^= *b as u128;
        h = h.wrapping_mul(0x0000_0000_0100_0000_0000_0000_0000_013b);
    }
    format!("{h:032x}")
}

const WORDS: &[&str] = &[
    "the",
    "replay",
    "cost",
    "grows",
    "with",
    "operation",
    "count",
    "not",
    "entity",
    "and",
    "nothing",
    "in",
    "this",
    "design",
    "bounds",
    "that",
    "number",
    "a",
    "problem",
    "edited",
    "five",
    "hundred",
    "times",
    "has",
    "operations",
    "before",
    "its",
    "current",
    "state",
    "is",
    "known",
    "which",
    "means",
    "prime",
    "pays",
    "for",
    "every",
    "edit",
    "ever",
    "made",
    "conflicted",
    "value",
    "should",
    "become",
    "an",
    "item",
    "on",
    "ready",
    "front",
    "instead",
    "of",
    "pick",
    "side",
    "silently",
    "choosing",
    "concurrent",
    "edits",
    "materialise",
    "as",
    "first",
    "class",
    "visible",
    "resolvable",
    "later",
    "schedulable",
    "work",
    "escalatable",
    "when",
    "no",
    "agent",
    "can",
    "adjudicate",
    "it",
    "lamport",
    "clocks",
    "order",
    "causally",
];

/// Body text of roughly `target` bytes, in whole words with paragraph breaks —
/// close enough in shape and size to what an agent writes into a description
/// that the JSON string escaping and allocation costs are representative.
fn body(rng: &mut Rng, target: usize) -> String {
    let mut s = String::with_capacity(target + 16);
    let mut since_break = 0usize;
    while s.len() < target {
        s.push_str(rng.pick(WORDS));
        s.push(' ');
        since_break += 1;
        if since_break > 40 {
            s.push_str("\n\n");
            since_break = 0;
        }
    }
    s
}

pub struct MixCounts {
    pub description: usize,
    pub title: usize,
    pub status: usize,
    pub priority: usize,
    pub add_tag: usize,
    pub remove_tag: usize,
    pub assignee: usize,
    pub parent: usize,
}

/// The operation mix, per 100 operations. Held constant across every operation
/// count and every body size so the curve measures one variable.
///
/// "Mostly body edits and status/field changes, with occasional tag churn":
/// 60 body, 12 title, 10 status, 5 priority, 6 tag churn, 4 assignee, 3 parent.
pub const MIX: MixCounts = MixCounts {
    description: 60,
    title: 12,
    status: 10,
    priority: 5,
    add_tag: 4,
    remove_tag: 2,
    assignee: 4,
    parent: 3,
};

/// Mean operations per pack: one editing session usually makes a few changes.
pub const OPS_PER_PACK: &[usize] = &[1, 1, 2, 2, 3, 3, 4, 5, 6];

/// Share of packs that are *concurrent* with their predecessor — same Lamport
/// clock, so replay has to fall through to the pack-id tie-break. Keeps the
/// comparison path warm rather than letting the clock alone decide every time.
pub const CONCURRENT_SHARE: f64 = 0.10;

const STATUSES: &[&str] = &["open", "in_progress", "solved", "dissolved"];
const PRIORITIES: &[&str] = &["low", "medium", "high", "critical"];
const TAGS: &[&str] = &[
    "perf",
    "substrate",
    "measure",
    "area:storage",
    "area:cli",
    "size:L",
    "size:S",
    "blocked",
    "swarm",
    "oplog",
];

fn one_op(rng: &mut Rng, body_bytes: usize) -> OpData {
    let total = MIX.description
        + MIX.title
        + MIX.status
        + MIX.priority
        + MIX.add_tag
        + MIX.remove_tag
        + MIX.assignee
        + MIX.parent;
    let mut roll = rng.below(total as u64) as usize;
    let mut take = |n: usize| {
        if roll < n {
            true
        } else {
            roll -= n;
            false
        }
    };
    if take(MIX.description) {
        // Size varies around the stated mean; a real corpus is not uniform.
        let scale = 0.4 + rng.unit() * 1.4;
        return OpData::SetDescription {
            description: body(rng, (body_bytes as f64 * scale) as usize),
        };
    }
    if take(MIX.title) {
        return OpData::SetTitle {
            title: format!(
                "{} {} {}",
                rng.pick(WORDS),
                rng.pick(WORDS),
                rng.pick(WORDS)
            ),
        };
    }
    if take(MIX.status) {
        return OpData::SetStatus {
            status: (*rng.pick(STATUSES)).to_string(),
        };
    }
    if take(MIX.priority) {
        return OpData::SetPriority {
            priority: (*rng.pick(PRIORITIES)).to_string(),
        };
    }
    if take(MIX.add_tag) {
        return OpData::AddTag {
            tag: (*rng.pick(TAGS)).to_string(),
        };
    }
    if take(MIX.remove_tag) {
        return OpData::RemoveTag {
            tag: (*rng.pick(TAGS)).to_string(),
        };
    }
    if take(MIX.assignee) {
        return OpData::SetAssignee {
            assignee: Some(format!("agent-{}", rng.below(12))),
        };
    }
    OpData::SetParent {
        parent_id: if rng.unit() < 0.7 {
            Some(rng.hex(32))
        } else {
            None
        },
    }
}

/// Synthesise `n_ops` operations for one entity, grouped into packs and
/// serialised exactly as they would be read back from storage.
pub fn synth_entity(
    seed: u64,
    n_ops: usize,
    body_bytes: usize,
    fmt: WireFormat,
) -> Vec<StoredPack> {
    let mut rng = Rng::new(seed);
    // One human plus five agents, each an identity *reference* like git-bug's.
    let principals: Vec<String> = (0..6).map(|_| rng.hex(40)).collect();

    let mut packs = Vec::new();
    let mut written = 0usize;
    let mut lamport = 1u64;
    while written < n_ops {
        let want = (*rng.pick(OPS_PER_PACK)).min(n_ops - written);
        let author = rng.pick(&principals).clone();
        let concurrent = !packs.is_empty() && rng.unit() < CONCURRENT_SHARE;
        if !concurrent {
            lamport += 1;
        }
        let ts = 1_760_000_000i64 + lamport as i64 * 37;

        let mut nested = Vec::with_capacity(want);
        for _ in 0..want {
            nested.push(OpNested {
                author: author.clone(),
                lamport,
                timestamp: ts,
                // git-bug's nonce is 20 random bytes; 40 hex characters of one.
                nonce: rng.hex(40),
                data: one_op(&mut rng, body_bytes),
            });
        }
        written += want;

        let bytes = match fmt {
            WireFormat::Nested => serde_json::to_vec(&PackNested {
                author: author.clone(),
                ops: nested,
            })
            .expect("serialise pack"),
            WireFormat::Flat => serde_json::to_vec(&PackFlat {
                author: author.clone(),
                ops: nested
                    .into_iter()
                    .map(|o| OpFlat {
                        author: o.author,
                        lamport: o.lamport,
                        timestamp: o.timestamp,
                        nonce: o.nonce,
                        data: o.data,
                    })
                    .collect(),
            })
            .expect("serialise pack"),
        };
        packs.push(StoredPack {
            id: pack_id(&bytes),
            bytes,
        });
    }
    packs
}

/// The equivalent state-based file: YAML frontmatter plus the body, as
/// `src/storage/problems.rs` writes it today. Built from the replayed state so
/// the two read paths are compared on identical content.
pub fn state_file(state: &crate::model::ProblemState, id: &str) -> Vec<u8> {
    let mut fm = String::from("---\n");
    fm.push_str(&format!("id: {id}\n"));
    fm.push_str(&format!(
        "title: {}\n",
        serde_json::to_string(&state.title).unwrap()
    ));
    fm.push_str(&format!("status: {}\n", state.status));
    fm.push_str(&format!("priority: {}\n", state.priority));
    if !state.tags.is_empty() {
        fm.push_str("tags:\n");
        for t in &state.tags {
            fm.push_str(&format!("- {t}\n"));
        }
    }
    if let Some(p) = &state.parent_id {
        fm.push_str(&format!("parent_id: {p}\n"));
    }
    if let Some(a) = &state.assignee {
        fm.push_str(&format!("assignee: {a}\n"));
    }
    fm.push_str("created_at: 2026-10-19T00:00:00Z\n");
    fm.push_str("updated_at: 2026-10-19T00:00:00Z\n");
    fm.push_str(&format!("lamport: {}\n", state.lamport));
    fm.push_str("---\n\n");
    fm.push_str(&state.description);
    fm.into_bytes()
}
