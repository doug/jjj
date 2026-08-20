//! Claims as leases (design decision 15).
//!
//! `jjj next --claim` is an *advisory* signal, never a lock: two agents reading
//! before either writes will both claim the same item, and that is by design —
//! the substrate allows free overlap because competing conjectures are a
//! feature. What the substrate must not do is let work be lost.
//!
//! Without expiry a claim is permanent, so an agent that dies mid-task holds its
//! item forever. Over a long swarm run that is not a hypothetical failure; it is
//! the expected one, and the corpus slowly fills with work nobody will ever
//! finish. A claim is therefore a **lease**: it is honoured while fresh, and the
//! item returns to the pool once it goes stale.
//!
//! The window has to exceed the interval between an agent's syncs, or a working
//! agent's own claim expires underneath it. Claims refresh at the agent's
//! existing sync boundaries rather than through a separate heartbeat, so the
//! default is generous.
//!
//! An assignment with no `claimed_at` is a deliberate hand-off — a human giving
//! work to someone — and never expires.

pub use chrono::Duration;
use chrono::{DateTime, Utc};

/// Default lease length. Long enough that an agent doing real work between syncs
/// keeps its claim, short enough that a dead agent's work returns the same day.
pub const DEFAULT_CLAIM_TTL_MINUTES: i64 = 60;

/// Config key for overriding the lease length in `config.toml`'s `[settings]`.
pub const CLAIM_TTL_SETTING: &str = "claim_ttl_minutes";

/// How long a claim stays honoured, from `[settings]` or the default.
pub fn claim_ttl(settings: &std::collections::HashMap<String, String>) -> Duration {
    let minutes = settings
        .get(CLAIM_TTL_SETTING)
        .and_then(|v| v.parse::<i64>().ok())
        .filter(|m| *m > 0)
        .unwrap_or(DEFAULT_CLAIM_TTL_MINUTES);
    Duration::minutes(minutes)
}

/// What an actor may do with an item, given who holds it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClaimState {
    /// Nobody holds it.
    Free,
    /// This actor holds it.
    Held,
    /// Someone else holds it and the lease is still good — leave it alone.
    HeldByOther,
    /// Someone else held it but the lease lapsed; anyone may reclaim.
    Expired,
}

impl ClaimState {
    /// Whether an actor looking for work should be offered this item.
    ///
    /// Work held by a live claimant is excluded: offering it anyway is what made
    /// every agent in the first swarm trial pile onto the same problem.
    pub fn available(self) -> bool {
        matches!(
            self,
            ClaimState::Free | ClaimState::Held | ClaimState::Expired
        )
    }
}

/// Classify an item for `actor`.
///
/// `now` is passed in rather than read, so the expiry boundary is testable
/// without waiting an hour.
pub fn classify(
    assignee: Option<&str>,
    claimed_at: Option<DateTime<Utc>>,
    actor: &str,
    ttl: Duration,
    now: DateTime<Utc>,
) -> ClaimState {
    let Some(assignee) = assignee.filter(|a| !a.trim().is_empty()) else {
        return ClaimState::Free;
    };

    if crate::identity::actor_matches(assignee, actor) {
        return ClaimState::Held;
    }

    match claimed_at {
        // A deliberate hand-off, not a claim. It is someone's job until they say
        // otherwise; expiring it would quietly reassign human decisions.
        None => ClaimState::HeldByOther,
        Some(at) if now.signed_duration_since(at) >= ttl => ClaimState::Expired,
        Some(_) => ClaimState::HeldByOther,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ttl() -> Duration {
        Duration::minutes(60)
    }

    #[test]
    fn unassigned_work_is_free() {
        let now = Utc::now();
        assert_eq!(classify(None, None, "a", ttl(), now), ClaimState::Free);
        assert_eq!(
            classify(Some("  "), None, "a", ttl(), now),
            ClaimState::Free
        );
    }

    #[test]
    fn your_own_claim_is_yours_however_old() {
        let now = Utc::now();
        let ancient = now - Duration::days(30);
        assert_eq!(
            classify(Some("agent-a"), Some(ancient), "agent-a", ttl(), now),
            ClaimState::Held,
            "an agent must not lose its own claim by holding it a long time"
        );
    }

    #[test]
    fn a_fresh_claim_by_another_agent_is_respected() {
        let now = Utc::now();
        let recent = now - Duration::minutes(5);
        assert_eq!(
            classify(Some("agent-b"), Some(recent), "agent-a", ttl(), now),
            ClaimState::HeldByOther
        );
    }

    #[test]
    fn a_stale_claim_returns_to_the_pool() {
        let now = Utc::now();
        let stale = now - Duration::minutes(61);
        assert_eq!(
            classify(Some("agent-b"), Some(stale), "agent-a", ttl(), now),
            ClaimState::Expired,
            "a dead agent must not hold work forever"
        );
    }

    #[test]
    fn expiry_is_inclusive_at_the_boundary() {
        let now = Utc::now();
        let exactly = now - Duration::minutes(60);
        assert_eq!(
            classify(Some("agent-b"), Some(exactly), "agent-a", ttl(), now),
            ClaimState::Expired
        );
    }

    #[test]
    fn a_deliberate_assignment_never_expires() {
        let now = Utc::now();
        // No claimed_at: a human handed this to someone. Expiring it would
        // silently undo a decision a person made.
        assert_eq!(
            classify(Some("alice"), None, "agent-a", ttl(), now),
            ClaimState::HeldByOther
        );
    }

    #[test]
    fn only_live_claims_by_others_are_withheld() {
        assert!(ClaimState::Free.available());
        assert!(ClaimState::Held.available());
        assert!(ClaimState::Expired.available());
        assert!(!ClaimState::HeldByOther.available());
    }

    #[test]
    fn ttl_comes_from_settings_when_valid() {
        let mut settings = std::collections::HashMap::new();
        assert_eq!(claim_ttl(&settings), Duration::minutes(60));

        settings.insert(CLAIM_TTL_SETTING.to_string(), "15".to_string());
        assert_eq!(claim_ttl(&settings), Duration::minutes(15));

        // Nonsense must not disable expiry — that would restore the permanent
        // claim this module exists to prevent.
        settings.insert(CLAIM_TTL_SETTING.to_string(), "0".to_string());
        assert_eq!(claim_ttl(&settings), Duration::minutes(60));
        settings.insert(CLAIM_TTL_SETTING.to_string(), "banana".to_string());
        assert_eq!(claim_ttl(&settings), Duration::minutes(60));
    }
}
