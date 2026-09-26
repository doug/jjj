//! The repository's Lamport clock.
//!
//! A Lamport logical clock gives edits a causal order that does not depend on
//! any machine's wall clock. Two rules define it:
//!
//! - **Increment on write.** A new version of an entity gets a clock strictly
//!   greater than the version it was derived from, and greater than anything
//!   this repository has seen.
//! - **Witness on read.** Observing a clock value advances the local clock to at
//!   least that value, so anything written *after* reading is ordered *after*
//!   what was read.
//!
//! Together those make "later" mean "causally later" rather than "stamped with a
//! bigger number by a machine whose clock is wrong".
//!
//! The counter is **machine-local** — it lives beside `.sync_state.json` and
//! `.events_offsets.json` and is never synced. It has to be: it is this
//! repository's view of how much of the world it has seen, and merging two
//! clones' counters would be meaningless.

use std::path::Path;

/// Filename under `.jj/jjj-meta/`, deliberately dot-prefixed to match the other
/// machine-local state and stay out of the synced set.
const CLOCK_FILE: &str = ".lamport";

/// How far a single observation may advance the local clock.
///
/// Without a bound, one entity claiming a clock of `u64::MAX` would pin the
/// local counter there and every subsequent local write would saturate, making
/// the order meaningless for good. A million is far above anything legitimate,
/// since a real repository advances the clock by one per write — so the cap
/// rejects only values that could not have been reached honestly.
const MAX_WITNESS_JUMP: u64 = 1_000_000;

fn clock_path(meta_path: &Path) -> std::path::PathBuf {
    meta_path.join(CLOCK_FILE)
}

/// Read the local clock, or 0 if it has never been written.
///
/// Unreadable or malformed content reads as 0 rather than failing: a lost
/// counter costs ordering precision until it catches up, where a hard error
/// would make the repository unusable.
pub fn read(meta_path: &Path) -> u64 {
    std::fs::read_to_string(clock_path(meta_path))
        .ok()
        .and_then(|s| s.trim().parse::<u64>().ok())
        .unwrap_or(0)
}

/// Write the local clock, best-effort.
fn write(meta_path: &Path, value: u64) {
    let _ = std::fs::write(clock_path(meta_path), value.to_string());
}

/// The clock to stamp on a new version of an entity whose current value is
/// `entity_clock`, advancing the local counter to match.
///
/// Strictly greater than both the local counter and the entity's own value, so
/// a new version always sorts after the version it was derived from even if this
/// repository has never seen a higher clock.
pub fn tick(meta_path: &Path, entity_clock: u64) -> u64 {
    let next = read(meta_path).max(entity_clock).saturating_add(1);
    write(meta_path, next);
    next
}

/// Advance the local clock to at least `observed`.
///
/// This is the "witness" half of the algorithm and the reason causality holds:
/// an edit made after reading someone else's edit is stamped higher than it.
///
/// Refuses to jump more than [`MAX_WITNESS_JUMP`] past the current value. An
/// absurd clock — whether hostile or a corrupted file — would otherwise poison
/// the counter permanently.
pub fn witness(meta_path: &Path, observed: u64) {
    if observed == 0 {
        return;
    }
    let current = read(meta_path);
    if observed <= current {
        return;
    }
    if observed - current > MAX_WITNESS_JUMP {
        crate::output::warn(&format!(
            "ignoring an implausible Lamport clock ({observed}); local clock is {current}. \
             An entity claiming a clock this far ahead would pin the counter and make \
             edit ordering meaningless."
        ));
        return;
    }
    write(meta_path, observed);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_at_zero_and_ticks_upward() {
        let tmp = tempfile::tempdir().unwrap();
        assert_eq!(read(tmp.path()), 0);
        assert_eq!(tick(tmp.path(), 0), 1);
        assert_eq!(tick(tmp.path(), 0), 2);
        assert_eq!(read(tmp.path()), 2);
    }

    #[test]
    fn a_tick_exceeds_the_entitys_own_clock() {
        // The entity came from a clone with a much higher clock. The new version
        // must still sort after it, or an edit would appear to precede what it
        // was derived from.
        let tmp = tempfile::tempdir().unwrap();
        assert_eq!(tick(tmp.path(), 500), 501);
        assert_eq!(read(tmp.path()), 501);
    }

    #[test]
    fn witnessing_advances_but_never_retreats() {
        let tmp = tempfile::tempdir().unwrap();
        witness(tmp.path(), 42);
        assert_eq!(read(tmp.path()), 42);
        witness(tmp.path(), 7);
        assert_eq!(read(tmp.path()), 42, "a lower observation must not rewind");
        witness(tmp.path(), 0);
        assert_eq!(read(tmp.path()), 42, "absent clocks are not observations");
    }

    #[test]
    fn an_implausible_clock_is_ignored() {
        // Otherwise one entity claiming u64::MAX pins the counter and every
        // later local write saturates at the same value, destroying the order
        // permanently.
        let tmp = tempfile::tempdir().unwrap();
        witness(tmp.path(), 10);
        witness(tmp.path(), u64::MAX);
        assert_eq!(read(tmp.path()), 10, "the spam guard did not hold");
        // And a legitimate large-but-plausible jump still lands.
        witness(tmp.path(), 10 + MAX_WITNESS_JUMP);
        assert_eq!(read(tmp.path()), 10 + MAX_WITNESS_JUMP);
    }

    #[test]
    fn a_corrupt_counter_reads_as_zero_rather_than_failing() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join(CLOCK_FILE), "not a number").unwrap();
        assert_eq!(read(tmp.path()), 0);
        assert_eq!(tick(tmp.path(), 0), 1, "the repository must stay usable");
    }
}
